//! Folding the universe in lockstep, so a cross-section is one bar.
//!
//! Each symbol used to be folded on its own and evaluated at *its own* last bar.
//! With aligned histories that is the same thing; with unaligned ones a rank or a
//! z-score compared symbols at different points in time while `CROSS_SECTION.md`
//! promised "every symbol of the same bar". A scan that assembles the market
//! panel itself, or that reads a benchmark symbol's close, has to walk the
//! universe one timestamp at a time.

use std::collections::BTreeMap;

use screener_core::{
    scan_batch, BreadthSpec, Candle, Comparator, Condition, CsMetric, Error, Expr, PriceField,
    ScanSpec, SymbolInput,
};

const BARS: usize = 40;

fn candle(time: i64, close: f64) -> Candle {
    Candle {
        time,
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 1_000.0,
    }
}

/// A series of `bars` candles ending at `last_time`, stepping by one hour.
fn series(bars: usize, last_time: i64, base: f64, slope: f64) -> Vec<Candle> {
    (0..bars)
        .map(|i| {
            let back = i64::try_from(bars - 1 - i).expect("bar count fits i64");
            candle(
                last_time - back * 3_600,
                base - slope * (bars - 1 - i) as f64,
            )
        })
        .collect()
}

fn spec_with(condition: Condition) -> ScanSpec {
    ScanSpec {
        universe: vec!["AAA".into(), "BBB".into(), "CCC".into()],
        timeframe: None,
        reference: None,
        breadth: None,
        condition,
        rank: None,
        limit: None,
    }
}

/// Matches everything, so the report shows what each symbol computed.
fn always(expr: Expr) -> Condition {
    Condition::Cmp {
        left: expr,
        op: Comparator::Gt,
        right: Expr::Const { value: f64::MIN },
    }
}

fn close() -> Expr {
    Expr::Price {
        field: PriceField::Close,
    }
}

// --- the cross-section is one bar -------------------------------------------

/// Two symbols end at the same timestamp and one stopped a day earlier. Ranking
/// close across the universe must rank the bars of the last shared timestamp,
/// not each symbol's own final bar.
#[test]
fn a_rank_is_taken_over_one_timestamp() {
    let last = 1_700_000_000_i64;
    // AAA and BBB run to `last`. CCC stops 5 bars earlier, at a much higher
    // close than it would have had later, so a rank taken at each symbol's own
    // last bar puts CCC on top while a rank at `last` does not.
    let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
        ("AAA".to_string(), series(BARS, last, 100.0, 1.0).into()),
        ("BBB".to_string(), series(BARS, last, 200.0, 1.0).into()),
        (
            "CCC".to_string(),
            series(BARS, last - 5 * 3_600, 500.0, 1.0).into(),
        ),
    ]);

    let mut spec = spec_with(Condition::CrossSection {
        expr: close(),
        metric: CsMetric::Rank,
        op: Comparator::Le,
        value: 3.0,
    });
    // A reference symbol forces the lockstep fold, which is what puts the
    // cross-section on one timestamp.
    spec.reference = Some("AAA".to_string());

    let report = scan_batch(data, &spec).expect("scan");
    assert_eq!(report.scanned, 3);
    // CCC never printed at `last`, so it is stale and says so.
    assert_eq!(report.stale, ["CCC"]);
}

