//! The scan specification: comparators, the condition tree, ranking and the
//! top-level [`ScanSpec`].

use crate::error::{Error, Result};
use crate::expr::Expr;
use serde::{Deserialize, Serialize};

/// How two scalar values are compared.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Comparator {
    /// `left > right`.
    Gt,
    /// `left >= right`.
    Ge,
    /// `left < right`.
    Lt,
    /// `left <= right`.
    Le,
    /// `left ~= right` (relative tolerance).
    Eq,
    /// `left` crosses above `right` this bar.
    CrossesAbove,
    /// `left` crosses below `right` this bar.
    CrossesBelow,
}

/// A cross-section reduction over the ready universe.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CsMetric {
    /// 1-based rank of the value (highest = rank 1).
    Rank,
    /// Fraction of symbols with a strictly smaller value, in `[0, 1]`.
    PercentileRank,
    /// Population z-score of the value across the universe.
    ZScore,
}

/// A boolean condition over a symbol, evaluated at the latest bar.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Compare two expressions.
    Cmp {
        /// Left-hand expression.
        left: Expr,
        /// Comparison operator.
        op: Comparator,
        /// Right-hand expression.
        right: Expr,
    },
    /// Compare a symbol's cross-section metric against a constant.
    CrossSection {
        /// Expression reduced across the universe.
        expr: Expr,
        /// Which cross-section metric.
        metric: CsMetric,
        /// Comparison operator.
        op: Comparator,
        /// Threshold value.
        value: f64,
    },
    /// A universe-wide market gate: the fraction of ready symbols for which
    /// `inner` holds, compared against `ratio`. Passes for every symbol or none.
    Breadth {
        /// Inner condition (must not itself contain a breadth).
        inner: Box<Condition>,
        /// Comparison operator.
        op: Comparator,
        /// Threshold ratio in `[0, 1]`.
        ratio: f64,
    },
    /// All sub-conditions must hold.
    All {
        /// Sub-conditions.
        conditions: Vec<Condition>,
    },
    /// Any sub-condition must hold.
    Any {
        /// Sub-conditions.
        conditions: Vec<Condition>,
    },
    /// The negation of a condition.
    Not {
        /// The negated condition.
        condition: Box<Condition>,
    },
}

/// How matches are ranked and scored.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RankSpec {
    /// Expression scored for ranking.
    pub by: Expr,
    /// Rank by descending score when true, ascending when false.
    #[serde(default)]
    pub desc: bool,
}

/// A complete scan specification.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScanSpec {
    /// The symbols scanned.
    pub universe: Vec<String>,
    /// Informational candle timeframe (e.g. `"1h"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
    /// The condition tree evaluated at the latest bar.
    pub condition: Condition,
    /// Optional ranking of the matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<RankSpec>,
    /// Optional cap on the number of matches returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl ScanSpec {
    /// Parse a spec from JSON and validate it.
    pub fn from_json(s: &str) -> Result<Self> {
        let spec: ScanSpec = serde_json::from_str(s)?;
        spec.validate()?;
        Ok(spec)
    }

    /// Parse a spec from TOML and validate it.
    pub fn from_toml(s: &str) -> Result<Self> {
        let spec: ScanSpec = toml::from_str(s)?;
        spec.validate()?;
        Ok(spec)
    }

    /// Structural validation: the universe is non-empty, any `limit` is
    /// positive, and no breadth condition is nested inside another breadth.
    /// (Indicator existence is enforced when the indicator set is built.)
    pub(crate) fn validate(&self) -> Result<()> {
        if self.universe.is_empty() {
            return Err(Error::BadSpec("universe is empty".into()));
        }
        if self.limit == Some(0) {
            return Err(Error::BadSpec("limit must be greater than 0".into()));
        }
        check_breadth_nesting(&self.condition)
    }
}

/// Reject a breadth condition whose inner subtree contains another breadth.
fn check_breadth_nesting(cond: &Condition) -> Result<()> {
    match cond {
        Condition::Breadth { inner, .. } => {
            if contains_breadth(inner) {
                return Err(Error::BadSpec(
                    "breadth condition nested inside another breadth".into(),
                ));
            }
            Ok(())
        }
        Condition::All { conditions } | Condition::Any { conditions } => {
            for c in conditions {
                check_breadth_nesting(c)?;
            }
            Ok(())
        }
        Condition::Not { condition } => check_breadth_nesting(condition),
        Condition::Cmp { .. } | Condition::CrossSection { .. } => Ok(()),
    }
}

/// Whether a condition subtree contains any breadth condition.
fn contains_breadth(cond: &Condition) -> bool {
    match cond {
        Condition::Breadth { .. } => true,
        Condition::All { conditions } | Condition::Any { conditions } => {
            conditions.iter().any(contains_breadth)
        }
        Condition::Not { condition } => contains_breadth(condition),
        Condition::Cmp { .. } | Condition::CrossSection { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::PriceField;

    fn cmp() -> Condition {
        Condition::Cmp {
            left: Expr::Price {
                field: PriceField::Close,
            },
            op: Comparator::Gt,
            right: Expr::Const { value: 0.0 },
        }
    }

    #[test]
    fn valid_spec_parses() {
        let spec = ScanSpec {
            universe: vec!["AAA".into()],
            timeframe: Some("1h".into()),
            condition: cmp(),
            rank: None,
            limit: Some(5),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(ScanSpec::from_json(&json).unwrap(), spec);
    }

    #[test]
    fn empty_universe_rejected() {
        let spec = ScanSpec {
            universe: vec![],
            timeframe: None,
            condition: cmp(),
            rank: None,
            limit: None,
        };
        assert!(matches!(spec.validate(), Err(Error::BadSpec(_))));
    }

    #[test]
    fn zero_limit_rejected() {
        let spec = ScanSpec {
            universe: vec!["AAA".into()],
            timeframe: None,
            condition: cmp(),
            rank: None,
            limit: Some(0),
        };
        assert!(matches!(spec.validate(), Err(Error::BadSpec(_))));
    }

    #[test]
    fn breadth_in_breadth_rejected() {
        let nested = Condition::Breadth {
            inner: Box::new(Condition::Breadth {
                inner: Box::new(cmp()),
                op: Comparator::Ge,
                ratio: 0.5,
            }),
            op: Comparator::Ge,
            ratio: 0.5,
        };
        let spec = ScanSpec {
            universe: vec!["AAA".into()],
            timeframe: None,
            condition: nested,
            rank: None,
            limit: None,
        };
        assert!(matches!(spec.validate(), Err(Error::BadSpec(_))));
    }

    #[test]
    fn top_level_breadth_allowed() {
        let breadth = Condition::Breadth {
            inner: Box::new(cmp()),
            op: Comparator::Ge,
            ratio: 0.5,
        };
        let spec = ScanSpec {
            universe: vec!["AAA".into()],
            timeframe: None,
            condition: breadth,
            rank: None,
            limit: None,
        };
        assert!(spec.validate().is_ok());
    }
}
