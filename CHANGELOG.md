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

### Added

- Side feeds: a scan can now supply the reference series, derivatives ticks,
  order-book snapshots, trades and market cross-sections that a large part of the
  indicator registry consumes. A batch scan passes them as parallel arrays beside
  a symbol's candles (`{"AAA": {"candles": [...], "books": [...]}}`), a streaming
  caller passes them per bar under `feeds`, and a symbol needing no feed keeps the
  bare `{"AAA": [candle, ...]}` form. The shapes mirror `wickra-backtest`'s
  `RunRequest` and `StepFeeds`, so the same document describes a bar in either
  tool. `Screener::feed_step` is the Rust entry point; every binding gets it
  through the existing JSON `command` boundary unchanged.
- A batch scan assembles the market cross-section from its own universe, so the
  breadth indicator family (`AdvanceDecline`, `McClellanOscillator`, `Trin`,
  `NewHighsNewLows`, `PercentAboveMa`, …) needs no second data source. Each
  symbol contributes a member built from its own bar — change against the
  previous close, bar volume, new high or low over `breadth.period`, and whether
  the close is above an `Sma(breadth.ma_period)`. An explicit `sections` feed
  still wins. A streaming screener sees one symbol at a time and cannot derive
  the panel, so there the feed is still required.
- `ScanSpec.reference` names a universe symbol as the benchmark whose close feeds
  every other symbol's pairwise indicators, read at the same bar — instead of
  repeating that symbol's series under every entry in the dataset.
- `ScanSpec.breadth` configures the derived panel: `period` for the new-high and
  new-low lookback (default 52) and `ma_period` for `above_ma` (default 200).
- `ScanReport.stale` names symbols whose most recent bar is older than the last
  bar in the universe, so a halted or delisted name does not read like a live one.
- `ScanSpec::validate` is public, so a caller can check a spec before scanning.
- `ScanReport.missing` names the universe symbols a scan received no data for,
  and `ScanReport.timeframe` echoes the spec's timeframe label so a report says
  which bars it describes. Both are omitted from the JSON when empty. The CLI's
  text output names the missing symbols after the match count.
- `screener_core::feed_kind` reports which feed an indicator consumes, and
  `ScanSpec::required_feeds` reports what a spec needs before a dataset is
  assembled.
- A `feed_payload` fuzz target over the streaming command envelope, covering the
  feed conversions an attacker-controlled document reaches.

### Fixed

- A scan that assembles the cross-section itself, or that names a benchmark
  symbol, folds the universe one timestamp at a time instead of symbol by symbol.
  Each symbol used to be folded independently and evaluated at *its own* last bar,
  so a rank or a z-score could compare symbols at different points in time while
  the documentation promised "every symbol of the same bar". The timeline is the
  union of the universe's bar timestamps, not the intersection, so one halted
  symbol does not rewind the scan for the rest. Per-symbol matching is unchanged:
  a symbol's indicators only ever see that symbol's own bars.
- `ScanSpec.universe` is enforced. It was validated for being non-empty and then
  never read again: a batch scan folded whatever symbols the caller sent and
  `scanned` counted them, so a symbol outside the universe was screened anyway
  and one inside it that never arrived left no trace. A scan now folds exactly the
  universe, `scanned` counts the universe symbols it actually folded, and the ones
  no data arrived for are named in `ScanReport.missing`. Feeding a symbol the
  universe does not name is refused.
- An indicator whose feed was not supplied used to resolve, tick and return
  nothing on every bar, so a screen naming one ran to completion and matched
  nothing — indistinguishable from a condition that was simply never true. That
  covered the pairwise, derivatives, order-book, trade-flow, trade-quote and
  market-breadth families. Those indicators now work when their feed is supplied,
  and a spec whose feed the scan cannot supply is refused, naming the feed.
- A side feed that is not exactly as long as the candle array is refused, and a
  malformed book, tick, trade or cross-section is reported rather than dropped.

### Changed

- `wickra-backtest-core` is consumed from crates.io (0.1.2) instead of git. It
  carries the indicator registry the screener resolves names through; taking it
  from the registry removes a git dependency from the published dependency graph
  and picks up `registry::feed_of` and the `StepFeeds` per-bar feed document.
- `wickra-core` and `wickra-data` move from `0.9` to `1.0`, the major
  `wickra-backtest-core` 0.1.2 resolves, so a default build links one copy of the
  indicator types rather than two incompatible ones.

[Unreleased]: https://github.com/wickra-lib/wickra-screener/commits/main
