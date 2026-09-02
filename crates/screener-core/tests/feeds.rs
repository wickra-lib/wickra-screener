//! Every feed family, end to end.
//!
//! For each of the six families that need something beyond the candle, one
//! representative indicator is scanned twice: once with its feed, where it must
//! produce a value and reach the report, and once without, where the spec must
//! be rejected. The second half is the point of the whole module — before the
//! feeds existed, a scan naming one of these indicators ran to completion and
//! matched nothing, which is indistinguishable from a condition that was simply
//! never true.

use std::collections::BTreeMap;

use screener_core::{
    scan_batch, Candle, Comparator, Condition, CrossSection, DerivativesTick, Error, Expr,
    OrderBook, ScanSpec, SymbolInput, SymbolSeries, TradePrint,
};

const BARS: usize = 60;
const SYMBOL: &str = "AAA";

/// A varying price path. A geometric path has constant log returns, which makes
/// the pairwise correlation family degenerate; this one does not.
fn close_at(index: usize) -> f64 {
    let t = index as f64;
    100.0 + 10.0 * (t * 0.35).sin() + 0.4 * t
}

fn candles() -> Vec<Candle> {
    (0..BARS)
        .map(|i| {
            let close = close_at(i);
            Candle {
                time: time_of(i),
                open: close - 0.5,
                high: close + 1.0,
                low: close - 1.0,
                close,
                volume: 1_000.0 + t_of(i) * 3.0,
            }
        })
        .collect()
}

fn t_of(index: usize) -> f64 {
    index as f64
}

/// The bar timestamp for an index. `usize as i64` is banned by the workspace
/// lints, and a bar index that does not fit an `i64` is not a case worth
/// carrying through a fixture.
fn time_of(index: usize) -> i64 {
    i64::try_from(index).expect("bar index fits i64") * 60_000
}

/// A reference series that moves differently from the symbol, so a correlation
/// or beta over the pair is defined rather than degenerate.
fn reference() -> Vec<Candle> {
    (0..BARS)
        .map(|i| {
            let t = t_of(i);
            let close = 50.0 + 6.0 * (t * 0.21).cos() + 0.2 * t;
            Candle {
                time: time_of(i),
                open: close,
                high: close + 0.5,
                low: close - 0.5,
                close,
                volume: 500.0,
            }
        })
        .collect()
}

fn derivs() -> Vec<DerivativesTick> {
    (0..BARS)
        .map(|i| {
            let t = t_of(i);
            DerivativesTick {
                funding_rate: 0.0001 + 0.00005 * (t * 0.3).sin(),
                mark_price: close_at(i) + 0.2,
                index_price: close_at(i),
                futures_price: close_at(i) + 0.5,
                open_interest: 1_000_000.0 + 5_000.0 * t,
                long_size: 600_000.0,
                short_size: 400_000.0,
                taker_buy_volume: 700.0 + 10.0 * t,
                taker_sell_volume: 500.0 + 5.0 * t,
                long_liquidation: 10.0,
                short_liquidation: 8.0,
                timestamp: time_of(i),
            }
        })
        .collect()
}

fn books() -> Vec<OrderBook> {
    use screener_core::Level;
    (0..BARS)
        .map(|i| {
            let mid = close_at(i);
            OrderBook {
                bids: vec![
                    Level {
                        price: mid - 0.5,
                        size: 12.0,
                    },
                    Level {
                        price: mid - 1.0,
                        size: 20.0,
                    },
                ],
                asks: vec![
                    Level {
                        price: mid + 0.5,
                        size: 9.0,
                    },
                    Level {
                        price: mid + 1.0,
                        size: 18.0,
                    },
                ],
            }
        })
        .collect()
}

fn trades() -> Vec<Vec<TradePrint>> {
    use screener_core::TradeSide;
    (0..BARS)
        .map(|i| {
            let mid = close_at(i);
            vec![
                TradePrint {
                    price: mid + 0.5,
                    size: 3.0,
                    side: TradeSide::Buy,
                    timestamp: time_of(i),
                },
                TradePrint {
                    price: mid - 0.5,
                    size: 2.0,
                    side: TradeSide::Sell,
                    timestamp: time_of(i) + 1,
                },
            ]
        })
        .collect()
}

