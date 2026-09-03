# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-09-03

### Added

- `screener-core`: the data-driven scan engine — a serde `ScanSpec` (expressions,
  the `cmp` / `cross_section` / `breadth` / `all` / `any` / `not` condition tree,
  rank + limit) folded over each symbol's history against the Wickra library of
  497 O(1) streaming indicators. Batch (`scan_batch`) and streaming
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
  new-low lookback (default 52), `ma_period` for `above_ma` (default 200), and
  `pnf_box` / `pnf_reversal` for the point-and-figure box and reversal behind
  `on_buy_signal` (default one percent of the symbol's first close, three boxes).
- The derived panel carries `on_buy_signal`, so `BullishPercentIndex` reads a real
  count. `PointAndFigureBars` decides where a column ends and the breakout is the
  close against the last completed column of that direction: a double-top
  breakout turns the signal on, a double-bottom breakdown takes it away.
- `ScanReport.stale` names symbols whose most recent bar is older than the last
  bar in the universe, so a halted or delisted name does not read like a live one.
- `ScanSpec::validate` is public, so a caller can check a spec before scanning.
- The expression grammar gains the compound forms `prev` (the value an expression
  had n bars ago) and `add` / `sub` / `mul` / `div`, mirroring
  `wickra_backtest_core::spec::OperandExpr`. A screen can now name the quantity it
  cares about — the gap between price and its average, a ratio against an earlier
  bar — instead of only the raw series. Each symbol keeps exactly as many bars as
  the deepest lookback in the spec.
- `PriceField` gains `hlc3` and `ohlc4`.
- The `ne` comparator, and the `between`, `rising` and `falling` conditions,
  completing the set `wickra_backtest_core::spec::Condition` already had.
- Six golden specs covering the side feeds — `feeds_pairwise`,
  `feeds_derivatives`, `feeds_orderbook`, `feeds_trades`, `feeds_breadth` and
  `derived_breadth` — over a second committed universe, `data-feeds.json`, which
  carries the same candles plus a reference series, derivatives ticks, order
  books, trades and market panels. A spec named `feeds_*` scans it; every other
  spec scans the candle-only `data.json`. Two invariants guard the corpus beyond
  byte equality: every match in a fed report carries a finite value for each
  indicator its spec names, and a candle-only spec produces the identical report
  against both datasets.
- A `compound` golden spec exercising every new form. Every binding globs the
  spec directory, so it is covered in all ten languages without a per-binding
  change.
- `ScanReport.missing` names the universe symbols a scan received no data for,
  and `ScanReport.timeframe` echoes the spec's timeframe label so a report says
  which bars it describes. Both are omitted from the JSON when empty. The CLI's
  text output names the missing symbols after the match count.
- `screener_core::feed_kind` reports which feed an indicator consumes, and
  `ScanSpec::required_feeds` reports what a spec needs before a dataset is
  assembled.
- A `feed_payload` fuzz target over the streaming command envelope, covering the
  feed conversions an attacker-controlled document reaches.

### Added

- A C++ wrapper, `bindings/c/include/wickra_screener.hpp`: header-only, C++17,
  owning the handle, doing the two-call length protocol behind
  `wickra_screener_command`, and turning a negative return into an exception. The
  C++ example is written against it, so CMake and ctest build and run it.
- `docs.rs` metadata on both published crates, and a copy of each licence text
  inside them — a crates.io package carries only what is under its own
  directory, so the root copies never travelled with it.

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

- The `live` feature is implemented. `LiveUniverse::connect(venue, interval,
  bars, options)` opens a public market-data connection to one of the ten venues
  the exchange facade supports, and `fetch(symbols)` pulls each symbol's recent
  candles into the same dataset `scan_batch` takes — so a live scan and a scan
  over committed CSVs run through identical code. The CLI grows `--live <venue>`
  alongside `--data` and `--stdin`, with `--interval` and `--bars`.
  Read-only throughout: no key is sent, no order is placed, and only candles are
  available, so a spec naming an order-book, trade-flow or derivatives indicator
  is refused against a live universe by the same feed check that refuses it
  against a file.

### Added

- An `examples` CI job that builds and runs every runnable example and checks
  each one's output. Only `examples/rust` was ever compiled — it is a workspace
  member, so cargo built it; the other seven were built by nothing, so a change
  to the JSON surface would have left them broken in the repository with every
  check green.

### Fixed

- The release workflow no longer publishes from a branch. A `workflow_dispatch`
  can be started from any ref, and every job in it pushes to a registry that does
  not take things back, so a dispatch from `main` would have published whatever
  `main` was at the time. A guard job refuses anything but a `v*` tag.
- `github-release` waits on every job that produces something it attaches.
  `csharp-publish`, `java-publish` and `go-mirror` were missing, so a run in
  which one of them failed still published a release page that read as complete.
- The NuGet package and the jar reach the release page. Both were published to
  their registries and neither was uploaded as an artefact, so the `find` that
  was supposed to collect the `.nupkg` matched nothing — silently, because a
  `find` that matches nothing succeeds.
- Build provenance covers the NuGet package, the jar and the C ABI archives, not
  only the crates and wheels.
- `go-mirror` builds, vets and tests the assembled module before pushing it.
  Three steps copied files into a tree and the fourth published it, so a mirror
  that does not compile became `go get`'s problem rather than the workflow's.
- The six npm platform packages carry the licence texts their `license` field
  names. They declared `MIT OR Apache-2.0` and shipped neither text.

### Security

- `js-yaml` moves from 4.3.0 to 4.3.2 (GHSA-5p4m-2wfm-xmqj, CVSS 7.5), a
  development dependency of `@napi-rs/cli` in the Node binding. Found by the new
  `osv-scanner` CI job on its first run; nothing else in CI could see it, because
  cargo-deny reads RustSec and does not look at npm manifests.

### Changed

- `wickra-backtest-core` is consumed from crates.io (0.1.2) instead of git. It
  carries the indicator registry the screener resolves names through; taking it
  from the registry removes a git dependency from the published dependency graph
  and picks up `registry::feed_of` and the `StepFeeds` per-bar feed document.
- `wickra-core` and `wickra-data` move from `0.9` to `1.0`, the major
  `wickra-backtest-core` 0.1.2 resolves, so a default build links one copy of the
  indicator types rather than two incompatible ones.
- The `wickra-exchange` git dependency is pinned to a revision, so a build is
  reproducible rather than resolving to whatever the default branch is at the
  time.

### Added

- Four static checks, run by a `binding-surface` CI job. Each guards something no
  other job can fail on: `check_binding_surface.py` holds all eight language
  reaches to the exports in `bindings/c/include/wickra_screener.h`, because the
  golden corpus compares values and a binding that lost a method has no test to
  run; `check_version_sync.py` asserts that all 23 version declarations across
  six package managers agree, because a bump that misses one ships a package
  pinning a binary that was never published; `check_readme_links.py` keeps the
  binding READMEs free of repository-relative links, which resolve on GitHub and
  are dead on the registry pages that render them; and `check_license_copies.py`
  keeps the committed licence texts beside the packages that ship them.

### Fixed

- The status claims match the repository. `ROADMAP.md` called phase 0 *in
  progress* and left phases 1 to 8 statusless while the README called the project
  functionally complete; all nine have landed and now say so. Two places still
  described this repository as screening over 514 indicators — the banner alt
  text and the entry above — where the registry reaches 497; the 514 references
  to the `wickra` library itself and to the sibling projects are correct and
  stay.
- `ROADMAP.md` no longer says the non-crates.io artefacts are unconstrained by
  the `wickra-exchange` git dependency. They are, indirectly: `github-release`
  needs `cargo-publish`, so a tag pushed while that crate is unpublished produces
  no release page and no assets at all. The section now shows the failing
  `cargo publish --dry-run` and names both ways out.
- The Python wheel and sdist carry `LICENSE-MIT` and `LICENSE-APACHE`. The
  manifest declared `MIT OR Apache-2.0` and shipped neither text, so maturin —
  which picks up any `LICEN[CS]E*` beside the manifest — had nothing to pick up,
  and the package named terms the recipient had to go and find.
- The Node binding records why CodeQL raises `rust/access-invalid-pointer`
  against its `Screener` struct. The file writes no unsafe and no raw pointer;
  the anchor is the napi-derive expansion, where the run-time type-tag check
  sits two lines before the dereference and CodeQL cannot follow that invariant
  across the FFI boundary. Kept in the source, so the reasoning outlives any one
  alert dismissal.
- CI installs its Python tooling from hash-locked requirement files. A
  hash-locked `ci-dev.txt` was committed and referenced by no workflow at all;
  the jobs ran `pip install maturin pytest` and `pip install --upgrade pip
  maturin`, unpinned, straight from PyPI. The pinning was there to be read, not
  to be used.
- The Python lock is split by interpreter version, because one resolution cannot
  serve the matrix: `pytest` 9.x and `iniconfig` 2.3 both declare
  `requires-python >= 3.10`, while the matrix still runs 3.9. The single
  `ci-dev.txt` pinned `pytest==9.1.1` and could not have installed on the 3.9
  row — invisible precisely because nothing installed it. `ci-dev-py39.txt` now
  resolves to `pytest==8.4.2`, and `ci-dev-py3.txt` keeps 9.1.1.
- `scripts/update-lockfiles.sh` regenerates every committed lockfile. The
  requirement files named it before it existed. uv is required rather than
  fetched: bootstrapping is opt-in behind `WICKRA_BOOTSTRAP_UV=1` and verifies a
  pinned release archive against a recorded checksum, instead of piping an
  installer URL into a shell.
- The link check reads every markdown file in the tree. Its globs were `*.md`
  and `bindings/*/README.md`, which reached 12 of 36 tracked markdown files —
  `docs/` (7 guides), the issue templates, `examples/`, `golden/` and
  `bindings/csharp/WickraScreener/README.md` were never checked.
- `actionlint` moves from 1.7.7 to 1.7.12, checksum taken from the release's own
  `checksums.txt`.

### Security

- `osv-scanner.toml` records the assessment of GHSA-6w46-j5rx-g56g (pytest tmpdir
  handling, fixed in 9.0.3). Pinning the 3.9 row at `pytest==8.4.2` puts it
  inside the advisory's range; `ci-dev-py3.txt` is on 9.1.1 and is outside it.
  pytest is CI-only and never shipped, and the flaw is local — it needs a second
  user on the machine, which an ephemeral single-user runner does not have.
  Taking the fixed version on 3.9 would mean dropping Python 3.9 from the
  support matrix, which is a different decision.

### Added

- A non-blocking `links` job in `ci.yml`. `links.yml` stays authoritative and
  weekly, since external link rot is non-deterministic and must not be able to
  fail a pull request; but a weekly run surfaces a link this PR breaks up to
  seven days later, when it reads as unrelated. The advisory job shows it now and
  cannot stop anything.

### Added

- A `codspeed` workflow measuring the benches on every pull request and
  commenting the result. `bench.yml` runs nightly, uploads a criterion artifact
  and prints numbers a person has to go and read, so a scan that got slower lands
  and is noticed weeks later or not at all. CodSpeed counts instructions under
  instrumentation rather than wall clock, which is what makes a shared runner
  usable. The workspace `criterion` is now an alias for
  `codspeed-criterion-compat` 5.0.1 — without it the job runs, reports nothing
  and passes. The bench source is unchanged.
- A `python-wheel-container-smoke` job building the wheel in the manylinux and
  musllinux containers on every push. That build happened only in `release.yml`,
  on a `v*` tag, where a failure is found at the one moment nothing can be taken
  back; the `python` job builds on the runner itself and cannot see a
  container-only break.

### Fixed

- Each published package ships its own README. `release.yml` copied the root
  README over `bindings/python/README.md` (for the wheel and again for the sdist)
  and over `bindings/node/README.md` before packing. Those are purpose-written
  62- and 61-line files linking absolutely; the root README is 322 lines with 19
  repository-relative links, kept relative by design because it is read on
  GitHub. PyPI and npm render the packaged README as the description, so both
  would have shipped 19 dead links — as the published `wickra` package on PyPI
  does today, from the same three lines in that repository.
- `check_readme_links.py` also fails when a workflow copies the root README over
  a binding one. Checking the file in the tree proves nothing if the release
  replaces it moments before packing.

### Fixed

- The C ABI runs a command once per delivered response, not once per call.
  `wickra_screener_command` executed `command_json` on every invocation,
  including the documented `out = NULL, cap = 0` length query and the retry after
  a too-small buffer. Every reach behind the hub asks for the length first and
  reads second, so **each `feed` applied its candle twice** in Go, C, C++, C#,
  Java and R — every indicator saw a doubled history. The native bindings
  (Python, Node, WASM) call the core directly and were never affected. A
  response that is produced but not written is now held until the call that
  reads it, and that call returns it without re-running the command.

  Nothing could have caught this before: the golden corpus only ever sends
  `{"cmd":"scan"}`, which is a pure function of its payload and therefore reads
  the same however many times it runs.

### Added

- Streaming-equals-batch tests in every binding: C, C++, C#, Go, Java, Node,
  Python, R and WASM. Each feeds the committed universe candle by candle through
  the JSON command boundary, evaluates, and asserts the result is byte-identical
  to a single `scan` over the same data. `screener-core` proved this in Rust, but
  that says nothing about the boundary each language actually crosses — and it
  is what found the double-execution above. Each also checks that `reset` returns
  a screener to its pre-feed state.
- The WASM binding has tests. The `wasm` job built it on every push and executed
  it on none of them, while the README advertises a live in-browser demo and the
  core documents the sequential WASM path as byte-identical to the parallel one.
  A `--target nodejs` build makes the module loadable without a browser; `pkg/`
  stays what ships.
- Golden parity from C, over the whole committed corpus. C has no portable
  directory API, so the spec list is globbed by CMake and written into a
  generated header — which keeps the property the other bindings get from a
  runtime glob: a spec added to the corpus is covered without editing the test.
- Three C ABI unit tests pinning the single-execution contract: the two-call
  idiom, the truncation retry, and a different command abandoning a queued
  response. Each was confirmed to fail with the fix removed.
### Changed

- The CLI reads its `<SYMBOL>.csv` universe through `wickra-data`, the
  ecosystem's own OHLCV reader, and the dependency moved to the crate that
  actually loads candles. `screener-core` declared it as "Candle + CSV loading
  for the universe data" and no Rust file referenced it; the CLI hand-rolled its
  own parser in a different crate.

  **This changes the accepted CSV format.** The header must now name `timestamp`,
  `open`, `high`, `low`, `close` and `volume`. Columns are matched by name, so
  their order does not matter and extra columns are ignored — the old parser read
  the first six positionally and accepted any header, or none, so a file whose
  columns were in a different order was screened as if they were not. A leading
  UTF-8 byte-order mark, which spreadsheet exports add, is stripped, and a bar
  whose values are not finite or whose high sits below its low is now rejected
  rather than screened. The committed fixtures move from `ts` to `timestamp`, and
  README and `golden/README.md` state the format, which neither did before.

  All six candle-only golden specs read from CSV produce byte-identical reports
  to `golden/expected/*.json`, so no value moved.

### Changed

- Dependabot manages `.github/requirements/` explicitly, with version-update
  pull requests disabled. It edits the hash-locked `.txt` directly and does not
  know which interpreter each lock was resolved against, so a bump lands a
  version the row cannot install — `ci-dev-py39.txt` exists precisely because
  pytest 9 and iniconfig 2.3 require Python 3.10. Regeneration goes through
  `scripts/update-lockfiles.sh`. Security updates stay exempt and still arrive,
  which is the intent: they are judged one at a time. The first, pytest
  GHSA-6w46-j5rx-g56g, proposed 9.0.3 for both files — a downgrade for
  `ci-dev-py3.txt`, already on 9.1.1, and an impossible install for the 3.9 row.
- The pinned uv release in `scripts/update-lockfiles.sh` moves from 0.12.7 to
  0.12.9, with all four checksums read from each archive's own `.sha256` on the
  release rather than carried over.
- `ci-dev-py39.in` caps `pytest<9`. The resolver already picks 8.x for that
  target, but a declared bound makes a hand-edit fail at compile time rather than
  at install time on the 3.9 runner. The lock is byte-identical.

### Security

- CI no longer fetches an unpinned pip before installing the hash-locked
  tooling. `pip install --upgrade pip` reached PyPI without a pin two steps
  before the `--require-hashes` install that exists to avoid exactly that, and
  OpenSSF Scorecard flagged both sites. The runner images ship a pip far newer
  than `--require-hashes` needs, so the upgrade bought nothing.

### Added

- `docs/FEEDS.md`: the whole side-feed model in one page — which of the seven
  families each indicator belongs to, the batch parallel-array shape and the
  singular per-bar `feeds` envelope (and why they differ), the market panel a
  batch scan assembles for itself, the two ways to name a reference series, the
  `breadth` knobs with their defaults, and how to ask a spec what it needs before
  assembling data. Six of the seven families had no page at all: the per-bar
  envelope, order books and derivatives were documented nowhere.
- `ARCHITECTURE.md` gains the workspace navigation table, performance
  characteristics, stability commitments, what is deliberately absent, and the
  known sharp edges — the streaming screener's inability to derive the market
  panel, the C ABI's once-per-delivered-response rule, `fuzz/` being a detached
  workspace, the narrow coverage scope, and the release gate on `wickra-exchange`.
- The governance and security documents carry the sections the main repository
  has: contribution flow and succession in `GOVERNANCE.md`, the actors and a
  threats-and-mitigations table in `THREAT_MODEL.md`, the assurance case,
  secrets management, release verification, support timeline and remediation
  policy in `SECURITY.md`, documentation-first and support expectations in
  `SUPPORT.md`, per-language build instructions and the lockfile policy in
  `CONTRIBUTING.md`, and how to make contact in `MAINTAINERS.md`.

### Changed

- CI rides out transient network failures instead of failing a job that then
  needs a manual re-run: a workflow-level env block (`CARGO_NET_RETRY`, the npm
  fetch-retry pair, `PIP_RETRIES`), a cargo pre-fetch with real backoff in all
  fifteen Rust jobs, and the toolchain-download retry the `wasm` job was missing.
  The block also carries `RUSTFLAGS: -D warnings`, which was previously enforced
  only through clippy.
- Coverage measures both feature sets and the CLI, merged into one report.
  `folded_states` exists twice behind `#[cfg(feature = "parallel")]`, so a
  single-configuration run left the rayon fold neither covered nor counted as
  missing.
- The nightly benchmark runs the sequential path as well as the parallel one.
  `BENCHMARKS.md` described the matrix as covering both; nothing measured the
  second.
- `repo-metadata.toml` names this repository's own site, its discussions URL and
  its Codecov slug, and the GitHub repository has a homepage set. The README's
  Docs badge and closing paragraph, and `docs/README.md`, pointed a reader at
  wickra.org for material that lives in `docs/` and on screener.wickra.org.

### Fixed

- `ARCHITECTURE.md` no longer says `screener-core` depends on `wickra-data`; the
  CLI does, since the CSV loader moved there.
- `check_readme_links.py` skips `bindings/wasm/pkg-node/`, the wasm-pack output
  the WASM tests load, which carries a generated README.

[Unreleased]: https://github.com/wickra-lib/wickra-screener/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/wickra-lib/wickra-screener/releases/tag/v0.1.0
