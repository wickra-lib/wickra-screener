//! Per-symbol state: the indicators a spec needs, folded one bar at a time, plus
//! the recent bars an expression can look back over.
//!
//! The window is exactly as deep as the spec reaches. A crossover needs the bar
//! before; a `prev(...,10)` needs ten; anything deeper than the spec asks for
//! would be memory held for nothing, and anything shallower would silently turn
//! a lookback into no value at all.

use std::collections::{BTreeMap, VecDeque};

use crate::error::Result;
use crate::expr::{Expr, PriceField};
use crate::feeds::BarFeeds;
use crate::indicator_set::IndicatorSet;
use crate::spec::ScanSpec;
use wickra_backtest_core::Candle;

/// One folded bar: the candle and every indicator value computed on it.
struct Snapshot {
    candle: Candle,
    values: BTreeMap<String, f64>,
}

/// The rolling state of one symbol: its indicator set, a bar counter, a readiness
/// flag (past the largest warmup) and a bounded window of folded bars.
pub(crate) struct SymbolState {
    inds: IndicatorSet,
    warmup: usize,
    bars: usize,
    ready: bool,
    /// Most recent bar first, so `history[n]` is the bar `n` back.
    history: VecDeque<Snapshot>,
    depth: usize,
}

impl SymbolState {
    /// Build the state for a spec: register every indicator referenced anywhere
    /// in the condition tree or the ranking expression, and size the lookback
    /// window to the deepest the spec reaches. Errors if the registry does not
    /// know an indicator.
    pub(crate) fn new(spec: &ScanSpec) -> Result<Self> {
        let mut inds = IndicatorSet::new();
        spec.visit_exprs(&mut |expr| inds.required(expr))?;
        let warmup = inds.max_warmup();
        Ok(Self {
            inds,
            warmup,
            bars: 0,
            ready: false,
            history: VecDeque::new(),
            // The current bar plus everything the spec looks back over. A
            // crossover reads the bar before, so one is the floor.
            depth: 1 + spec.lookback().max(1),
        })
    }

    /// Fold one bar in O(1): tick every indicator against the candle and the
    /// bar's side feeds, push the snapshot and drop the one that fell out of the
    /// window.
    pub(crate) fn fold(&mut self, candle: &Candle, feeds: BarFeeds<'_>) {
        let values = self.inds.update(candle, feeds);
        self.history.push_front(Snapshot {
            candle: *candle,
            values,
        });
        if self.history.len() > self.depth {
            self.history.pop_back();
        }
        self.bars += 1;
        self.ready = self.bars >= self.warmup && self.bars > 0;
    }

    /// Whether the symbol is past the largest indicator warmup and has data.
    pub(crate) fn is_ready(&self) -> bool {
        self.ready
    }

    /// The value of an expression `back` bars ago, `0` being the current bar.
    ///
    /// A compound form reads both sides at the same bar, and a `prev` shifts the
    /// bar it reads at rather than the value it returns, so `prev(a - b, 2)` and
    /// `prev(a, 2) - prev(b, 2)` are the same thing.
    pub(crate) fn expr_at(&self, expr: &Expr, back: usize) -> Option<f64> {
        match expr {
            Expr::Const { value } => Some(*value),
            Expr::Price { field } => self
                .history
                .get(back)
                .map(|snap| price_field(&snap.candle, *field)),
            Expr::Indicator { .. } => self
                .history
                .get(back)
                .and_then(|snap| snap.values.get(&expr.key()).copied()),
            Expr::Prev { of, bars } => self.expr_at(of, back + *bars as usize),
            Expr::Add { left, right } => self.arithmetic(left, right, back, |a, b| a + b),
            Expr::Sub { left, right } => self.arithmetic(left, right, back, |a, b| a - b),
            Expr::Mul { left, right } => self.arithmetic(left, right, back, |a, b| a * b),
            Expr::Div { left, right } => self.arithmetic(left, right, back, |a, b| a / b),
        }
    }

    /// The current value of an expression.
    pub(crate) fn expr_cur(&self, expr: &Expr) -> Option<f64> {
        self.expr_at(expr, 0)
    }

    /// The previous-bar value of an expression.
    pub(crate) fn expr_prev(&self, expr: &Expr) -> Option<f64> {
        self.expr_at(expr, 1)
    }

