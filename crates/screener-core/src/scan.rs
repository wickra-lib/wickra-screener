//! The batch scan: fold a universe, evaluate the condition per symbol, and rank
//! the matches into a deterministic [`ScanReport`].

use crate::error::Result;
use crate::eval::eval_condition;
use crate::expr::Expr;
use crate::spec::{Condition, CsMetric, ScanSpec};
use crate::symbol_state::SymbolState;
use crate::universe::Universe;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use wickra_backtest_core::Candle;

/// One symbol's scan outcome: the values that drove the match and an optional
/// ranking score.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScanResult {
    /// The symbol.
    pub symbol: String,
    /// Always true in a report (a report holds only matches); present so a
    /// streaming caller can distinguish a non-match.
    pub matched: bool,
    /// The referenced expression values (and cross-section metrics) that explain
    /// the match, keyed by canonical string and rounded to 1e-8.
    pub values: BTreeMap<String, f64>,
    /// The ranking score, if the spec ranks matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

/// The result of a scan: the matches (sorted and limited) and how many symbols
/// were scanned.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScanReport {
    /// The matching symbols, after sorting and any limit.
    pub matches: Vec<ScanResult>,
    /// The number of symbols scanned.
    pub scanned: usize,
}

/// Round a value to a fixed 1e-8 grid so every language serializes it
/// identically (§6.8).
fn round_to(x: f64) -> f64 {
    (x * 1e8).round() / 1e8
}

/// Scan a universe of candle series against a spec.
///
/// Validates the spec, folds every symbol over its full history (in parallel
/// with the `parallel` feature, sequentially otherwise — byte-identical),
/// evaluates the condition at the last bar, collects the match reasons and
/// ranking score, then sorts and limits the matches.
pub fn scan_batch(data: &BTreeMap<String, Vec<Candle>>, spec: &ScanSpec) -> Result<ScanReport> {
    spec.validate()?;
    let mut universe = Universe::new();
    universe.symbols = folded_states(data, spec)?;
    Ok(evaluate_universe(&universe, spec, data.len()))
}

/// Evaluate an already-folded universe against the spec: filter matches, collect
/// their values and rank score, then sort and limit. Shared by `scan_batch` and
/// the streaming `Screener::evaluate`; the spec is assumed already validated.
pub(crate) fn evaluate_universe(
    universe: &Universe,
    spec: &ScanSpec,
    scanned: usize,
) -> ScanReport {
    let mut matches: Vec<ScanResult> = Vec::new();
    for (symbol, state) in &universe.symbols {
        if !eval_condition(&spec.condition, symbol, universe) {
            continue;
        }
        let mut values = BTreeMap::new();
        collect_values(&spec.condition, symbol, state, universe, &mut values);
        if let Some(rank) = &spec.rank {
            add_expr_value(&rank.by, state, &mut values);
        }
        let score = spec.rank.as_ref().and_then(|rank| {
            state
                .expr_cur(&rank.by)
                .filter(|v| v.is_finite())
                .map(round_to)
        });
        matches.push(ScanResult {
            symbol: symbol.clone(),
            matched: true,
            values,
            score,
        });
    }
    sort_matches(&mut matches, spec);
    if let Some(limit) = spec.limit {
        matches.truncate(limit);
    }
    ScanReport { matches, scanned }
}

/// Build a fully-folded state per symbol, in parallel with rayon.
#[cfg(feature = "parallel")]
fn folded_states(
    data: &BTreeMap<String, Vec<Candle>>,
    spec: &ScanSpec,
) -> Result<BTreeMap<String, SymbolState>> {
    use rayon::prelude::*;
    let built: Vec<Result<(String, SymbolState)>> = data
        .par_iter()
        .map(|(symbol, candles)| Ok((symbol.clone(), fold_symbol(candles, spec)?)))
        .collect();
    let mut states = BTreeMap::new();
    for entry in built {
        let (symbol, state) = entry?;
        states.insert(symbol, state);
    }
    Ok(states)
}

/// Build a fully-folded state per symbol, sequentially (the WASM fallback).
#[cfg(not(feature = "parallel"))]
fn folded_states(
    data: &BTreeMap<String, Vec<Candle>>,
    spec: &ScanSpec,
) -> Result<BTreeMap<String, SymbolState>> {
    let mut states = BTreeMap::new();
    for (symbol, candles) in data {
        states.insert(symbol.clone(), fold_symbol(candles, spec)?);
    }
    Ok(states)
}

/// Fold one symbol's candle history into a ready state.
fn fold_symbol(candles: &[Candle], spec: &ScanSpec) -> Result<SymbolState> {
    let mut state = SymbolState::new(spec)?;
    for candle in candles {
        state.fold(candle);
    }
    Ok(state)
}

