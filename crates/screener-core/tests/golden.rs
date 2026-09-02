//! Golden: replaying the committed specs over the committed universes must
//! reproduce `golden/expected/<spec>.json` byte-for-byte. This is the same
//! serialization every language binding returns from `command_json`, so byte
//! equality here is the cross-language contract.
//!
//! There are two committed universes. `data.json` is candles only; a spec named
//! `feeds_*` scans `data-feeds.json` instead, which carries the same candles plus
//! a reference series, derivatives ticks, order books, trades and market panels.
//! That one rule is all a binding needs to pick the right dataset, which is why
//! it is a file-name convention rather than a manifest each language has to
//! parse.
//!
//! Bless (regenerate the expected files) with:
//!
//! ```text
//! cargo test -p screener-core --test golden -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use screener_core::{scan_batch, ScanSpec, SymbolInput};

const SPECS: [&str; 12] = [
    "momentum",
    "mean_reversion",
    "cross_section_rank",
    "breadth",
    "crossover",
    "compound",
    "derived_breadth",
    "feeds_pairwise",
    "feeds_derivatives",
    "feeds_orderbook",
    "feeds_trades",
    "feeds_breadth",
];

/// The dataset a spec scans: the fed universe for a `feeds_*` spec, the
/// candle-only one otherwise.
fn dataset_for(name: &str) -> &'static str {
    if name.starts_with("feeds_") {
        "data-feeds.json"
    } else {
        "data.json"
    }
}

fn golden_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/screener-core; golden/ lives at the repo root.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../golden")
}

/// A committed universe, parsed once per file.
fn dataset(file: &str) -> BTreeMap<String, SymbolInput> {
    let json =
        fs::read_to_string(golden_dir().join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse {file}: {e}"))
}

/// The scan report for a spec, serialized exactly as `command_json` returns it.
fn report_json(name: &str) -> String {
    let spec_json = fs::read_to_string(golden_dir().join("specs").join(format!("{name}.json")))
        .expect("read spec");
    let spec: ScanSpec = serde_json::from_str(&spec_json).expect("parse spec");
    let report = scan_batch(dataset(dataset_for(name)), &spec)
        .unwrap_or_else(|e| panic!("scan {name}: {e}"));
    serde_json::to_string(&report).expect("serialize report")
}

#[test]
fn golden_reports_match_byte_for_byte() {
    for name in SPECS {
        let got = report_json(name);
        let expected =
            fs::read_to_string(golden_dir().join("expected").join(format!("{name}.json")))
                .unwrap_or_else(|_| {
                    panic!("missing golden/expected/{name}.json — run the bless command")
                });
        assert_eq!(got, expected, "golden mismatch for {name}");
    }
}

/// Every committed spec is in `SPECS`. A spec added to the directory and not
/// listed here would be scanned by the bindings, which glob the directory, and
/// silently not by the core.
#[test]
fn every_committed_spec_is_covered() {
    let mut on_disk: Vec<String> = fs::read_dir(golden_dir().join("specs"))
        .expect("read specs")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then(|| path.file_stem()?.to_str().map(str::to_string))?
        })
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = SPECS.iter().map(|s| (*s).to_string()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}

/// Every match in a fed report carries a finite value for each indicator the
/// spec names. A feed that silently produced nothing would leave the report
/// empty, which a byte comparison against a blessed-empty file would happily
/// accept.
#[test]
fn the_fed_reports_are_not_empty() {
    for name in SPECS.iter().filter(|n| n.starts_with("feeds_")) {
        let json = report_json(name);
        let report: serde_json::Value = serde_json::from_str(&json).expect("parse report");
        let matches = report["matches"].as_array().expect("matches array");
        assert!(!matches.is_empty(), "{name} matched nothing");
        for entry in matches {
            let values = entry["values"].as_object().expect("values object");
            assert!(!values.is_empty(), "{name}: a match explains nothing");
            for (key, value) in values {
                assert!(
                    value.as_f64().is_some_and(f64::is_finite),
                    "{name}: {key} is not a finite value"
                );
            }
        }
    }
}

/// Bless: overwrite the expected files with the current output. Ignored by
/// default; run with `--ignored` to regenerate, then review the diff and commit.
#[test]
#[ignore = "bless: regenerates golden/expected/*.json"]
fn bless_golden() {
    for name in SPECS {
        let got = report_json(name);
        let path = golden_dir().join("expected").join(format!("{name}.json"));
        fs::write(&path, &got).expect("write expected");
        println!("blessed {}", path.display());
    }
}

/// Supplying side feeds must not move an indicator that does not read them.
///
/// The two committed universes carry the same candles, so a spec built only from
/// candle-driven indicators has to produce the identical report against both. If
/// a feed ever leaked into the candle path — a book changing an `Rsi` — this is
/// where it would show, and a byte comparison of two blessed files could not see
/// it because both would have moved together.
#[test]
fn feeds_do_not_change_a_candle_only_scan() {
    for name in ["momentum", "mean_reversion", "crossover", "compound"] {
        let spec_json = fs::read_to_string(golden_dir().join("specs").join(format!("{name}.json")))
            .expect("read spec");
        let spec: ScanSpec = serde_json::from_str(&spec_json).expect("parse spec");

        let plain = scan_batch(dataset("data.json"), &spec).expect("candle-only scan");
        let fed = scan_batch(dataset("data-feeds.json"), &spec).expect("fed scan");
        assert_eq!(
            serde_json::to_string(&plain).unwrap(),
            serde_json::to_string(&fed).unwrap(),
            "{name} moved when side feeds were supplied"
        );
    }
}