    /// Combine two expressions at the same bar, dropping a result that is not
    /// finite so a condition over it is false rather than surprising. This is
    /// where a division by zero stops.
    fn arithmetic(
        &self,
        left: &Expr,
        right: &Expr,
        back: usize,
        combine: fn(f64, f64) -> f64,
    ) -> Option<f64> {
        let a = self.expr_at(left, back)?;
        let b = self.expr_at(right, back)?;
        let value = combine(a, b);
        value.is_finite().then_some(value)
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
        PriceField::Hlc3 => (candle.high + candle.low + candle.close) / 3.0,
        PriceField::Ohlc4 => (candle.open + candle.high + candle.low + candle.close) / 4.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Comparator, Condition};

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
            reference: None,
            breadth: None,
            condition: cond,
            rank: None,
            limit: None,
        }
    }

    fn close_expr() -> Expr {
        Expr::Price {
            field: PriceField::Close,
        }
    }

    fn always(expr: Expr) -> Condition {
        Condition::Cmp {
            left: expr,
            op: Comparator::Gt,
            right: Expr::Const { value: f64::MIN },
        }
    }

    #[test]
    fn folds_and_reads_expressions() {
        let sma = Expr::Indicator {
            name: "Sma".into(),
            params: vec![3.0],
            field: None,
        };
        let cond = Condition::Cmp {
            left: sma.clone(),
            op: Comparator::Gt,
            right: close_expr(),
        };
        let mut state = SymbolState::new(&spec_with(cond)).unwrap();
        assert!(!state.is_ready());

        for c in [1.0, 2.0, 3.0] {
            state.fold(&candle(c), BarFeeds::default());
        }
        assert!(state.is_ready());
        assert_eq!(state.expr_cur(&sma), Some(2.0)); // (1+2+3)/3
        assert_eq!(state.expr_cur(&close_expr()), Some(3.0));
        assert_eq!(state.expr_cur(&Expr::Const { value: 9.0 }), Some(9.0));

        state.fold(&candle(6.0), BarFeeds::default());
        assert_eq!(state.expr_cur(&close_expr()), Some(6.0));
        assert_eq!(state.expr_prev(&close_expr()), Some(3.0));
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

    #[test]
    fn the_window_is_as_deep_as_the_spec_reaches() {
        let five_back = Expr::Prev {
            of: Box::new(close_expr()),
            bars: 5,
        };
        let mut state = SymbolState::new(&spec_with(always(five_back.clone()))).unwrap();
        for c in 1..=8 {
            state.fold(&candle(f64::from(c)), BarFeeds::default());
        }
        // Eight bars in, five back from the last (8) is 3.
        assert_eq!(state.expr_cur(&five_back), Some(3.0));

        // A spec that never looks back keeps only what a crossover needs, so the
        // same lookback finds nothing rather than reading a stale bar.
        let mut shallow = SymbolState::new(&spec_with(always(close_expr()))).unwrap();
        for c in 1..=8 {
            shallow.fold(&candle(f64::from(c)), BarFeeds::default());
        }
        assert_eq!(shallow.expr_at(&five_back, 0), None);
    }

    #[test]
    fn arithmetic_reads_both_sides_at_one_bar() {
        let gap = Expr::Sub {
            left: Box::new(close_expr()),
            right: Box::new(Expr::Const { value: 2.0 }),
        };
        let mut state = SymbolState::new(&spec_with(always(gap.clone()))).unwrap();
        state.fold(&candle(10.0), BarFeeds::default());
        state.fold(&candle(15.0), BarFeeds::default());
        assert_eq!(state.expr_cur(&gap), Some(13.0));
        assert_eq!(state.expr_prev(&gap), Some(8.0));
    }

    #[test]
    fn a_lookback_over_a_compound_shifts_the_bar_it_reads() {
        let gap = Expr::Sub {
            left: Box::new(close_expr()),
            right: Box::new(Expr::Const { value: 1.0 }),
        };
        let shifted = Expr::Prev {
            of: Box::new(gap),
            bars: 2,
        };
        let distributed = Expr::Sub {
            left: Box::new(Expr::Prev {
                of: Box::new(close_expr()),
                bars: 2,
            }),
            right: Box::new(Expr::Const { value: 1.0 }),
        };
        let cond = Condition::All {
            conditions: vec![always(shifted.clone()), always(distributed.clone())],
        };
        let mut state = SymbolState::new(&spec_with(cond)).unwrap();
        for c in [10.0, 20.0, 30.0, 40.0] {
            state.fold(&candle(c), BarFeeds::default());
        }
        assert_eq!(state.expr_cur(&shifted), Some(19.0));
        assert_eq!(state.expr_cur(&distributed), state.expr_cur(&shifted));
    }

    #[test]
    fn a_division_that_is_not_finite_has_no_value() {
        let ratio = Expr::Div {
            left: Box::new(close_expr()),
            right: Box::new(Expr::Const { value: 0.0 }),
        };
        let mut state = SymbolState::new(&spec_with(always(ratio.clone()))).unwrap();
        state.fold(&candle(10.0), BarFeeds::default());
        assert_eq!(state.expr_cur(&ratio), None);
    }

    #[test]
    fn the_averaged_price_fields_read_off_the_candle() {
        let hlc3 = Expr::Price {
            field: PriceField::Hlc3,
        };
        let ohlc4 = Expr::Price {
            field: PriceField::Ohlc4,
        };
        let cond = Condition::All {
            conditions: vec![always(hlc3.clone()), always(ohlc4.clone())],
        };
        let mut state = SymbolState::new(&spec_with(cond)).unwrap();
        // open 10, high 11, low 9, close 10.
        state.fold(&candle(10.0), BarFeeds::default());
        assert_eq!(state.expr_cur(&hlc3), Some((11.0 + 9.0 + 10.0) / 3.0));
        assert_eq!(
            state.expr_cur(&ohlc4),
            Some((10.0 + 11.0 + 9.0 + 10.0) / 4.0)
        );
    }
}
