//! The operand and condition grammar.
//!
//! The spec could compare an indicator, a price field or a constant, and nothing
//! else. It could not say "the gap between price and its average", "a fifth
//! above its own low", "higher than it was ten bars ago", or "between these two
//! levels" — the shapes most screens are actually written around. The set here
//! mirrors `wickra_backtest_core::spec::{Operand, OperandExpr, Condition}`, so a
//! screen can say what a strategy can.

use std::collections::BTreeMap;

use screener_core::{
    scan_batch, Candle, Comparator, Condition, Expr, PriceField, ScanSpec, SymbolInput,
};

const SYMBOL: &str = "AAA";

fn candle(time: i64, close: f64) -> Candle {
    Candle {
        time,
        open: close - 1.0,
        high: close + 2.0,
        low: close - 2.0,
        close,
        volume: 100.0,
    }
}

/// A universe of one symbol walking the given closes.
fn data(closes: &[f64]) -> BTreeMap<String, SymbolInput> {
    let candles: Vec<Candle> = closes
        .iter()
        .enumerate()
        .map(|(i, close)| candle(i64::try_from(i).expect("bar index fits i64"), *close))
        .collect();
    BTreeMap::from([(SYMBOL.to_string(), candles.into())])
}

fn spec(condition: Condition) -> ScanSpec {
    ScanSpec {
        universe: vec![SYMBOL.to_string()],
        timeframe: None,
        reference: None,
        breadth: None,
        condition,
        rank: None,
        limit: None,
    }
}

/// Whether the single symbol matched.
fn matches(closes: &[f64], condition: Condition) -> bool {
    let report = scan_batch(data(closes), &spec(condition)).expect("scan");
    !report.matches.is_empty()
}

fn close() -> Expr {
    Expr::Price {
        field: PriceField::Close,
    }
}

fn constant(value: f64) -> Expr {
    Expr::Const { value }
}

fn cmp(left: Expr, op: Comparator, right: Expr) -> Condition {
    Condition::Cmp { left, op, right }
}

// --- comparators -------------------------------------------------------------

#[test]
fn ne_is_the_negation_of_eq() {
    let closes = [10.0, 20.0];
    assert!(matches(
        &closes,
        cmp(close(), Comparator::Ne, constant(19.0))
    ));
    assert!(!matches(
        &closes,
        cmp(close(), Comparator::Ne, constant(20.0))
    ));
    // And the pair really is complementary at the same tolerance.
    assert!(matches(
        &closes,
        cmp(close(), Comparator::Eq, constant(20.0))
    ));
}

// --- arithmetic --------------------------------------------------------------

#[test]
fn arithmetic_combines_expressions() {
    let closes = [10.0, 20.0];

    // close - 5 == 15
    let gap = Expr::Sub {
        left: Box::new(close()),
        right: Box::new(constant(5.0)),
    };
    assert!(matches(&closes, cmp(gap, Comparator::Eq, constant(15.0))));

    // close / 4 == 5
    let ratio = Expr::Div {
        left: Box::new(close()),
        right: Box::new(constant(4.0)),
    };
    assert!(matches(&closes, cmp(ratio, Comparator::Eq, constant(5.0))));

    // (close + 10) * 2 == 60
    let compound = Expr::Mul {
        left: Box::new(Expr::Add {
            left: Box::new(close()),
            right: Box::new(constant(10.0)),
        }),
        right: Box::new(constant(2.0)),
    };
    assert!(matches(
        &closes,
        cmp(compound, Comparator::Eq, constant(60.0))
    ));
}

#[test]
fn a_division_by_zero_matches_nothing_rather_than_infinity() {
    let ratio = Expr::Div {
        left: Box::new(close()),
        right: Box::new(constant(0.0)),
    };
    // Neither the comparison nor its negation holds: the expression has no value.
    assert!(!matches(
        &[10.0, 20.0],
        cmp(ratio.clone(), Comparator::Gt, constant(0.0))
    ));
    assert!(!matches(
        &[10.0, 20.0],
        cmp(ratio, Comparator::Le, constant(0.0))
    ));
}

// --- lookback ----------------------------------------------------------------

#[test]
fn prev_reads_the_bar_it_names() {
    let closes = [10.0, 20.0, 30.0, 40.0];
    for (bars, expected) in [(0_u32, 40.0), (1, 30.0), (3, 10.0)] {
        let expr = Expr::Prev {
            of: Box::new(close()),
            bars,
        };
        assert!(
            matches(&closes, cmp(expr, Comparator::Eq, constant(expected))),
            "prev(close,{bars}) should be {expected}"
        );
    }
}