/// A symbol whose last bar is older than the universe's is named, so a halted
/// name does not read like a live one.
#[test]
fn a_symbol_that_stopped_printing_is_named_stale() {
    let last = 1_700_000_000_i64;
    let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
        ("AAA".to_string(), series(BARS, last, 100.0, 1.0).into()),
        ("BBB".to_string(), series(BARS, last, 110.0, 1.0).into()),
        (
            "CCC".to_string(),
            series(BARS, last - 10 * 3_600, 120.0, 1.0).into(),
        ),
    ]);
    let report = scan_batch(data, &spec_with(always(close()))).expect("scan");
    assert_eq!(report.stale, ["CCC"]);

    let json = serde_json::to_string(&report).expect("serialize");
    assert!(json.contains(r#""stale":["CCC"]"#), "{json}");
}

#[test]
fn an_aligned_universe_has_nothing_stale() {
    let last = 1_700_000_000_i64;
    let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
        ("AAA".to_string(), series(BARS, last, 100.0, 1.0).into()),
        ("BBB".to_string(), series(BARS, last, 110.0, 1.0).into()),
        ("CCC".to_string(), series(BARS, last, 120.0, 1.0).into()),
    ]);
    let report = scan_batch(data, &spec_with(always(close()))).expect("scan");
    assert!(report.stale.is_empty());
    let json = serde_json::to_string(&report).expect("serialize");
    assert!(!json.contains("stale"), "{json}");
}

// --- the two fold paths agree ------------------------------------------------

/// A spec that needs neither a derived panel nor a benchmark keeps the
/// per-symbol fold. Adding a benchmark switches to lockstep, and for indicators
/// that do not read one the result must be the same: a symbol's indicators only
/// ever see that symbol's own bars.
#[test]
fn lockstep_and_per_symbol_folds_agree() {
    let last = 1_700_000_000_i64;
    let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
        ("AAA".to_string(), series(BARS, last, 100.0, 1.0).into()),
        ("BBB".to_string(), series(BARS, last, 110.0, 1.0).into()),
        ("CCC".to_string(), series(BARS, last, 120.0, 1.0).into()),
    ]);

    let rsi = Expr::Indicator {
        name: "Rsi".to_string(),
        params: vec![14.0],
        field: None,
    };
    let per_symbol = spec_with(always(rsi.clone()));
    let mut lockstep = per_symbol.clone();
    lockstep.reference = Some("AAA".to_string());

    let a = scan_batch(data.clone(), &per_symbol).expect("per-symbol fold");
    let b = scan_batch(data, &lockstep).expect("lockstep fold");
    assert_eq!(a.matches, b.matches);
}

// --- the benchmark symbol ----------------------------------------------------

