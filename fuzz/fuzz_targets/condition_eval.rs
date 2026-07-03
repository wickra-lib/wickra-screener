#![no_main]
//! Fuzz the condition/indicator evaluation: an arbitrary spec (any condition
//! tree, any indicator name/params) is scanned over a fixed, bounded universe.
//! No spec — however adversarial — may panic; an unresolvable one is a clean
//! `Err`.

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use screener_core::{scan_batch, Candle, ScanSpec};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(spec) = ScanSpec::from_json(text) else {
        return;
    };
    let mut universe = BTreeMap::new();
    for sym in ["s0", "s1", "s2"] {
        let candles: Vec<Candle> = (0i64..40)
            .map(|i| {
                let c = 100.0 + i as f64;
                Candle {
                    time: i,
                    open: c,
                    high: c,
                    low: c,
                    close: c,
                    volume: 1000.0,
                }
            })
            .collect();
        universe.insert(sym.to_string(), candles);
    }
    let _ = scan_batch(&universe, &spec);
});
