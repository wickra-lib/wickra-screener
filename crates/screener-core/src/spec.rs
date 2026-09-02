//! The scan specification: comparators, the condition tree, ranking and the
//! top-level [`ScanSpec`].

use crate::breadth::{BreadthSpec, NEEDS_BUY_SIGNAL};
use crate::error::{Error, Result};
use crate::expr::Expr;
use crate::feeds::{Available, FeedKind};
use crate::indicator_set::feed_kind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
    /// Candle timeframe label (e.g. `"1h"`), echoed into the report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
    /// A benchmark symbol from the universe whose close feeds the pairwise
    /// indicators of every other symbol, read at the same bar.
    ///
    /// This is the convenient form of the reference feed: a screen that ranks a
    /// universe against one of its members ("beta to BTC") names it once instead
    /// of repeating that member's whole series under every symbol. A per-symbol
    /// `reference` series in the feeds still wins where both are given, which is
    /// how a benchmark outside the universe is expressed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// How the cross-section the screener assembles for itself is parameterised.
    /// Absent means the defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breadth: Option<BreadthSpec>,
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
    /// positive, a named reference symbol is in the universe, the breadth
    /// configuration is sane, and no breadth condition is nested inside another
    /// breadth. (Indicator existence is enforced when the indicator set is
    /// built, and feed availability when a scan knows what it can supply.)
    pub fn validate(&self) -> Result<()> {
        if self.universe.is_empty() {
            return Err(Error::BadSpec("universe is empty".into()));
        }
        if self.limit == Some(0) {
            return Err(Error::BadSpec("limit must be greater than 0".into()));
        }
        if let Some(reference) = &self.reference {
            if !self.universe.contains(reference) {
                return Err(Error::BadSpec(format!(
                    "reference symbol {reference} is not in the universe"
                )));
            }
        }
        if let Some(breadth) = &self.breadth {
            breadth.validate()?;
        }
        check_breadth_nesting(&self.condition)
    }

    /// Whether the spec names an indicator that reads the market cross-section.
    #[must_use]
    pub fn needs_cross_section(&self) -> bool {
        self.required_feeds().contains(&FeedKind::CrossSection)
    }

    /// The universe as a set, for membership tests during a scan.
    ///
    /// The field is a `Vec` because a spec is a document and order is what a
    /// reader wrote; the set is what a scan asks questions of.
    #[must_use]
    pub fn universe_set(&self) -> BTreeSet<&str> {
        self.universe.iter().map(String::as_str).collect()
    }

    /// The universe symbols that `present` does not cover, in universe order and
    /// without repeats.
    #[must_use]
    pub fn missing_from<'a>(&self, present: impl Iterator<Item = &'a str>) -> Vec<String> {
        let present: BTreeSet<&str> = present.collect();
        let mut seen = BTreeSet::new();
        self.universe
            .iter()
            .filter(|s| !present.contains(s.as_str()))
            .filter(|s| seen.insert(s.as_str()))
            .cloned()
            .collect()
    }

    /// Visit every expression the spec references — the whole condition tree plus
    /// the ranking expression — short-circuiting on the first error.
    pub(crate) fn visit_exprs<F>(&self, visit: &mut F) -> Result<()>
    where
        F: FnMut(&Expr) -> Result<()>,
    {
        visit_exprs(&self.condition, visit)?;
        match &self.rank {
            Some(rank) => visit(&rank.by),
            None => Ok(()),
        }
    }

    /// Reject the spec if it names an indicator whose feed the scan cannot
    /// supply.
    ///
    /// An indicator without its feed is not an error inside `wickra-core`: it
    /// ticks and returns `None`, on this bar and every later one. A screen built
    /// on it would run to completion and match nothing, which reads exactly like
    /// a screen whose condition was simply never true. Refusing the spec is what
    /// separates the two.
    pub(crate) fn check_feeds(&self, available: Available) -> Result<()> {
        self.visit_exprs(&mut |expr| {
            let Expr::Indicator { name, .. } = expr else {
                return Ok(());
            };
            let kind = feed_kind(name).ok_or_else(|| Error::UnknownIndicator(name.clone()))?;
            if kind == FeedKind::CrossSection
                && available.sections_are_derived
                && name == NEEDS_BUY_SIGNAL
            {
                // The derived panel carries every member signal that can be read
                // off a candle. This one cannot, so answering with a panel where
                // it is false for every symbol would report a confident zero.
                return Err(Error::UnderivableSignal {
                    indicator: name.clone(),
                    signal: "point-and-figure buy".to_string(),
                });
            }
            if available.has(kind) {
                return Ok(());
            }
            Err(Error::MissingFeed {
                indicator: name.clone(),
                feed: kind.as_str().to_string(),
            })
        })
    }

    /// The feed families this spec's indicators need beyond the candle.
    #[must_use]
    pub fn required_feeds(&self) -> Vec<FeedKind> {
        let mut kinds: Vec<FeedKind> = Vec::new();
        let _ = self.visit_exprs(&mut |expr| {
            if let Expr::Indicator { name, .. } = expr {
                if let Some(kind) = feed_kind(name) {
                    if kind != FeedKind::Candle && !kinds.contains(&kind) {
                        kinds.push(kind);
                    }
                }
            }
            Ok(())
        });
        kinds
    }
}

/// Visit every expression in a condition tree, short-circuiting on error.
pub(crate) fn visit_exprs<F>(cond: &Condition, visit: &mut F) -> Result<()>
where
    F: FnMut(&Expr) -> Result<()>,
{
    match cond {
        Condition::Cmp { left, right, .. } => {
            visit(left)?;
            visit(right)
        }
        Condition::CrossSection { expr, .. } => visit(expr),
        Condition::Breadth { inner, .. } => visit_exprs(inner, visit),
        Condition::All { conditions } | Condition::Any { conditions } => {
            for c in conditions {
                visit_exprs(c, visit)?;
            }
            Ok(())
        }
        Condition::Not { condition } => visit_exprs(condition, visit),
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
            reference: None,
            breadth: None,
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
            reference: None,
            breadth: None,
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
            reference: None,
            breadth: None,
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
            reference: None,
            breadth: None,
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
            reference: None,
            breadth: None,
            condition: breadth,
            rank: None,
            limit: None,
        };
        assert!(spec.validate().is_ok());
    }
}
