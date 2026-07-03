//! Streaming equals batch: feeding the universe candle-by-candle through a
//! `Screener` and calling `evaluate()` must produce the exact same `ScanReport`
//! as `scan_batch` over the whole history. Same core, two drive modes, one
//! byte-identical result — verified over every golden spec.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use screener_core::{scan_batch, Candle, ScanSpec, Screener};

const SPECS: [&str; 5] = [
    "momentum",
    "mean_reversion",
    "cross_section_rank",
    "breadth",
    "crossover",
];

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../golden")
}

fn dataset() -> BTreeMap<String, Vec<Candle>> {
    let json = fs::read_to_string(golden_dir().join("data.json")).expect("read data.json");
    serde_json::from_str(&json).expect("parse data.json")
}

#[test]
fn streaming_equals_batch_over_golden_specs() {
    let data = dataset();
    for name in SPECS {
        let spec_json = fs::read_to_string(golden_dir().join("specs").join(format!("{name}.json")))
            .expect("read spec");
        let spec: ScanSpec = serde_json::from_str(&spec_json).expect("parse spec");

        // Batch: fold every symbol's full history at once.
        let batch = scan_batch(&data, &spec).expect("scan_batch");
        let batch_json = serde_json::to_string(&batch).expect("serialize batch");

        // Streaming: feed candle-by-candle, then evaluate the current universe.
        let mut screener = Screener::new(&spec_json).expect("build screener");
        for (symbol, candles) in &data {
            for candle in candles {
                screener.feed(symbol, candle).expect("feed");
            }
        }
        let streaming = screener.evaluate();
        let streaming_json = serde_json::to_string(&streaming).expect("serialize streaming");

        assert_eq!(
            streaming_json, batch_json,
            "streaming != batch for spec {name}"
        );
    }
}
