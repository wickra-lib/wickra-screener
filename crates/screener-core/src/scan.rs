//! The batch scan: fold a universe, evaluate the condition per symbol, and rank
//! the matches into a deterministic [`ScanReport`].

use crate::breadth::{assemble, BreadthState};
use crate::error::Result;
use crate::eval::eval_condition;
use crate::expr::Expr;
use crate::feeds::{CoreSeries, SymbolInput};
use crate::spec::{Condition, CsMetric, ScanSpec};
use crate::symbol_state::SymbolState;
use crate::universe::Universe;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

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

/// The result of a scan: the matches (sorted and limited), how many symbols were
/// scanned, and which of the spec's symbols never arrived.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScanReport {
    /// The matching symbols, after sorting and any limit.
    pub matches: Vec<ScanResult>,
    /// The number of the spec's symbols that were actually folded.
    pub scanned: usize,
    /// Symbols the spec's universe names that the scan received no data for.
    ///
    /// A screen that quietly leaves out a third of its universe reports the same
    /// shape as one that saw everything, so the gap is named rather than
    /// inferred from a count. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
    /// Symbols whose most recent bar is older than the last bar in the universe.
    ///
    /// A symbol that stopped printing still carries the state of its last bar and
    /// is still screened, so without naming it a halted or delisted name reads
    /// exactly like a live one. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale: Vec<String>,
    /// The spec's timeframe label, echoed so a report says which bars it
    /// describes. Omitted when the spec declares none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
}

/// Round a value to a fixed 1e-8 grid so every language serializes it
/// identically (§6.8).
fn round_to(x: f64) -> f64 {
    (x * 1e8).round() / 1e8
}

/// Scan a universe against a spec.
///
/// Each symbol's input is either a bare candle array or a [`SymbolSeries`] with
/// the side feeds its indicators need. The spec is validated, every feed array
/// is checked to be as long as the candle array and converted once, and a spec
/// naming an indicator whose feed a symbol does not supply is rejected — an
/// unsupplied feed would otherwise make that indicator return nothing on every
/// bar and the screen match nothing at all, indistinguishably from a condition
/// that was simply never true.
///
/// Every symbol is then folded over its full history (in parallel with the
/// `parallel` feature, sequentially otherwise — byte-identical), the condition
/// is evaluated at the last bar, the match reasons and ranking score are
/// collected, and the matches are sorted and limited.
///
/// [`SymbolSeries`]: crate::SymbolSeries
pub fn scan_batch(data: BTreeMap<String, SymbolInput>, spec: &ScanSpec) -> Result<ScanReport> {
    spec.validate()?;
    let series = ingest(data, spec)?;
    let missing = spec.missing_from(series.keys().map(String::as_str));
    let scanned = series.len();

    // A batch scan holds the whole universe, so it can assemble the market
    // cross-section itself and read a benchmark close off the same bar. Neither
    // is true of a single symbol's feeds, which is why the feed check is made
    // against what this scan can supply rather than against the feeds alone.
    let lockstep = needs_lockstep(&series, spec);
    for symbol_series in series.values() {
        let available = symbol_series
            .available()
            .with_derived_sections(lockstep)
            .with_reference_symbol(spec.reference.is_some());
        spec.check_feeds(available)?;
    }

    let stale = stale_symbols(&series);
    let mut universe = Universe::new();
    universe.symbols = if lockstep {
        lockstep_states(&series, spec)?
    } else {
        folded_states(&series, spec)?
    };
    Ok(evaluate_universe(&universe, spec, scanned, missing, stale))
}

/// Whether this scan has to fold the universe in lockstep rather than symbol by
/// symbol.
///
/// Two things need it: a cross-section the screener assembles itself, which by
/// definition wants every symbol at the same timestamp, and a benchmark
/// reference symbol, whose close has to be read at that timestamp. A scan
/// needing neither keeps the per-symbol fold, which parallelises with no
/// cross-symbol ordering at all.
fn needs_lockstep(data: &BTreeMap<String, CoreSeries>, spec: &ScanSpec) -> bool {
    if spec.reference.is_some() {
        return true;
    }
    spec.needs_cross_section() && data.values().any(|series| !series.available().sections)
}

