//! A runnable Rust example: scan a small universe with the native `scan_batch`
//! API and print the report.
//!
//! ```bash
//! cargo run -p wickra-screener-example
//! ```

use std::collections::BTreeMap;

use screener_core::{scan_batch, Candle, ScanSpec};

const SPEC: &str = r#"{
    "universe": ["AAA", "BBB"],
    "condition": {
        "type": "cmp",
        "left": {"kind": "price", "field": "close"},
        "op": "gt",
        "right": {"kind": "const", "value": 10.0}
    }
}"#;

fn candle(close: f64) -> Candle {
    Candle {
        time: 1,
        open: close,
        high: close,
        low: close,
        close,
        volume: 1.0,
    }
}

fn main() {
    let spec: ScanSpec = ScanSpec::from_json(SPEC).expect("valid spec");

    let mut data = BTreeMap::new();
    data.insert("AAA".to_string(), vec![candle(5.0)]);
    data.insert("BBB".to_string(), vec![candle(15.0)]);

    let report = scan_batch(&data, &spec).expect("scan");

    println!("wickra-screener {}", screener_core::version());
    println!(
        "{}",
        serde_json::to_string(&report).expect("serialize report")
    );
    for m in &report.matches {
        println!("  matched: {}", m.symbol);
    }
}
