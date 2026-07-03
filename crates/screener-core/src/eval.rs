//! Condition evaluation: turns a condition tree into a boolean for one symbol.
//!
//! (The handoff sketched a `CsCtx<'a>` wrapper around `&Universe`; passing the
//! `&Universe` directly is equivalent and avoids a redundant wrapper.)
//!
//! Consumed by `scan` (P-SCR-1.9); the module-level `dead_code` allow is removed
//! once that module wires it in.
#![allow(dead_code)]

use crate::expr::Expr;
use crate::spec::{Comparator, Condition};
use crate::universe::Universe;

/// Evaluate a condition for `sym` against the whole `universe`. Missing values
/// (a symbol still warming up, or an indicator without an output) make the
/// condition false. Boolean combinators short-circuit.
pub(crate) fn eval_condition(cond: &Condition, sym: &str, universe: &Universe) -> bool {
    match cond {
        Condition::Cmp { left, op, right } => eval_cmp(left, *op, right, sym, universe),
        Condition::CrossSection {
            expr,
            metric,
            op,
            value,
        } => match universe.cross_section(expr, *metric).get(sym) {
            Some(v) => point_compare(*v, *op, *value),
            None => false,
        },
        Condition::Breadth { inner, op, ratio } => {
            let n = universe.ready_count();
            let ratio_universe = if n == 0 {
                0.0
            } else {
                let hits = universe
                    .symbols
                    .iter()
                    .filter(|(_, s)| s.is_ready())
                    .filter(|(k, _)| eval_condition(inner, k, universe))
                    .count();
                hits as f64 / n as f64
            };
            point_compare(ratio_universe, *op, *ratio)
        }
        Condition::All { conditions } => {
            conditions.iter().all(|c| eval_condition(c, sym, universe))
        }
        Condition::Any { conditions } => {
            conditions.iter().any(|c| eval_condition(c, sym, universe))
        }
        Condition::Not { condition } => !eval_condition(condition, sym, universe),
    }
}

/// Evaluate a comparison between two expressions for one symbol.
fn eval_cmp(left: &Expr, op: Comparator, right: &Expr, sym: &str, universe: &Universe) -> bool {
    let Some(state) = universe.symbols.get(sym) else {
        return false;
    };
    match op {
        Comparator::CrossesAbove => {
            match (
                state.expr_prev(left),
                state.expr_prev(right),
                state.expr_cur(left),
                state.expr_cur(right),
            ) {
                (Some(pl), Some(pr), Some(cl), Some(cr)) => pl <= pr && cl > cr,
                _ => false,
            }
        }
        Comparator::CrossesBelow => {
            match (
                state.expr_prev(left),
                state.expr_prev(right),
                state.expr_cur(left),
                state.expr_cur(right),
            ) {
                (Some(pl), Some(pr), Some(cl), Some(cr)) => pl >= pr && cl < cr,
                _ => false,
            }
        }
        _ => match (state.expr_cur(left), state.expr_cur(right)) {
            (Some(a), Some(b)) => point_compare(a, op, b),
            _ => false,
        },
    }
}

/// A point comparison of two values. Crossover operators are not point
/// comparisons (they are handled in `eval_cmp`) and yield false here.
fn point_compare(a: f64, op: Comparator, b: f64) -> bool {
    match op {
        Comparator::Gt => a > b,
        Comparator::Ge => a >= b,
        Comparator::Lt => a < b,
        Comparator::Le => a <= b,
        // Relative tolerance, not bit equality: the golden compares report JSON.
        Comparator::Eq => (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0),
        Comparator::CrossesAbove | Comparator::CrossesBelow => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::PriceField;
    use crate::spec::{CsMetric, ScanSpec};
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

    fn spec(cond: Condition) -> ScanSpec {
        ScanSpec {
            universe: vec!["A".into(), "B".into(), "C".into()],
            timeframe: None,
            condition: cond,
            rank: None,
            limit: None,
        }
    }

    /// A, B, C at closes 10, 20, 30, each folded once.
    fn seeded(cond: Condition) -> Universe {
        let s = spec(cond);
        let mut u = Universe::new();
        for (sym, c) in [("A", 10.0), ("B", 20.0), ("C", 30.0)] {
            u.ensure(sym, &s).unwrap();
            u.fold(sym, &candle(c));
        }
        u
    }

    #[test]
    fn cmp_greater_than() {
        let cond = Condition::Cmp {
            left: close(),
            op: Comparator::Gt,
            right: Expr::Const { value: 15.0 },
        };
        let u = seeded(cond.clone());
        assert!(!eval_condition(&cond, "A", &u));
        assert!(eval_condition(&cond, "B", &u));
        assert!(eval_condition(&cond, "C", &u));
    }

    #[test]
    fn cross_section_top_one() {
        let cond = Condition::CrossSection {
            expr: close(),
            metric: CsMetric::Rank,
            op: Comparator::Le,
            value: 1.0,
        };
        let u = seeded(cond.clone());
        assert!(!eval_condition(&cond, "A", &u));
        assert!(!eval_condition(&cond, "B", &u));
        assert!(eval_condition(&cond, "C", &u)); // highest close = rank 1
    }

    #[test]
    fn breadth_is_a_universe_gate() {
        let inner = Condition::Cmp {
            left: close(),
            op: Comparator::Gt,
            right: Expr::Const { value: 15.0 },
        };
        // Two of three symbols pass the inner condition -> ratio 2/3 >= 0.6.
        let cond = Condition::Breadth {
            inner: Box::new(inner),
            op: Comparator::Ge,
            ratio: 0.6,
        };
        let u = seeded(cond.clone());
        // Passes for every symbol (universe-wide gate), even the failing A.
        assert!(eval_condition(&cond, "A", &u));
        assert!(eval_condition(&cond, "C", &u));
    }

    #[test]
    fn all_any_not_combine() {
        let gt15 = Condition::Cmp {
            left: close(),
            op: Comparator::Gt,
            right: Expr::Const { value: 15.0 },
        };
        let lt25 = Condition::Cmp {
            left: close(),
            op: Comparator::Lt,
            right: Expr::Const { value: 25.0 },
        };
        let all = Condition::All {
            conditions: vec![gt15.clone(), lt25.clone()],
        };
        let u = seeded(all.clone());
        assert!(!eval_condition(&all, "A", &u)); // 10: fails gt15
        assert!(eval_condition(&all, "B", &u)); // 20: passes both
        assert!(!eval_condition(&all, "C", &u)); // 30: fails lt25

        let not = Condition::Not {
            condition: Box::new(gt15),
        };
        assert!(eval_condition(&not, "A", &u));
        assert!(!eval_condition(&not, "B", &u));
    }

    #[test]
    fn crossover_needs_two_bars() {
        let cond = Condition::Cmp {
            left: close(),
            op: Comparator::CrossesAbove,
            right: Expr::Const { value: 15.0 },
        };
        let s = spec(cond.clone());
        let mut u = Universe::new();
        u.ensure("A", &s).unwrap();
        u.fold("A", &candle(10.0)); // below 15
        assert!(!eval_condition(&cond, "A", &u)); // no prev cross yet? prev==None first bar
        u.fold("A", &candle(20.0)); // above 15
        assert!(eval_condition(&cond, "A", &u)); // prev 10<=15, cur 20>15
    }
}
