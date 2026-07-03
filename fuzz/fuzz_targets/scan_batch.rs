#![no_main]
//! Fuzz the full batch scan: a `{spec, data}` object is parsed and scanned. Both
//! the spec and the universe are attacker-controlled; the scan must never panic.

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use screener_core::{scan_batch, Candle, ScanSpec};
use serde::Deserialize;

#[derive(Deserialize)]
struct Input {
    spec: ScanSpec,
    data: BTreeMap<String, Vec<Candle>>,
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(input) = serde_json::from_str::<Input>(text) else {
        return;
    };
    // Bound the total work so the fuzzer cannot request an unbounded scan.
    let bars: usize = input.data.values().map(Vec::len).sum();
    if bars > 5000 {
        return;
    }
    let _ = scan_batch(&input.data, &input.spec);
});
