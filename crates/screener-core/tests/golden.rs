//! Golden: replaying the committed specs over the committed universe must
//! reproduce `golden/expected/<spec>.json` byte-for-byte. This is the same
//! serialization every language binding returns from `command_json`, so byte
//! equality here is the cross-language contract.
//!
//! Bless (regenerate the expected files) with:
//!
//! ```text
//! cargo test -p screener-core --test golden -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use screener_core::{scan_batch, Candle, ScanSpec};

const SPECS: [&str; 5] = [
    "momentum",
    "mean_reversion",
    "cross_section_rank",
    "breadth",
    "crossover",
];

fn golden_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/screener-core; golden/ lives at the repo root.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../golden")
}

/// The universe loaded from `golden/data.json` (the same f64 values as the CSVs).
fn dataset() -> BTreeMap<String, Vec<Candle>> {
    let json = fs::read_to_string(golden_dir().join("data.json")).expect("read data.json");
    serde_json::from_str(&json).expect("parse data.json")
}

/// The scan report for a spec, serialized exactly as `command_json` returns it.
fn report_json(data: &BTreeMap<String, Vec<Candle>>, name: &str) -> String {
    let spec_json = fs::read_to_string(golden_dir().join("specs").join(format!("{name}.json")))
        .expect("read spec");
    let spec: ScanSpec = serde_json::from_str(&spec_json).expect("parse spec");
    let report = scan_batch(data, &spec).expect("scan");
    serde_json::to_string(&report).expect("serialize report")
}

#[test]
fn golden_reports_match_byte_for_byte() {
    let data = dataset();
    for name in SPECS {
        let got = report_json(&data, name);
        let expected =
            fs::read_to_string(golden_dir().join("expected").join(format!("{name}.json")))
                .unwrap_or_else(|_| {
                    panic!("missing golden/expected/{name}.json — run the bless command")
                });
        assert_eq!(got, expected, "golden mismatch for {name}");
    }
}

/// Bless: overwrite the expected files with the current output. Ignored by
/// default; run with `--ignored` to regenerate, then review the diff and commit.
#[test]
#[ignore = "bless: regenerates golden/expected/*.json"]
fn bless_golden() {
    let data = dataset();
    for name in SPECS {
        let got = report_json(&data, name);
        let path = golden_dir().join("expected").join(format!("{name}.json"));
        fs::write(&path, &got).expect("write expected");
        println!("blessed {}", path.display());
    }
}