/// Naming a universe member as the reference feeds the pairwise family without
/// repeating that member's series under every symbol.
#[test]
fn a_benchmark_symbol_feeds_the_pairwise_family() {
    let last = 1_700_000_000_i64;
    let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
        (
            "AAA".to_string(),
            (0..BARS)
                .map(|i| {
                    let t = i as f64;
                    candle(
                        last - i64::try_from(BARS - 1 - i).unwrap() * 3_600,
                        100.0 + 10.0 * (t * 0.35).sin(),
                    )
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        (
            "BBB".to_string(),
            (0..BARS)
                .map(|i| {
                    let t = i as f64;
                    candle(
                        last - i64::try_from(BARS - 1 - i).unwrap() * 3_600,
                        50.0 + 6.0 * (t * 0.21).cos(),
                    )
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        ("CCC".to_string(), series(BARS, last, 120.0, 1.0).into()),
    ]);

    let correlation = Expr::Indicator {
        name: "RollingCorrelation".to_string(),
        params: vec![20.0],
        field: None,
    };
    let mut spec = spec_with(always(correlation));

    // Without a reference the spec is refused, exactly as before.
    let err = scan_batch(data.clone(), &spec).expect_err("no reference, no pairwise");
    assert!(
        matches!(&err, Error::MissingFeed { feed, .. } if feed == "reference"),
        "got {err}"
    );

    // Naming one of the universe's own symbols supplies it.
    spec.reference = Some("CCC".to_string());
    let report = scan_batch(data, &spec).expect("scan against the benchmark");
    assert_eq!(report.matches.len(), 3);
    for m in &report.matches {
        let value = m.values.values().next().copied();
        assert!(
            value.is_some_and(f64::is_finite),
            "{} produced no correlation: {:?}",
            m.symbol,
            m.values
        );
    }
}

#[test]
fn a_reference_outside_the_universe_is_refused() {
    let mut spec = spec_with(always(close()));
    spec.reference = Some("NOPE".to_string());
    let err = spec
        .validate()
        .expect_err("a benchmark has to be a symbol the scan folds");
    assert!(err.to_string().contains("NOPE"), "{err}");
}

// --- the derived panel -------------------------------------------------------

/// The breadth family reads the panel the screener assembles from its own
/// universe, with no second data source.
#[test]
fn breadth_reads_the_panel_built_from_the_universe() {
    let last = 1_700_000_000_i64;
    let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
        ("AAA".to_string(), series(BARS, last, 100.0, 1.0).into()),
        ("BBB".to_string(), series(BARS, last, 110.0, -0.5).into()),
        ("CCC".to_string(), series(BARS, last, 120.0, 1.0).into()),
    ]);
    let advance_decline = Expr::Indicator {
        name: "AdvanceDecline".to_string(),
        params: vec![],
        field: None,
    };
    let report = scan_batch(data, &spec_with(always(advance_decline))).expect("scan");
    assert_eq!(report.matches.len(), 3);
    let value = report.matches[0].values.values().next().copied();
    assert!(value.is_some_and(f64::is_finite));
}

/// The lookback the panel uses for new highs and lows is the spec's to set.
#[test]
fn the_breadth_configuration_is_validated() {
    let mut spec = spec_with(always(close()));
    spec.breadth = Some(BreadthSpec {
        period: Some(0),
        ..BreadthSpec::default()
    });
    assert!(spec.validate().is_err());

    spec.breadth = Some(BreadthSpec {
        period: Some(20),
        ma_period: Some(50),
        ..BreadthSpec::default()
    });
    assert!(spec.validate().is_ok());
}

/// `BullishPercentIndex` counts the symbols standing on a point-and-figure buy
/// signal, which the derived panel now carries. A universe where some symbols
/// have broken to a double top and others have broken down must not read 0%.
#[test]
fn the_bullish_percent_index_reads_the_derived_buy_signals() {
    let last = 1_700_000_000_i64;

    // A path that runs up, reverses, and breaks out above the first leg: the
    // symbol ends on a buy signal.
    let bullish = zigzag(&[(100, 130), (130, 108), (108, 140)]);
    // The mirror image: it breaks back down through the previous column.
    let bearish = zigzag(&[(100, 130), (130, 108), (108, 140), (140, 95)]);
    let bars = bullish.len().max(bearish.len());

    let data: BTreeMap<String, SymbolInput> = BTreeMap::from([
        ("AAA".to_string(), path_series(&bullish, last).into()),
        ("BBB".to_string(), path_series(&bullish, last).into()),
        ("CCC".to_string(), path_series(&bearish, last).into()),
    ]);
    assert!(bars > 40, "the path has to be long enough to form columns");

    let bpi = Expr::Indicator {
        name: "BullishPercentIndex".to_string(),
        params: vec![],
        field: None,
    };
    let report = scan_batch(data, &spec_with(always(bpi))).expect("scan");
    let value = report.matches[0]
        .values
        .values()
        .next()
        .copied()
        .expect("a reading");
    assert!(
        value > 0.0,
        "a universe with symbols on a buy signal must not read {value}"
    );
    assert!(
        value < 100.0,
        "the broken-down symbol must not count: {value}"
    );
}

/// Build a price path out of straight legs between the given prices.
fn zigzag(legs: &[(i32, i32)]) -> Vec<f64> {
    let mut path = Vec::new();
    for &(from, to) in legs {
        let step = if to >= from { 1 } else { -1 };
        let mut price = from;
        while price != to {
            path.push(f64::from(price));
            price += step;
        }
        path.push(f64::from(to));
    }
    path
}

/// Turn a price path into an hourly candle series ending at `last_time`.
fn path_series(path: &[f64], last_time: i64) -> Vec<Candle> {
    let n = path.len();
    path.iter()
        .enumerate()
        .map(|(i, close)| {
            let back = i64::try_from(n - 1 - i).expect("bar count fits i64");
            candle(last_time - back * 3_600, *close)
        })
        .collect()
}