/// Sort the matches per §6.7: by ranking score (descending or ascending), ties
/// by symbol key; matches without a score go to the end. Without a rank spec,
/// sort by symbol key alone.
fn sort_matches(matches: &mut [ScanResult], spec: &ScanSpec) {
    let Some(rank) = &spec.rank else {
        matches.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        return;
    };
    matches.sort_by(|a, b| match (a.score, b.score) {
        (Some(sa), Some(sb)) => {
            let by_score = if rank.desc {
                sb.partial_cmp(&sa)
            } else {
                sa.partial_cmp(&sb)
            }
            .unwrap_or(Ordering::Equal);
            by_score.then_with(|| a.symbol.cmp(&b.symbol))
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.symbol.cmp(&b.symbol),
    });
}

/// Collect the expression values that explain a match for one symbol.
fn collect_values(
    cond: &Condition,
    symbol: &str,
    state: &SymbolState,
    universe: &Universe,
    out: &mut BTreeMap<String, f64>,
) {
    match cond {
        Condition::Cmp { left, right, .. } => {
            add_expr_value(left, state, out);
            add_expr_value(right, state, out);
        }
        Condition::CrossSection { expr, metric, .. } => {
            add_expr_value(expr, state, out);
            if let Some(v) = universe.cross_section(expr, *metric).get(symbol) {
                if v.is_finite() {
                    out.insert(
                        format!("{}#{}", expr.key(), metric_key(*metric)),
                        round_to(*v),
                    );
                }
            }
        }
        Condition::Breadth { inner, .. } => collect_values(inner, symbol, state, universe, out),
        Condition::All { conditions } | Condition::Any { conditions } => {
            for c in conditions {
                collect_values(c, symbol, state, universe, out);
            }
        }
        Condition::Not { condition } => collect_values(condition, symbol, state, universe, out),
    }
}

/// Insert an expression's current value into the map, keyed by its canonical
/// string. Constants are self-evident and omitted.
fn add_expr_value(expr: &Expr, state: &SymbolState, out: &mut BTreeMap<String, f64>) {
    if matches!(expr, Expr::Const { .. }) {
        return;
    }
    if let Some(v) = state.expr_cur(expr) {
        if v.is_finite() {
            out.insert(expr.key(), round_to(v));
        }
    }
}

/// The canonical suffix for a cross-section metric key.
fn metric_key(metric: CsMetric) -> &'static str {
    match metric {
        CsMetric::Rank => "rank",
        CsMetric::PercentileRank => "percentile_rank",
        CsMetric::ZScore => "z_score",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::PriceField;
    use crate::spec::{Comparator, RankSpec};

    fn candle(close: f64) -> Candle {
        Candle {
            time: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        }
    }

    fn close() -> Expr {
        Expr::Price {
            field: PriceField::Close,
        }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn data() -> BTreeMap<String, Vec<Candle>> {
        BTreeMap::from([
            ("A".to_string(), vec![candle(10.0)]),
            ("B".to_string(), vec![candle(20.0)]),
            ("C".to_string(), vec![candle(30.0)]),
        ])
    }

    fn gt15() -> Condition {
        Condition::Cmp {
            left: close(),
            op: Comparator::Gt,
            right: Expr::Const { value: 15.0 },
        }
    }

    #[test]
    fn ranks_descending_and_limits() {
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: gt15(),
            rank: Some(RankSpec {
                by: close(),
                desc: true,
            }),
            limit: Some(2),
        };
        let report = scan_batch(&data(), &spec).unwrap();
        assert_eq!(report.scanned, 3);
        assert_eq!(report.matches.len(), 2);
        assert_eq!(report.matches[0].symbol, "C");
        assert_eq!(report.matches[1].symbol, "B");
        assert!(approx(report.matches[0].values["price.close"], 30.0));
        assert!(report.matches[0].score.is_some_and(|s| approx(s, 30.0)));
    }

    #[test]
    fn without_rank_sorts_by_symbol_and_omits_score() {
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: gt15(),
            rank: None,
            limit: None,
        };
        let report = scan_batch(&data(), &spec).unwrap();
        assert_eq!(report.matches.len(), 2);
        assert_eq!(report.matches[0].symbol, "B");
        assert_eq!(report.matches[1].symbol, "C");
        assert!(report.matches[0].score.is_none());
    }

    #[test]
    fn cross_section_value_is_keyed_with_metric() {
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: Condition::CrossSection {
                expr: close(),
                metric: CsMetric::Rank,
                op: Comparator::Le,
                value: 1.0,
            },
            rank: None,
            limit: None,
        };
        let report = scan_batch(&data(), &spec).unwrap();
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].symbol, "C");
        assert!(approx(report.matches[0].values["price.close#rank"], 1.0));
    }

    #[test]
    fn report_round_trips_as_json() {
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: gt15(),
            rank: None,
            limit: None,
        };
        let report = scan_batch(&data(), &spec).unwrap();
        let json = serde_json::to_string(&report).unwrap();
        assert_eq!(serde_json::from_str::<ScanReport>(&json).unwrap(), report);
    }

    #[test]
    fn score_ties_break_by_symbol() {
        let data = BTreeMap::from([
            ("A".to_string(), vec![candle(20.0)]),
            ("B".to_string(), vec![candle(20.0)]),
            ("C".to_string(), vec![candle(30.0)]),
        ]);
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: gt15(),
            rank: Some(RankSpec {
                by: close(),
                desc: true,
            }),
            limit: None,
        };
        let report = scan_batch(&data, &spec).unwrap();
        assert_eq!(report.matches.len(), 3);
        assert_eq!(report.matches[0].symbol, "C"); // 30
        assert_eq!(report.matches[1].symbol, "A"); // 20, tie -> A before B
        assert_eq!(report.matches[2].symbol, "B");
    }

    #[test]
    fn scan_is_deterministic() {
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: gt15(),
            rank: None,
            limit: None,
        };
        let first = scan_batch(&data(), &spec).unwrap();
        let second = scan_batch(&data(), &spec).unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }
}
