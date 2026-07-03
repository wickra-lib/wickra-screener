#![no_main]
//! Fuzz the spec-parsing path: arbitrary bytes are parsed as a scan spec (JSON
//! and TOML) and as a config. None must panic; malformed input must surface as a
//! clean `Err`.

use libfuzzer_sys::fuzz_target;
use screener_core::{Config, ScanSpec};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = ScanSpec::from_json(text);
    let _ = ScanSpec::from_toml(text);
    let _ = Config::from_json(text);
    let _ = Config::from_toml(text);
});
