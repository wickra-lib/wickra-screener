# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `screener-core`: the data-driven scan engine — a serde `ScanSpec` (expressions,
  the `cmp` / `cross_section` / `breadth` / `all` / `any` / `not` condition tree,
  rank + limit) folded over each symbol's history against the Wickra library of
  514 O(1) streaming indicators. Batch (`scan_batch`) and streaming
  (`feed` / `evaluate`) paths that produce a byte-identical report, parallel
  (rayon) or sequential (the WASM fallback).
- `wickra-screener` CLI: run a spec over a directory of CSV candle files or a
  JSON dataset on stdin, with text or JSON output.
- Language bindings exposing the same JSON-over-C-ABI data API in ten languages —
  native Rust, Python (PyO3), Node.js (napi) and WASM (wasm-bindgen), plus a C ABI
  hub for C, C++, C#, Go, Java and R.
- Byte-exact golden corpus, conformance / streaming-equivalence / property tests,
  cargo-fuzz targets, criterion benchmarks, and one runnable example per language.
- CI across all ten languages on three OSes, CodeQL, OpenSSF Scorecard, zizmor
  workflow auditing, a tag-triggered release pipeline, and the `docs/` guides.
- Repository scaffolding: Cargo workspace, supply-chain configuration
  (`deny.toml`, `osv-scanner.toml`, `lychee.toml`), lint configuration
  (`clippy.toml`), `repo-metadata.toml`, and dual `MIT OR Apache-2.0` licensing.

### Changed

- `wickra-backtest-core` is consumed from crates.io (0.1.2) instead of git. It
  carries the indicator registry the screener resolves names through; taking it
  from the registry removes a git dependency from the published dependency graph
  and picks up `registry::feed_of` and the `StepFeeds` per-bar feed document.
- `wickra-core` and `wickra-data` move from `0.9` to `1.0`, the major
  `wickra-backtest-core` 0.1.2 resolves, so a default build links one copy of the
  indicator types rather than two incompatible ones.

[Unreleased]: https://github.com/wickra-lib/wickra-screener/commits/main