#[test]
fn a_lookback_past_the_history_has_no_value() {
    let expr = Expr::Prev {
        of: Box::new(close()),
        bars: 10,
    };
    assert!(!matches(
        &[10.0, 20.0],
        cmp(expr, Comparator::Gt, constant(f64::MIN))
    ));
}

// --- the new conditions ------------------------------------------------------

#[test]
fn between_is_inclusive_on_both_ends() {
    let between = |low: f64, high: f64| Condition::Between {
        value: close(),
        low: constant(low),
        high: constant(high),
    };
    let closes = [10.0, 20.0];
    assert!(matches(&closes, between(15.0, 25.0)));
    assert!(matches(&closes, between(20.0, 25.0)), "the low end is in");
    assert!(matches(&closes, between(15.0, 20.0)), "the high end is in");
    assert!(!matches(&closes, between(21.0, 25.0)));
    assert!(!matches(&closes, between(5.0, 19.0)));
}

#[test]
fn rising_and_falling_compare_against_an_earlier_bar() {
    let up = [10.0, 20.0, 30.0, 40.0];
    let down = [40.0, 30.0, 20.0, 10.0];
    let rising = Condition::Rising {
        expr: close(),
        bars: 2,
    };
    let falling = Condition::Falling {
        expr: close(),
        bars: 2,
    };

    assert!(matches(&up, rising.clone()));
    assert!(!matches(&up, falling.clone()));
    assert!(matches(&down, falling));
    assert!(!matches(&down, rising));
}

#[test]
fn a_flat_series_is_neither_rising_nor_falling() {
    let flat = [10.0, 10.0, 10.0, 10.0];
    assert!(!matches(
        &flat,
        Condition::Rising {
            expr: close(),
            bars: 2
        }
    ));
    assert!(!matches(
        &flat,
        Condition::Falling {
            expr: close(),
            bars: 2
        }
    ));
}

#[test]
fn a_zero_bar_lookback_is_refused() {
    let err = spec(Condition::Rising {
        expr: close(),
        bars: 0,
    })
    .validate()
    .expect_err("rising over the bar it stands on can never hold");
    assert!(err.to_string().contains("at least 1 bar"), "{err}");

    let err = spec(Condition::Falling {
        expr: close(),
        bars: 0,
    })
    .validate()
    .expect_err("same for falling");
    assert!(err.to_string().contains("at least 1 bar"), "{err}");
}

// --- price fields ------------------------------------------------------------

#[test]
fn the_averaged_price_fields_are_available_to_a_screen() {
    // candle(20) is open 19, high 22, low 18, close 20.
    let closes = [10.0, 20.0];
    let hlc3 = Expr::Price {
        field: PriceField::Hlc3,
    };
    let ohlc4 = Expr::Price {
        field: PriceField::Ohlc4,
    };
    assert!(matches(
        &closes,
        cmp(hlc3, Comparator::Eq, constant((22.0 + 18.0 + 20.0) / 3.0))
    ));
    assert!(matches(
        &closes,
        cmp(
            ohlc4,
            Comparator::Eq,
            constant((19.0 + 22.0 + 18.0 + 20.0) / 4.0)
        )
    ));
}

// --- what the report says ----------------------------------------------------

#[test]
fn a_compound_expression_explains_itself_in_the_report() {
    let gap = Expr::Sub {
        left: Box::new(close()),
        right: Box::new(constant(5.0)),
    };
    let report = scan_batch(
        data(&[10.0, 20.0]),
        &spec(cmp(gap, Comparator::Gt, constant(0.0))),
    )
    .expect("scan");
    let values = &report.matches[0].values;
    assert_eq!(values.get("sub(price.close,const(5))"), Some(&15.0));
    // The nested close is not repeated: only the expressions the condition names.
    assert!(!values.contains_key("price.close"), "{values:?}");
}

#[test]
fn a_trend_condition_names_the_bar_it_compared_against() {
    let report = scan_batch(
        data(&[10.0, 20.0, 30.0, 40.0]),
        &spec(Condition::Rising {
            expr: close(),
            bars: 2,
        }),
    )
    .expect("scan");
    let values = &report.matches[0].values;
    assert_eq!(values.get("price.close"), Some(&40.0));
    assert_eq!(values.get("prev(price.close,2)"), Some(&20.0));
}
