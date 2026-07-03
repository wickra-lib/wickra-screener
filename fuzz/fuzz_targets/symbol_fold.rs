#![no_main]
//! Fuzz the streaming symbol fold: an arbitrary candle sequence is fed into a
//! screener and evaluated. No sequence — however adversarial — may panic.

use libfuzzer_sys::fuzz_target;
use screener_core::{Candle, Screener};

const SPEC: &str = r#"{"universe":["s0"],"condition":{"type":"cmp",
"left":{"kind":"indicator","name":"Rsi","params":[14]},"op":"gt",
"right":{"kind":"const","value":50}}}"#;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(candles) = serde_json::from_str::<Vec<Candle>>(text) else {
        return;
    };
    if candles.len() > 5000 {
        return;
    }
    let Ok(mut screener) = Screener::new(SPEC) else {
        return;
    };
    for candle in &candles {
        let _ = screener.feed("s0", candle);
    }
    let _ = screener.evaluate();
});