fn sections() -> Vec<CrossSection> {
    use screener_core::CrossSectionMember;
    (0..BARS)
        .map(|i| {
            let up = i % 3 != 0;
            CrossSection {
                members: (0..8)
                    .map(|m| CrossSectionMember {
                        change: if up && m % 2 == 0 { 1.5 } else { -0.8 },
                        volume: 1_000.0 + f64::from(m) * 10.0,
                        new_high: up && m == 0,
                        new_low: !up && m == 1,
                    })
                    .collect(),
                timestamp: time_of(i),
            }
        })
        .collect()
}

/// A spec that matches whenever the named indicator has any finite value: the
/// question under test is whether it produces one at all.
fn spec_for(name: &str, params: Vec<f64>) -> ScanSpec {
    ScanSpec {
        universe: vec![SYMBOL.to_string()],
        timeframe: None,
        condition: Condition::Cmp {
            left: Expr::Indicator {
                name: name.to_string(),
                params,
                field: None,
            },
            op: Comparator::Gt,
            right: Expr::Const { value: f64::MIN },
        },
        rank: None,
        limit: None,
    }
}

fn dataset(series: SymbolSeries) -> BTreeMap<String, SymbolInput> {
    BTreeMap::from([(SYMBOL.to_string(), SymbolInput::Series(Box::new(series)))])
}

fn candles_only() -> BTreeMap<String, SymbolInput> {
    BTreeMap::from([(SYMBOL.to_string(), candles().into())])
}

/// The indicator produces a value once its feed is supplied.
fn assert_matches_with_feed(name: &str, params: Vec<f64>, series: SymbolSeries) {
    let spec = spec_for(name, params);
    let report = scan_batch(dataset(series), &spec).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(
        report.matches.len(),
        1,
        "{name} produced no value with its feed supplied"
    );
    let value = report.matches[0].values.values().next().copied();
    assert!(
        value.is_some_and(f64::is_finite),
        "{name} reported no finite value: {:?}",
        report.matches[0].values
    );
}

/// The same spec is refused when the feed is absent, naming the feed.
fn assert_rejected_without_feed(name: &str, params: Vec<f64>, feed: &str) {
    let spec = spec_for(name, params);
    let err = scan_batch(candles_only(), &spec)
        .expect_err("a spec whose feed is not supplied must be refused");
    match &err {
        Error::MissingFeed { indicator, feed: f } => {
            assert_eq!(indicator, name);
            assert_eq!(f, feed);
        }
        other => panic!("{name}: expected MissingFeed, got {other}"),
    }
}

fn with(series: impl FnOnce(&mut SymbolSeries)) -> SymbolSeries {
    let mut s = SymbolSeries {
        candles: candles(),
        ..SymbolSeries::default()
    };
    series(&mut s);
    s
}

#[test]
fn pairwise_needs_a_reference_series() {
    assert_matches_with_feed(
        "RollingCorrelation",
        vec![20.0],
        with(|s| s.reference = Some(reference())),
    );
    assert_rejected_without_feed("RollingCorrelation", vec![20.0], "reference");
}

#[test]
fn derivatives_need_a_tick() {
    assert_matches_with_feed("FundingRate", vec![], with(|s| s.derivs = Some(derivs())));
    assert_rejected_without_feed("FundingRate", vec![], "derivs");
}

#[test]
fn order_book_indicators_need_a_book() {
    assert_matches_with_feed("Microprice", vec![], with(|s| s.books = Some(books())));
    assert_rejected_without_feed("Microprice", vec![], "books");
}

#[test]
fn trade_flow_indicators_need_trades() {
    assert_matches_with_feed(
        "CumulativeVolumeDelta",
        vec![],
        with(|s| s.trades = Some(trades())),
    );
    assert_rejected_without_feed("CumulativeVolumeDelta", vec![], "trades");
}

