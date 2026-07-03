//! Property-based invariants: for random universes and random condition trees,
//! `scan_batch` never panics and the report obeys the structural contract —
//! matches are a unique subset of the universe, capped by `limit`, byte-identical
//! to the streaming path (parallel fold == sequential fold), and sorted
//! monotonically.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use screener_core::{
    scan_batch, Candle, Comparator, Condition, CsMetric, Expr, PriceField, RankSpec, ScanSpec,
    Screener,
};

fn arb_comparator() -> impl Strategy<Value = Comparator> {
    prop_oneof![
        Just(Comparator::Gt),
        Just(Comparator::Ge),
        Just(Comparator::Lt),
        Just(Comparator::Le),
        Just(Comparator::Eq),
        Just(Comparator::CrossesAbove),
        Just(Comparator::CrossesBelow),
    ]
}

fn arb_price_field() -> impl Strategy<Value = PriceField> {
    prop_oneof![
        Just(PriceField::Open),
        Just(PriceField::High),
        Just(PriceField::Low),
        Just(PriceField::Close),
        Just(PriceField::Volume),
    ]
}

fn arb_cs_metric() -> impl Strategy<Value = CsMetric> {
    prop_oneof![
        Just(CsMetric::Rank),
        Just(CsMetric::PercentileRank),
        Just(CsMetric::ZScore),
    ]
}

/// A scalar indicator that exists in the registry (avoids unknown-indicator
/// errors so the Ok-path invariants get exercised).
fn arb_indicator() -> impl Strategy<Value = Expr> {
    (
        prop_oneof![Just("Sma"), Just("Ema"), Just("Rsi"), Just("Roc")],
        2u32..30u32,
    )
        .prop_map(|(name, period)| Expr::Indicator {
            name: name.to_string(),
            params: vec![f64::from(period)],
            field: None,
        })
}

fn arb_expr() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (-1000.0f64..2000.0).prop_map(|value| Expr::Const { value }),
        arb_price_field().prop_map(|field| Expr::Price { field }),
        arb_indicator(),
    ]
}

/// A bounded condition tree of `cmp`/`cross_section` leaves under `all`/`any`/
/// `not` (breadth is excluded — it is covered by the golden and conformance
/// suites and has a no-nesting rule that would mostly generate invalid specs).
fn arb_condition() -> impl Strategy<Value = Condition> {
    let leaf =
        prop_oneof![
            (arb_expr(), arb_comparator(), arb_expr())
                .prop_map(|(left, op, right)| Condition::Cmp { left, op, right }),
            (
                arb_indicator(),
                arb_cs_metric(),
                arb_comparator(),
                0.0f64..6.0
            )
                .prop_map(|(expr, metric, op, value)| Condition::CrossSection {
                    expr,
                    metric,
                    op,
                    value,
                }),
        ];
    leaf.prop_recursive(3, 12, 3, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 1..3)
                .prop_map(|conditions| Condition::All { conditions }),
            prop::collection::vec(inner.clone(), 1..3)
                .prop_map(|conditions| Condition::Any { conditions }),
            inner.prop_map(|c| Condition::Not {
                condition: Box::new(c),
            }),
        ]
    })
}

fn arb_dataset() -> impl Strategy<Value = BTreeMap<String, Vec<Candle>>> {
    (2usize..6, 20usize..50).prop_flat_map(|(symbols, bars)| {
        prop::collection::vec(prop::collection::vec(1.0f64..1000.0, bars), symbols).prop_map(
            |per_symbol| {
                let mut data = BTreeMap::new();
                for (s, closes) in per_symbol.into_iter().enumerate() {
                    let candles = closes
                        .into_iter()
                        .enumerate()
                        .map(|(t, close)| Candle {
                            time: i64::try_from(t).unwrap(),
                            open: close,
                            high: close,
                            low: close,
                            close,
                            volume: 1000.0,
                        })
                        .collect();
                    data.insert(format!("s{s}"), candles);
                }
                data
            },
        )
    })
}

fn arb_spec(universe: Vec<String>) -> impl Strategy<Value = ScanSpec> {
    (
        arb_condition(),
        prop::option::of((arb_price_field(), any::<bool>())),
        prop::option::of(1usize..8),
    )
        .prop_map(move |(condition, rank, limit)| ScanSpec {
            universe: universe.clone(),
            timeframe: None,
            condition,
            rank: rank.map(|(field, desc)| RankSpec {
                by: Expr::Price { field },
                desc,
            }),
            limit,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn scan_report_obeys_the_contract(
        (data, spec) in arb_dataset().prop_flat_map(|data| {
            let universe: Vec<String> = data.keys().cloned().collect();
            (Just(data), arb_spec(universe))
        })
    ) {
        // A structurally-invalid spec is a well-formed error, not a panic; skip it.
        let Ok(report) = scan_batch(&data, &spec) else { return Ok(()); };

        prop_assert_eq!(report.scanned, data.len());

        if let Some(limit) = spec.limit {
            prop_assert!(report.matches.len() <= limit);
        }

        let universe: BTreeSet<&str> = spec.universe.iter().map(String::as_str).collect();
        let mut seen = BTreeSet::new();
        for m in &report.matches {
            prop_assert!(universe.contains(m.symbol.as_str()));
            prop_assert!(seen.insert(m.symbol.clone()));
        }

        // Streaming (sequential fold) must reproduce the batch (parallel) report.
        let spec_json = serde_json::to_string(&spec).unwrap();
        if let Ok(mut screener) = Screener::new(&spec_json) {
            for (symbol, candles) in &data {
                for candle in candles {
                    screener.feed(symbol, candle).unwrap();
                }
            }
            let streaming = screener.evaluate();
            prop_assert_eq!(
                serde_json::to_string(&streaming).unwrap(),
                serde_json::to_string(&report).unwrap()
            );
        }

        // Monotonic order: by score when ranked (ties broken by symbol), else by
        // symbol ascending.
        match &spec.rank {
            Some(rank) => {
                for w in report.matches.windows(2) {
                    if let (Some(a), Some(b)) = (w[0].score, w[1].score) {
                        if rank.desc {
                            prop_assert!(a >= b);
                        } else {
                            prop_assert!(a <= b);
                        }
                    }
                }
            }
            None => {
                for w in report.matches.windows(2) {
                    prop_assert!(w[0].symbol <= w[1].symbol);
                }
            }
        }
    }
}