/// The symbols whose last bar is older than the last bar anywhere in the
/// universe.
fn stale_symbols(data: &BTreeMap<String, CoreSeries>) -> Vec<String> {
    let Some(last) = data
        .values()
        .filter_map(|series| series.candles.last().map(|candle| candle.time))
        .max()
    else {
        return Vec::new();
    };
    data.iter()
        .filter(|(_, series)| series.candles.last().is_some_and(|c| c.time < last))
        .map(|(symbol, _)| symbol.clone())
        .collect()
}

/// Validate and convert the input for every symbol the spec's universe names,
/// rejecting a symbol whose feeds do not cover what the spec's indicators
/// consume.
///
/// Data for a symbol outside the universe is dropped here rather than folded.
/// The universe is what the spec asks to be screened; scanning whatever the
/// caller happened to send instead would make the field decorative.
fn ingest(
    data: BTreeMap<String, SymbolInput>,
    spec: &ScanSpec,
) -> Result<BTreeMap<String, CoreSeries>> {
    let wanted = spec.universe_set();
    let mut out = BTreeMap::new();
    for (symbol, input) in data {
        if !wanted.contains(symbol.as_str()) {
            continue;
        }
        let series = CoreSeries::build(&symbol, input.into_series())?;
        out.insert(symbol, series);
    }
    Ok(out)
}

/// Evaluate an already-folded universe against the spec: filter matches, collect
/// their values and rank score, then sort and limit. Shared by `scan_batch` and
/// the streaming `Screener::evaluate`; the spec is assumed already validated.
pub(crate) fn evaluate_universe(
    universe: &Universe,
    spec: &ScanSpec,
    scanned: usize,
    missing: Vec<String>,
    stale: Vec<String>,
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
    ScanReport {
        matches,
        scanned,
        missing,
        stale,
        timeframe: spec.timeframe.clone(),
    }
}

/// Fold the whole universe one timestamp at a time.
///
/// The timeline is the sorted *union* of the bar timestamps, not the
/// intersection: one delisted symbol must not rewind the scan for everyone else.
/// At each timestamp only the symbols that printed a bar advance and the rest
/// hold the state they had. Because a symbol's indicators only ever see that
/// symbol's own bars, this yields exactly the states the per-symbol fold yields.
/// What changes is that the cross-section each of them reads is assembled from
/// the members of that same timestamp, which is what a rank or a breadth reading
/// has to mean.
fn lockstep_states(
    data: &BTreeMap<String, CoreSeries>,
    spec: &ScanSpec,
) -> Result<BTreeMap<String, SymbolState>> {
    let mut timeline: Vec<i64> = data
        .values()
        .flat_map(|series| series.candles.iter().map(|candle| candle.time))
        .collect();
    timeline.sort_unstable();
    timeline.dedup();

    let breadth_spec = spec.breadth.clone().unwrap_or_default();
    let mut states = BTreeMap::new();
    let mut breadth = BTreeMap::new();
    let mut cursors: BTreeMap<&str, usize> = BTreeMap::new();
    for symbol in data.keys() {
        states.insert(symbol.clone(), SymbolState::new(spec)?);
        breadth.insert(symbol.clone(), BreadthState::new(&breadth_spec)?);
        cursors.insert(symbol.as_str(), 0);
    }

    let mut reference_close: Option<f64> = None;
    for timestamp in timeline {
        // First pass: which symbols printed at this timestamp, and the member
        // each contributes. The panel has to exist before any indicator reads it.
        let mut printing: Vec<&str> = Vec::new();
        let mut members = Vec::new();
        for (symbol, series) in data {
            let index = cursors[symbol.as_str()];
            let Some(candle) = series.candles.get(index) else {
                continue;
            };
            if candle.time != timestamp {
                continue;
            }
            printing.push(symbol.as_str());
            members.push(
                breadth
                    .get_mut(symbol)
                    .expect("a breadth state per symbol")
                    .update(candle),
            );
            if spec.reference.as_deref() == Some(symbol.as_str()) {
                reference_close = Some(candle.close);
            }
        }
        let section = assemble(members, timestamp);

        // Second pass: fold each printing symbol against the panel of this bar.
        for symbol in printing {
            let series = &data[symbol];
            let index = cursors[symbol];
            let candle = &series.candles[index];
            let mut feeds = series.bar(index);
            // A feed the caller supplied wins: an explicit panel or reference is
            // a statement about the data, and the derived one is a convenience.
            if feeds.cross_section.is_none() {
                feeds.cross_section = section.as_ref();
            }
            if feeds.reference.is_none() {
                feeds.reference = reference_close;
            }
            states
                .get_mut(symbol)
                .expect("a state per symbol")
                .fold(candle, feeds);
            *cursors.get_mut(symbol).expect("a cursor per symbol") += 1;
        }
    }
    Ok(states)
}

