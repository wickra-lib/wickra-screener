# Architecture

`wickra-screener` is one data-driven core with many thin consumers. A screen is a
piece of **data** — a serde condition tree (`ScanSpec`) — that is folded over each
symbol's history with the [Wickra](https://github.com/wickra-lib/wickra) library
of 497 O(1) streaming indicators and evaluated across the whole universe. Because
the screen is data, not code, the exact same scan runs natively, across the C ABI
and in WASM, byte-for-byte identical.

## The layers

```
CONSUMERS   CLI: crates/screener-cli        ·   any language via its binding (command JSON)
      ▲ ScanReport JSON                                      ▲
CORE  crates/screener-core:  ScanSpec (JSON) → Universe<Symbol, SymbolState> (O(1)/bar)
                             → condition eval → scan_batch (rayon) / streaming
      ▼ data-driven JSON API in ten languages (like backtest run_json / terminal command_json)
BINDINGS  python · node · wasm · c (C-ABI hub) → c / c++ / c# / go / java / r
CORES  wickra-core (indicators) · wickra-data (Candle / CSV) · [feature "live"] wickra-exchange
```

Each binding ships the same surface — a `Screener` handle plus
`command(json) -> json` and `version` — with its own README, tests, a runnable
example, and a completeness guard.

## The core is data-driven

Conditions are a serde enum (`Condition`: comparisons, crossovers, cross-section
rank/percentile/z-score, market breadth, and `all`/`any`/`not`), never Rust
closures. Closures cannot cross the C ABI or compile to a WASM data boundary; a
serde tree can. So a Python or Go caller sends the same `ScanSpec` JSON a Rust
caller would, and gets the same `ScanReport` back.

## The command boundary

Every consumer talks to the core through a single JSON-in / JSON-out function,
`Screener::command`. The binding does no logic of its own — it forwards the
command string and returns the core's response verbatim. That verbatim pass-through
is what makes the golden corpus a **cross-language** parity corpus: the same
command produces a byte-identical report in every language, with no per-language
JSON reformatting.

## Two modes, one result type

- **Batch** — `scan_batch(universe, spec)` folds every symbol over its full
  history and evaluates at the last bar. Symbols fold independently, so the scan
  runs in parallel via rayon (the default `parallel` feature) and sequentially as
  the WASM fallback (`--no-default-features`) — the two paths produce a
  byte-identical `ScanReport`.
- **Streaming** — `feed(symbol, candle)` + `evaluate()`, O(1) per tick, for a
  live scan over the current universe state.

Both modes evaluate the same condition tree and return the same `ScanReport`.

## Cross-section and breadth

