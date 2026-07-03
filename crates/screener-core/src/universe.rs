//! The universe of symbols: one [`SymbolState`] each, plus the cross-section
//! reductions (rank, percentile, z-score) that need every symbol of a bar at
//! once.
//!
//! Consumed by `eval` (P-SCR-1.8); the module-level `dead_code` allow is removed
//! once that module wires it in.
#![allow(dead_code)]

use crate::error::Result;
use crate::expr::Expr;
use crate::spec::{CsMetric, ScanSpec};
use crate::symbol_state::SymbolState;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use wickra_backtest_core::Candle;

/// The set of symbols being scanned, keyed by symbol name.
pub(crate) struct Universe {
    /// Per-symbol state, iterated in key order (`BTreeMap`) for determinism.
    pub(crate) symbols: BTreeMap<String, SymbolState>,
}

impl Universe {
    /// An empty universe.
    pub(crate) fn new() -> Self {
        Self {
            symbols: BTreeMap::new(),
        }
    }

    /// Ensure a symbol has a state, building it from the spec if absent. Errors
    /// if an indicator the spec references is unknown.
    pub(crate) fn ensure(&mut self, sym: &str, spec: &ScanSpec) -> Result<()> {
        if !self.symbols.contains_key(sym) {
            self.symbols
                .insert(sym.to_string(), SymbolState::new(spec)?);
        }
        Ok(())
    }

    /// Fold one candle into a symbol's state (no-op if the symbol is absent).
    pub(crate) fn fold(&mut self, sym: &str, candle: &Candle) {
        if let Some(state) = self.symbols.get_mut(sym) {
            state.fold(candle);
        }
    }

    /// The number of ready symbols (past the largest indicator warmup).
    pub(crate) fn ready_count(&self) -> usize {
        self.symbols.values().filter(|s| s.is_ready()).count()
    }

    /// The cross-section metric of `expr` for every ready symbol with a defined,
    /// finite value, keyed by symbol. Reductions run serially in symbol-key
    /// order so the f64 rounding is identical everywhere (§6.5).
    pub(crate) fn cross_section(&self, expr: &Expr, metric: CsMetric) -> BTreeMap<String, f64> {
        // R: ready symbols with a defined, finite value, in key order.
        let values: Vec<(String, f64)> = self
            .symbols
            .iter()
            .filter(|(_, s)| s.is_ready())
            .filter_map(|(k, s)| {
                s.expr_cur(expr)
                    .filter(|v| v.is_finite())
                    .map(|v| (k.clone(), v))
            })
            .collect();

        let mut out = BTreeMap::new();
        let n = values.len();
        if n == 0 {
            return out;
        }

        match metric {
            CsMetric::Rank => {
                // Sort by (value desc, key asc); rank is the 1-based index.
                let mut order: Vec<&(String, f64)> = values.iter().collect();
                order.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1)
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| a.0.cmp(&b.0))
                });
                for (idx, (key, _)) in order.iter().enumerate() {
                    out.insert((*key).clone(), (idx + 1) as f64);
                }
            }
            CsMetric::PercentileRank => {
                for (key, value) in &values {
                    let less = values.iter().filter(|(_, other)| other < value).count();
                    let pct = if n > 1 {
                        less as f64 / (n - 1) as f64
                    } else {
                        0.0
                    };
                    out.insert(key.clone(), pct);
                }
            }
            CsMetric::ZScore => {
                let mean = values.iter().map(|(_, v)| *v).sum::<f64>() / n as f64;
                let variance = values
                    .iter()
                    .map(|(_, v)| {
                        let d = *v - mean;
                        d * d
                    })
                    .sum::<f64>()
                    / n as f64;
                let std = variance.sqrt();
                for (key, value) in &values {
                    let z = if std == 0.0 {
                        0.0
                    } else {
                        (*value - mean) / std
                    };
                    out.insert(key.clone(), z);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::PriceField;
    use crate::spec::{Comparator, Condition};

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

    fn price_spec() -> ScanSpec {
        // A price-only condition: no indicators, so symbols are ready after one bar.
        ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: Condition::Cmp {
                left: Expr::Price {
                    field: PriceField::Close,
                },
                op: Comparator::Gt,
                right: Expr::Const { value: 0.0 },
            },
            rank: None,
            limit: None,
        }
    }

    fn seeded_universe() -> Universe {
        let spec = price_spec();
        let mut u = Universe::new();
        for (sym, close) in [("A", 10.0), ("B", 20.0), ("C", 30.0)] {
            u.ensure(sym, &spec).unwrap();
            u.fold(sym, &candle(close));
        }
        u
    }

    #[test]
    fn ready_count_counts_ready_symbols() {
        assert_eq!(seeded_universe().ready_count(), 3);
    }

    #[test]
    fn rank_is_highest_first() {
        let u = seeded_universe();
        let close = Expr::Price {
            field: PriceField::Close,
        };
        let rank = u.cross_section(&close, CsMetric::Rank);
        assert_eq!(rank["C"], 1.0);
        assert_eq!(rank["B"], 2.0);
        assert_eq!(rank["A"], 3.0);
    }

    #[test]
    fn percentile_rank_in_unit_interval() {
        let u = seeded_universe();
        let close = Expr::Price {
            field: PriceField::Close,
        };
        let pct = u.cross_section(&close, CsMetric::PercentileRank);
        assert_eq!(pct["A"], 0.0);
        assert_eq!(pct["B"], 0.5);
        assert_eq!(pct["C"], 1.0);
    }

    #[test]
    fn zscore_is_centred() {
        let u = seeded_universe();
        let close = Expr::Price {
            field: PriceField::Close,
        };
        let z = u.cross_section(&close, CsMetric::ZScore);
        assert!(z["B"].abs() < 1e-12);
        assert!(z["A"] < 0.0);
        assert!(z["C"] > 0.0);
    }
}
