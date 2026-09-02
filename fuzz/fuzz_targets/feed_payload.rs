#![no_main]
//! Fuzz the streaming feed payload: an arbitrary `feed` command, including the
//! side feeds a bar may carry, is driven through the JSON boundary every binding
//! uses.
//!
//! The feeds are converted on the way in — an order book is checked for the
//! level and ordering invariants, a trade for a finite price and size, a
//! cross-section for its member invariants — so this target covers the
//! conversion path an attacker-controlled document reaches. A malformed feed is
//! an in-band error; nothing here may panic.

use libfuzzer_sys::fuzz_target;
use screener_core::Screener;

/// A spec naming one indicator per feed family, so a fed bar is checked against
/// something that actually consumes it.
const SPEC: &str = r#"{
  "universe": ["s0"],
  "condition": {"type":"any","conditions":[
    {"type":"cmp","left":{"kind":"indicator","name":"Microprice","params":[]},
     "op":"gt","right":{"kind":"const","value":0}},
    {"type":"cmp","left":{"kind":"indicator","name":"CumulativeVolumeDelta","params":[]},
     "op":"gt","right":{"kind":"const","value":0}},
    {"type":"cmp","left":{"kind":"indicator","name":"FundingRate","params":[]},
     "op":"gt","right":{"kind":"const","value":0}},
    {"type":"cmp","left":{"kind":"indicator","name":"AdvanceDecline","params":[]},
     "op":"gt","right":{"kind":"const","value":0}},
    {"type":"cmp","left":{"kind":"indicator","name":"Beta","params":[5]},
     "op":"gt","right":{"kind":"const","value":0}}
  ]}
}"#;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // Bound the work: a single command, not an unbounded document.
    if text.len() > 64 * 1024 {
        return;
    }
    let Ok(mut screener) = Screener::new(SPEC) else {
        return;
    };
    // The fuzzer supplies the whole command envelope, so it controls the candle
    // and every feed field alike.
    let _ = screener.command_json(text);
});