/// Build a fully-folded state per symbol, in parallel with rayon.
#[cfg(feature = "parallel")]
fn folded_states(
    data: &BTreeMap<String, CoreSeries>,
    spec: &ScanSpec,
) -> Result<BTreeMap<String, SymbolState>> {
    use rayon::prelude::*;
    let built: Vec<Result<(String, SymbolState)>> = data
        .par_iter()
        .map(|(symbol, series)| Ok((symbol.clone(), fold_symbol(series, spec)?)))
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
    data: &BTreeMap<String, CoreSeries>,
    spec: &ScanSpec,
) -> Result<BTreeMap<String, SymbolState>> {
    let mut states = BTreeMap::new();
    for (symbol, series) in data {
        states.insert(symbol.clone(), fold_symbol(series, spec)?);
    }
    Ok(states)
}

/// Fold one symbol's history, bar and feeds together, into a ready state.
fn fold_symbol(series: &CoreSeries, spec: &ScanSpec) -> Result<SymbolState> {
    let mut state = SymbolState::new(spec)?;
    for (index, candle) in series.candles.iter().enumerate() {
        state.fold(candle, series.bar(index));
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
    use wickra_backtest_core::Candle;

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

    fn data() -> BTreeMap<String, SymbolInput> {
        BTreeMap::from([
            ("A".to_string(), vec![candle(10.0)].into()),
            ("B".to_string(), vec![candle(20.0)].into()),
            ("C".to_string(), vec![candle(30.0)].into()),
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
            reference: None,
            breadth: None,
            condition: gt15(),
            rank: Some(RankSpec {
                by: close(),
                desc: true,
            }),
            limit: Some(2),
        };
        let report = scan_batch(data(), &spec).unwrap();
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
            reference: None,
            breadth: None,
            condition: gt15(),
            rank: None,
            limit: None,
        };
        let report = scan_batch(data(), &spec).unwrap();
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
            reference: None,
            breadth: None,
            condition: Condition::CrossSection {
                expr: close(),
                metric: CsMetric::Rank,
                op: Comparator::Le,
                value: 1.0,
            },
            rank: None,
            limit: None,
        };
        let report = scan_batch(data(), &spec).unwrap();
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].symbol, "C");
        assert!(approx(report.matches[0].values["price.close#rank"], 1.0));
    }

    #[test]
    fn report_round_trips_as_json() {
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            reference: None,
            breadth: None,
            condition: gt15(),
            rank: None,
            limit: None,
        };
        let report = scan_batch(data(), &spec).unwrap();
        let json = serde_json::to_string(&report).unwrap();
        assert_eq!(serde_json::from_str::<ScanReport>(&json).unwrap(), report);
    }

    #[test]
    fn score_ties_break_by_symbol() {
        let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
            ("A".to_string(), vec![candle(20.0)].into()),
            ("B".to_string(), vec![candle(20.0)].into()),
            ("C".to_string(), vec![candle(30.0)].into()),
        ]);
        let spec = ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            reference: None,
            breadth: None,
            condition: gt15(),
            rank: Some(RankSpec {
                by: close(),
                desc: true,
            }),
            limit: None,
        };
        let report = scan_batch(data, &spec).unwrap();
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
            reference: None,
            breadth: None,
            condition: gt15(),
            rank: None,
            limit: None,
        };
        let first = scan_batch(data(), &spec).unwrap();
        let second = scan_batch(data(), &spec).unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }
}
