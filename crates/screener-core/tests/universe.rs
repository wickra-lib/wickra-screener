//! The universe is what the spec asks to be screened.
//!
//! Before this, `ScanSpec.universe` was checked for being non-empty and then
//! never read again: a batch scan folded whatever symbols the caller happened to
//! send, and `scanned` counted them. A screen over a 500-name universe that
//! received 300 names produced the same shape of report as one that received all
//! 500, and a symbol the caller was not asked about was screened anyway.

use std::collections::BTreeMap;

use screener_core::{
    scan_batch, Candle, Comparator, Condition, Error, Expr, PriceField, ScanSpec, Screener,
    SymbolInput,
};

fn candle(close: f64) -> Candle {
    Candle {
        time: 0,
        open: close,
        high: close,
        low: close,
        close,
        volume: 1.0,
    }
}

/// Matches any symbol whose close is above zero, so membership is the only thing
/// deciding what reaches the report.
fn spec(universe: &[&str]) -> ScanSpec {
    ScanSpec {
        universe: universe.iter().map(|s| (*s).to_string()).collect(),
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

fn data(symbols: &[&str]) -> BTreeMap<String, SymbolInput> {
    symbols
        .iter()
        .map(|s| ((*s).to_string(), vec![candle(10.0)].into()))
        .collect()
}

#[test]
fn a_symbol_outside_the_universe_is_not_scanned() {
    let report = scan_batch(data(&["AAA", "BBB", "ZZZ"]), &spec(&["AAA", "BBB"])).unwrap();
    assert_eq!(report.scanned, 2);
    let matched: Vec<&str> = report.matches.iter().map(|m| m.symbol.as_str()).collect();
    assert_eq!(matched, ["AAA", "BBB"]);
    assert!(report.missing.is_empty());
}

#[test]
fn a_universe_symbol_with_no_data_is_reported_missing() {
    let report = scan_batch(data(&["AAA"]), &spec(&["AAA", "BBB", "CCC"])).unwrap();
    assert_eq!(report.scanned, 1);
    assert_eq!(report.missing, ["BBB", "CCC"]);
    assert_eq!(report.matches.len(), 1);
}

#[test]
fn missing_keeps_universe_order_and_drops_repeats() {
    let report = scan_batch(data(&["AAA"]), &spec(&["ZZZ", "AAA", "BBB", "ZZZ"])).unwrap();
    assert_eq!(report.missing, ["ZZZ", "BBB"]);
}

#[test]
fn missing_is_absent_from_the_json_when_empty() {
    let report = scan_batch(data(&["AAA"]), &spec(&["AAA"])).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("missing"), "{json}");
    // And present once something is missing.
    let report = scan_batch(data(&["AAA"]), &spec(&["AAA", "BBB"])).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains(r#""missing":["BBB"]"#), "{json}");
}

#[test]
fn the_timeframe_label_is_echoed_into_the_report() {
    let mut spec = spec(&["AAA"]);
    assert_eq!(
        scan_batch(data(&["AAA"]), &spec).unwrap().timeframe,
        None,
        "a spec without a timeframe reports none"
    );

    spec.timeframe = Some("1h".to_string());
    let report = scan_batch(data(&["AAA"]), &spec).unwrap();
    assert_eq!(report.timeframe.as_deref(), Some("1h"));
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains(r#""timeframe":"1h""#), "{json}");
}

#[test]
fn feeding_a_symbol_outside_the_universe_is_refused() {
    let spec_json = serde_json::to_string(&spec(&["AAA"])).unwrap();
    let mut screener = Screener::new(&spec_json).unwrap();

    screener.feed("AAA", &candle(10.0)).expect("AAA is named");
    let err = screener
        .feed("ZZZ", &candle(10.0))
        .expect_err("ZZZ is not named");
    assert!(
        matches!(&err, Error::NotInUniverse(s) if s == "ZZZ"),
        "{err}"
    );

    // Through the command boundary the refusal comes back in band.
    let cmd = serde_json::json!({
        "cmd": "feed",
        "symbol": "ZZZ",
        "candle": candle(10.0),
    })
    .to_string();
    let out = screener.command_json(&cmd).unwrap();
    assert!(out.contains(r#""ok":false"#), "{out}");
    assert!(out.contains("ZZZ"), "{out}");
}

#[test]
fn streaming_reports_the_symbols_that_never_arrived() {
    let spec_json = serde_json::to_string(&spec(&["AAA", "BBB", "CCC"])).unwrap();
    let mut screener = Screener::new(&spec_json).unwrap();
    screener.feed("AAA", &candle(10.0)).unwrap();

    let report = screener.evaluate();
    assert_eq!(report.scanned, 1);
    assert_eq!(report.missing, ["BBB", "CCC"]);
}

#[test]
fn streaming_and_batch_agree_on_a_partial_universe() {
    let spec = spec(&["AAA", "BBB", "CCC"]);
    let spec_json = serde_json::to_string(&spec).unwrap();
    let mut screener = Screener::new(&spec_json).unwrap();
    for symbol in ["AAA", "BBB"] {
        screener.feed(symbol, &candle(10.0)).unwrap();
    }
    assert_eq!(
        screener.evaluate(),
        scan_batch(data(&["AAA", "BBB"]), &spec).unwrap()
    );
}