Comparisons and crossovers are per-symbol, but rank, percentile, z-score and
market-breadth conditions need **every symbol of a bar at once**. The `Universe`
therefore holds the `SymbolState` of all symbols; a cross-section reduction runs
serially over the universe in key order for determinism, and only **ready**
symbols (those past their indicators' warmup) take part.

## Indicators come from the Wickra core

No indicator mathematics lives in this repository. `IndicatorSet` resolves each
building block from the `wickra-core` registry by name and parameters (the same
resolver the backtester uses), so the screener inherits all 497 indicators and
any future additions for free. Price fields read straight from the candle.

## Integration with the rest of Wickra

`wickra-screener` sits beside the other Wickra consumers — the terminal, the
backtester and the exchange layer — over the same core. It depends on
`wickra-core` (indicators, through the backtester's registry); the CLI depends on
`wickra-data` to read its CSV universe. The optional `live` feature pulls
`wickra-exchange` to source a live symbol universe. It never places orders and
holds no order-secret material.

## Workspace layout — what lives where

| Path | What is in it | Published as |
| --- | --- | --- |
| `crates/screener-core` | The scan engine: `ScanSpec`, `Expr`, `Condition`, `SymbolState`, `Universe`, `IndicatorSet`, `scan_batch`, `Screener`. | `screener-core` (crates.io) |
| `crates/screener-cli` | The reference binary: argument parsing, the CSV / stdin universe loader, text and JSON rendering. | `wickra-screener` (crates.io) |
| `crates/screener-bench` | Criterion benchmarks over `scan_batch`. | not published |
| `bindings/c` | The C ABI hub plus the generated header and the header-only C++ hull. | C ABI archives on the release page |
| `bindings/python`, `node`, `wasm` | The three native bindings, each calling the core directly. | PyPI / npm / npm |
| `bindings/csharp`, `go`, `java`, `r` | The four reaches over the C ABI hub. | NuGet / Go module mirror / Maven Central / r-universe |
| `golden/` | Specs, committed universes and blessed reports — the cross-language contract. | not published |
| `fuzz/` | cargo-fuzz targets. A **detached workspace**: `--workspace` never builds it. | not published |
| `examples/` | One runnable program per language, each built and run in CI. | not published |
| `scripts/` | The static checks CI runs, and `update-lockfiles.sh`. | not published |

## Performance characteristics

- **Per-bar cost is O(1) per indicator.** Every indicator is a streaming
  accumulator, so a symbol's cost is linear in its bar count and in the number of
  *distinct* indicator instances its spec names — an indicator referenced twice
  with the same parameters is folded once.
- **Symbols fold independently**, which is what makes the batch scan
  embarrassingly parallel; rayon splits the universe and the sequential fallback
  walks it in key order.
- **Cross-section reductions are the exception.** Rank, percentile, z-score and
  breadth need every symbol of a bar at once, so they run serially over the
  universe in key order — that ordering is what keeps the report deterministic.
- **Measured throughput** is in [`BENCHMARKS.md`](BENCHMARKS.md); the shape is
  that per-symbol throughput stays roughly constant as the universe grows, so
  cost scales linearly with universe size and with distinct indicator count.

## Stability commitments

- **MSRV.** Workspace: Rust 1.86. Node binding: 1.88 (napi pins it).
- **The `ScanSpec` schema.** Adding a new `Condition` or `Expr` variant is a
  minor change: serde rejects an unknown variant, so an older core refuses a
  newer spec loudly rather than mis-reading it. Removing or renaming a variant is
  a major event.
- **The command boundary.** `{"cmd": ...}` names are part of the public contract
  in every language; a new command is minor, a removed one is major.
- **The report shape.** Adding a field to `ScanReport` is non-breaking — every
  binding passes the JSON through verbatim and consumers accept extra keys.
  Fields that are empty are omitted rather than serialised as `null`.
- **Pre-1.0 caveat.** Until 1.0 the spec and report schemas may still change
  between minor versions; `CHANGELOG.md` names every such change.

## What is **deliberately** not in this repo

- **Indicator mathematics.** Every indicator comes from the `wickra-core`
  registry by name. A screener that reimplemented them would drift from the
  library its results are supposed to match.
- **Conditions as code.** A screen is never a Rust closure. A closure cannot
  cross the C ABI or a WASM data boundary, and the golden corpus could not
  compare it across languages.
- **Order placement, credentials, portfolio state.** The screener answers "which
  symbols match"; acting on that answer is the backtester's or the caller's job.
- **A second copy of the API reference.** The per-language reference is generated
  from the source; `docs/` carries only what a generator cannot produce.

## Open questions / known sharp edges

Documented so a contributor does not re-discover them.

- **A streaming screener cannot derive the market panel.** It sees one symbol's
  bar at a time and cannot know which other symbols print at that timestamp, so a
  breadth spec fed through `feed` still needs an explicit `sections` feed, while
  a batch scan assembles the panel itself. The fix is a buffered streaming step
  that holds bars for timestamp T until a later one arrives; it is on the roadmap
  under *Beyond 1.0*. Until then, the streaming-equals-batch tests deliberately
  cover the candle-only specs.
- **The C ABI runs a command once per delivered response, not once per call.**
  The two-call length idiom means a length query produces a response that is not
  written; that body is held on the handle and returned by the call that reads
  it. Anything added to `bindings/c` that produces a response must go through the
  same path, or a mutating command will run twice in six of the ten reaches.
- **`fuzz/` is a detached workspace.** `cargo build --workspace` and
  `cargo clippy --workspace` never compile it, so a public signature change in
  the core compiles clean locally and fails only in CI. Run
  `cargo check --manifest-path fuzz/Cargo.toml` before pushing one.
- **Coverage measures `screener-core` under `--no-default-features` only.** The
  parallel path, the CLI and the bindings are exercised by their own suites but
  do not appear in the coverage figure, so that number is narrower than it looks.
- **The first release is gated on `wickra-exchange` reaching crates.io.** The
  optional `live` feature depends on it from git, and cargo refuses a git
  dependency in a published package whether or not the feature is on — which in
  turn blocks the release page, and with it the R `configure` and the r-universe
  entry. See the *Publication* section of [`ROADMAP.md`](ROADMAP.md).
