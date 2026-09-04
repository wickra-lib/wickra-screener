#![no_main]
//! Fuzz the full batch scan: a `{spec, data}` object is parsed and scanned. Both
//! the spec and the universe are attacker-controlled, and each symbol may be a
//! bare candle array or a series carrying arbitrary side feeds; the scan must
//! never panic, whichever form arrives.

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use wickra_screener_core::{scan_batch, ScanSpec, SymbolInput};
use serde::Deserialize;

#[derive(Deserialize)]
struct Input {
    spec: ScanSpec,
    data: BTreeMap<String, SymbolInput>,
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(input) = serde_json::from_str::<Input>(text) else {
        return;
    };
    // Bound the total work so the fuzzer cannot request an unbounded scan.
    let bars: usize = input.data.values().map(|s| s.candles().len()).sum();
    if bars > 5000 {
        return;
    }
    let _ = scan_batch(input.data, &input.spec);
});
