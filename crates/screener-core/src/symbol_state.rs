//! Per-symbol state: the indicators a spec needs, folded one candle at a time,
//! plus the current and previous candle for price-field expressions.

use crate::error::Result;
use crate::expr::{Expr, PriceField};
use crate::indicator_set::IndicatorSet;
use crate::spec::{Condition, ScanSpec};
use wickra_backtest_core::Candle;

/// The rolling state of one symbol: its indicator set, a bar counter, a readiness
/// flag (past the largest warmup) and the current/previous candle.
pub(crate) struct SymbolState {
    inds: IndicatorSet,
    warmup: usize,
    bars: usize,
    ready: bool,
    cur: Option<Candle>,
    prev: Option<Candle>,
}

impl SymbolState {
    /// Build the state for a spec: register every indicator referenced in the
    /// condition tree and the optional rank expression. Errors if the registry
    /// does not know an indicator.
    pub(crate) fn new(spec: &ScanSpec) -> Result<Self> {
        let mut inds = IndicatorSet::new();
        collect_exprs(&spec.condition, &mut |e| inds.required(e))?;
        if let Some(rank) = &spec.rank {
            inds.required(&rank.by)?;
        }
        let warmup = inds.max_warmup();
        Ok(Self {
            inds,
            warmup,
            bars: 0,
            ready: false,
            cur: None,
            prev: None,
        })
    }

    /// Fold one candle in O(1): tick every indicator, shift the candle window and
    /// update the readiness flag.
    pub(crate) fn fold(&mut self, candle: &Candle) {
        self.inds.update(candle);
        self.prev = self.cur.take();
        self.cur = Some(*candle);
        self.bars += 1;
        self.ready = self.bars >= self.warmup && self.bars > 0;
    }

    /// Whether the symbol is past the largest indicator warmup and has data.
    pub(crate) fn is_ready(&self) -> bool {
        self.ready
    }

    /// The current value of an expression: a constant directly, a price field
    /// from the current candle, or an indicator output by its canonical key.
    pub(crate) fn expr_cur(&self, expr: &Expr) -> Option<f64> {
        match expr {
            Expr::Const { value } => Some(*value),
            Expr::Price { field } => self.cur.as_ref().map(|c| price_field(c, *field)),
            Expr::Indicator { .. } => self.inds.cur(&expr.key()),
        }
    }

    /// The previous-bar value of an expression (constants are unchanged).
    pub(crate) fn expr_prev(&self, expr: &Expr) -> Option<f64> {
        match expr {
            Expr::Const { value } => Some(*value),
            Expr::Price { field } => self.prev.as_ref().map(|c| price_field(c, *field)),
            Expr::Indicator { .. } => self.inds.prev(&expr.key()),
        }
    }
}

/// Read a price field from a candle.
fn price_field(candle: &Candle, field: PriceField) -> f64 {
    match field {
        PriceField::Open => candle.open,
        PriceField::High => candle.high,
        PriceField::Low => candle.low,
        PriceField::Close => candle.close,
        PriceField::Volume => candle.volume,
    }
}

/// Visit every expression in a condition tree, short-circuiting on error.
fn collect_exprs<F>(cond: &Condition, visit: &mut F) -> Result<()>
where
    F: FnMut(&Expr) -> Result<()>,
{
    match cond {
        Condition::Cmp { left, right, .. } => {
            visit(left)?;
            visit(right)
        }
        Condition::CrossSection { expr, .. } => visit(expr),
        Condition::Breadth { inner, .. } => collect_exprs(inner, visit),
        Condition::All { conditions } | Condition::Any { conditions } => {
            for c in conditions {
                collect_exprs(c, visit)?;
            }
            Ok(())
        }
        Condition::Not { condition } => collect_exprs(condition, visit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Comparator;

    fn candle(close: f64) -> Candle {
        Candle {
            time: 0,
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 100.0,
        }
    }

    fn spec_with(cond: Condition) -> ScanSpec {
        ScanSpec {
            universe: vec!["A".into()],
            timeframe: None,
            condition: cond,
            rank: None,
            limit: None,
        }
    }

    #[test]
    fn folds_and_reads_expressions() {
        let sma = Expr::Indicator {
            name: "Sma".into(),
            params: vec![3.0],
            field: None,
        };
        let close = Expr::Price {
            field: PriceField::Close,
        };
        let cond = Condition::Cmp {
            left: sma.clone(),
            op: Comparator::Gt,
            right: close.clone(),
        };
        let mut state = SymbolState::new(&spec_with(cond)).unwrap();
        assert!(!state.is_ready());

        for c in [1.0, 2.0, 3.0] {
            state.fold(&candle(c));
        }
        assert!(state.is_ready());
        assert_eq!(state.expr_cur(&sma), Some(2.0)); // (1+2+3)/3
        assert_eq!(state.expr_cur(&close), Some(3.0));
        assert_eq!(state.expr_cur(&Expr::Const { value: 9.0 }), Some(9.0));

        state.fold(&candle(6.0));
        assert_eq!(state.expr_cur(&close), Some(6.0));
        assert_eq!(state.expr_prev(&close), Some(3.0));
        assert_eq!(state.expr_prev(&sma), Some(2.0));
    }

    #[test]
    fn unknown_indicator_fails_new() {
        let cond = Condition::Cmp {
            left: Expr::Indicator {
                name: "NopeIndicator".into(),
                params: vec![],
                field: None,
            },
            op: Comparator::Gt,
            right: Expr::Const { value: 0.0 },
        };
        assert!(SymbolState::new(&spec_with(cond)).is_err());
    }
}