#[test]
fn trade_quote_indicators_need_trades_and_a_book() {
    assert_matches_with_feed(
        "EffectiveSpread",
        vec![],
        with(|s| {
            s.trades = Some(trades());
            s.books = Some(books());
        }),
    );
    // Trades alone are not enough: the family quotes them against the book mid.
    let spec = spec_for("EffectiveSpread", vec![]);
    let err = scan_batch(dataset(with(|s| s.trades = Some(trades()))), &spec)
        .expect_err("EffectiveSpread without a book must be refused");
    assert!(
        matches!(&err, Error::MissingFeed { feed, .. } if feed == "trades and books"),
        "got {err}"
    );
}

#[test]
fn breadth_indicators_need_a_cross_section() {
    assert_matches_with_feed(
        "AdvanceDecline",
        vec![],
        with(|s| s.sections = Some(sections())),
    );
    assert_rejected_without_feed("AdvanceDecline", vec![], "sections");
}

#[test]
fn a_candle_only_indicator_still_needs_nothing() {
    let spec = spec_for("Rsi", vec![14.0]);
    let report = scan_batch(candles_only(), &spec).expect("Rsi scans on candles alone");
    assert_eq!(report.matches.len(), 1);
}

#[test]
fn a_feed_shorter_than_the_candles_is_refused() {
    let spec = spec_for("RollingCorrelation", vec![20.0]);
    let mut short = reference();
    short.truncate(BARS - 1);
    let err = scan_batch(dataset(with(|s| s.reference = Some(short.clone()))), &spec)
        .expect_err("a short feed must be refused");
    match &err {
        Error::FeedLength {
            symbol,
            feed,
            len,
            candles,
        } => {
            assert_eq!(symbol, SYMBOL);
            assert_eq!(feed, "reference");
            assert_eq!(*len, BARS - 1);
            assert_eq!(*candles, BARS);
        }
        other => panic!("expected FeedLength, got {other}"),
    }
}

/// Every name the screener treats as pairwise really does need a reference: it
/// yields nothing on candles alone and a value once the reference arrives.
///
/// This is the half of the drift check that can be written today. The other
/// half — proving no *other* registry name needs a reference — would have to
/// enumerate the catalogue, and `wickra-backtest-core` exposes no such listing.
#[test]
fn every_pairwise_name_behaves_like_one() {
    const PAIRWISE: [(&str, &[f64]); 24] = [
        // Parameters follow each indicator's own constructor: a period, or a
        // period plus its second argument (an ADF lag, a z-score window, a
        // std-dev multiplier, a risk-free rate, a Kalman delta).
        ("Alpha", &[20.0, 0.0]),
        ("Beta", &[20.0]),
        ("BetaNeutralSpread", &[20.0]),
        ("Cointegration", &[30.0, 1.0]),
        ("DistanceSsd", &[20.0]),
        ("GrangerCausality", &[30.0, 2.0]),
        ("HasbrouckInformationShare", &[20.0]),
        ("InformationRatio", &[20.0]),
        ("KalmanHedgeRatio", &[0.0001, 0.001]),
        ("KendallTau", &[20.0]),
        ("LeadLagCrossCorrelation", &[20.0, 3.0]),
        ("OuHalfLife", &[30.0]),
        ("PairSpreadZScore", &[20.0, 20.0]),
        ("PairwiseBeta", &[20.0]),
        ("PearsonCorrelation", &[20.0]),
        ("RelativeStrengthAB", &[20.0, 14.0]),
        ("RollingCorrelation", &[20.0]),
        ("RollingCovariance", &[20.0]),
        ("SpearmanCorrelation", &[20.0]),
        ("SpreadAr1Coefficient", &[20.0]),
        ("SpreadBollingerBands", &[20.0, 2.0]),
        ("SpreadHurst", &[30.0]),
        ("TreynorRatio", &[20.0, 0.0]),
        ("VarianceRatio", &[20.0, 4.0]),
    ];

    for (name, params) in PAIRWISE {
        assert_rejected_without_feed(name, params.to_vec(), "reference");
        assert_matches_with_feed(
            name,
            params.to_vec(),
            with(|s| s.reference = Some(reference())),
        );
    }
}
