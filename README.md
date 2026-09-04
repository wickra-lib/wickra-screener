<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![CodeQL](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codeql.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/codeql.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![GitHub release](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/release.svg)](https://github.com/wickra-lib/wickra-screener/releases/latest)
[![crates.io](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/crates.svg)](https://crates.io/crates/wickra-screener)
[![PyPI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/pypi.svg)](https://pypi.org/project/wickra-screener/)
[![npm](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/npm.svg)](https://www.npmjs.com/package/wickra-screener)
[![NuGet](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/nuget.svg)](https://www.nuget.org/packages/Wickra.Screener)
[![Maven Central](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/maven.svg)](https://central.sonatype.com/artifact/org.wickra/wickra-screener)
[![Go module](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/go.svg)](https://pkg.go.dev/github.com/wickra-lib/wickra-screener-go)
[![R-universe](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/r-universe.svg)](https://wickra-lib.r-universe.dev)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](#license)
[![OpenSSF Scorecard](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/scorecard.svg)](https://scorecard.dev/viewer/?uri=github.com/wickra-lib/wickra-screener)
[![OpenSSF Best Practices](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/best-practices.svg)](https://www.bestpractices.dev)
[![Build provenance](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/provenance.svg)](https://github.com/wickra-lib/wickra-screener/attestations)
[![Docs](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/docs.svg)](https://github.com/wickra-lib/wickra-screener/tree/main/docs)
[![Verified across 10 languages](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/verified.svg)](golden/)
[![Live demo](https://img.shields.io/badge/live%20demo-live.wickra.org-3b82f6)](https://live.wickra.org)

---

# Wickra Screener

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators.**

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

> **Part of the [Wickra ecosystem](https://github.com/wickra-lib):** the same data-driven core and ten-language binding surface also power [wickra-exchange](https://github.com/wickra-lib/wickra-exchange), [wickra-backtest](https://github.com/wickra-lib/wickra-backtest), [wickra-terminal](https://github.com/wickra-lib/wickra-terminal) and 20 more — see [the full list](https://github.com/wickra-lib).

Wickra Screener is one data-driven core, [`wickra-screener-core`](crates/wickra-screener-core):
a serde **condition tree** (`ScanSpec`) is folded over each symbol's history with
the [Wickra](https://github.com/wickra-lib/wickra) indicator library — 497 of them
resolvable by name — evaluated at the latest bar, and scanned across the whole
universe in parallel (rayon) or sequentially (the WASM fallback) —
**byte-for-byte identical**.

Because conditions are **data, not code**, the exact same scan crosses the C ABI
and WASM unchanged. The core is exposed as a **JSON-over-C-ABI data API**
(`Screener::command`) in **Rust, Python, Node.js, WASM, C, C++, C#, Go, Java and
R**, so a developer in any language runs the same screen.

- **Batch** — `scan_batch(universe, spec)` folds every symbol over its full history and evaluates at the last bar.
- **Streaming** — `feed(symbol, candle)` + `evaluate()`, O(1) per tick, for a live scan over the current state.
- **Cross-section & breadth** — rank, percentile, z-score and market-breadth conditions that see every symbol of a bar at once.

```rust
use wickra_screener_core::{scan_batch, ScanSpec, Screener};

// Batch: fold every symbol's full history, evaluate at the last bar.
let spec: ScanSpec = serde_json::from_str(spec_json)?;
let report = scan_batch(universe, &spec)?;

// Streaming: the same spec, O(1) per tick, evaluated against the current state.
let mut screener = Screener::new(spec_json)?;
screener.feed("BTCUSDT", &candle)?;
let report = screener.evaluate();
```

## Status

**0.1.1 — the first complete release.** The core, the CLI, all ten language bindings, the
byte-exact golden corpus, property + fuzz tests, benchmarks and one runnable
example per language are in place and green across the full CI matrix
(10 languages × 3 OS). [ROADMAP.md](ROADMAP.md) has what is done, what is open
and what is not planned.

## Documentation

- [Architecture](ARCHITECTURE.md) — the core, the data-driven boundary, the binding surface.
- Guides under [`docs/`](docs): [Conditions & ScanSpec](docs/CONDITIONS.md) · [Indicators](docs/INDICATORS.md) · [Cross-section & breadth](docs/CROSS_SECTION.md) · [Streaming vs batch](docs/STREAMING.md) · [Cookbook](docs/Cookbook.md) · [Internals](docs/ARCHITECTURE.md).
- [ROADMAP.md](ROADMAP.md) · [BENCHMARKS.md](BENCHMARKS.md) · [THREAT_MODEL.md](THREAT_MODEL.md) · [SECURITY.md](SECURITY.md).

## Quickstart

```bash
# Scan a universe of CSV candle files against a spec, human-readable table:
cargo run -p wickra-screener -- --spec golden/specs/momentum.json --data golden/data

# Raw ScanReport JSON (the same bytes every binding returns):
cargo run -p wickra-screener -- --spec golden/specs/momentum.json --data golden/data --format json
```

Each `<SYMBOL>.csv` in the `--data` directory is one symbol's candle history. The
first row must be a header naming `timestamp`, `open`, `high`, `low`, `close` and
`volume`; the columns are matched by name, so their order does not matter and any
extra columns are ignored. A leading UTF-8 byte-order mark, which spreadsheet
exports add, is stripped. Or pass a JSON dataset on standard input with
`--stdin`. Built with the optional
`live` feature, `--live <venue>` pulls the spec's universe straight from one of
the ten exchanges the [wickra-exchange](https://github.com/wickra-lib/wickra-exchange)
facade supports — public market data only, no key sent and no order placed:

```bash
cargo run -p wickra-screener --features live --   --spec myscreen.json --live binance --interval 1h --bars 500
```

## ScanSpec / conditions

A scan is a JSON (or TOML) document: a `universe`, a `condition` tree, and an
optional `limit` and `rank`. Expressions (`kind`) are constants, price fields or
indicators; conditions (`type`) compare, cross, or aggregate them.

```json
{
  "universe": ["AAA", "BBB", "CCC"],
  "condition": {
    "type": "all",
    "conditions": [
      { "type": "cmp",
        "left":  { "kind": "indicator", "name": "Rsi", "params": [14] },
        "op":    "lt",
        "right": { "kind": "const", "value": 30.0 } },
      { "type": "cmp",
        "left":  { "kind": "indicator", "name": "Ema", "params": [7] },
        "op":    "crosses_above",
        "right": { "kind": "indicator", "name": "Ema", "params": [19] } }
    ]
  },
  "rank": { "by": { "kind": "indicator", "name": "Roc", "params": [10] }, "desc": true },
  "limit": 25
}
```

- **Expressions** (`kind`): `const`, `price` (`open`/`high`/`low`/`close`/…), `indicator` (a PascalCase Wickra indicator + params — `Rsi`, `Ema`, `Macd`, `BollingerBands`, …).
- **Conditions** (`type`): `cmp` (`gt`/`lt`/`crosses_above`/…), `cross_section`, `breadth`, and the boolean combinators `all` / `any` / `not`.

## Cross-section & breadth

Most screens look at one symbol at a time. Cross-section and breadth conditions
look at **every symbol of the same bar at once** — the thing a per-symbol loop
cannot express:

- **Cross-section** — `percentile_rank`, `z_score` and `rank` of a symbol's metric within the universe (e.g. "top-decile 10-bar momentum": `Roc(10)` ranked across all symbols).
- **Breadth** — a market-wide predicate ("more than 60% of the universe above its `Sma(200)`") usable as a gate on the individual matches.

Both are evaluated once per bar over the full cross-section and stay
byte-identical between the parallel and sequential scan paths.

## Use in any language

The same `Screener` handle — construct from a JSON spec, drive with
`command(json) -> json`, read `version` — is reachable from every binding:

```python
from wickra_screener import Screener
s = Screener('{"universe":["AAA","BBB"],"condition":{"type":"cmp",'
             '"left":{"kind":"price","field":"close"},"op":"gt",'
             '"right":{"kind":"const","value":10.0}}}')
report = s.command('{"cmd":"scan","data":{"AAA":[...],"BBB":[...]}}')  # JSON ScanReport
```

The C ABI hub (`bindings/c`) backs C, C++, C#, Go, Java and R; Rust, Python,
Node.js and WASM are native. See each `bindings/<lang>/README.md` and the runnable
[`examples/`](examples).

## Project layout

```
crates/wickra-screener-core    the data-driven core (ScanSpec, Expr, Condition, scan_batch, streaming)
crates/screener-cli     the CLI (bin: wickra-screener)
crates/wickra-screener-bench   criterion benchmarks
bindings/{python,node,wasm,c,go,csharp,java,r}   the ten-language surface
golden/                 CSV + JSON universes, specs, and byte-exact expected reports
fuzz/                   cargo-fuzz targets (spec_parse, condition_eval, scan_batch, symbol_fold)
examples/               one runnable "scan a small universe" example per language
```

## Building from source

```bash
cargo build --workspace
cargo run -p wickra-screener -- --spec golden/specs/momentum.json --data golden/data
```

## Building everything from source

The Rust core, the CLI and the WASM package build from the workspace; each
binding has its own toolchain and builds on its own.

```bash
# Rust core, CLI, C ABI, WASM crate
cargo build --workspace --all-features

# The fuzz crate is a detached workspace (cargo-fuzz builds it with sanitizer
# flags on nightly), so --workspace does not reach it.
cargo check --manifest-path fuzz/Cargo.toml

# Python: an abi3 wheel via maturin
python -m venv .venv && . .venv/bin/activate
pip install maturin pytest
maturin develop --release -m bindings/python/Cargo.toml

# Node.js: a native addon via napi-rs
( cd bindings/node && npm ci && npm run build )

# WASM: a browser/bundler package via wasm-pack
wasm-pack build bindings/wasm --target bundler --out-dir pkg

# C ABI: the cdylib and staticlib every non-native binding links against
cargo build -p wickra-screener-c --release

# C / C++: the example harness, which is also the header smoke test
cmake -S examples/c -B examples/c/build && cmake --build examples/c/build

# C# · Go · Java · R link the C ABI above; each needs it built first
( cd bindings/csharp && dotnet build )
( cd bindings/go && go build ./... )
( cd bindings/java && mvn -q package )
R CMD INSTALL bindings/r
```

`--features live` on the workspace or the CLI adds the exchange-sourced universe;
it pulls the networking stack in, so it is off by default.

## Testing

```bash
# The core, in both scan configurations — they must agree byte for byte
cargo test --workspace --all-features
cargo test --workspace --no-default-features

cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo fmt --all --check
cargo deny check

# Per binding
"$(pwd)/.venv/bin/python" -m pytest bindings/python/tests -q
( cd bindings/node && npm test )
( cd bindings/go && go test ./... )
( cd bindings/java && mvn -q test )
( cd bindings/csharp && dotnet test )
Rscript bindings/r/tests/run_tests.R
ctest --test-dir examples/c/build --output-on-failure

# Fuzzing (nightly)
cargo +nightly fuzz run spec_parse -- -max_total_time=30
```

Every binding runs the same committed corpus under [`golden/`](golden/) and has
to reproduce each expected report **byte for byte** — that is what makes the
cross-language claim checkable rather than asserted. Regenerating those files is
`cargo test -p wickra-screener-core --test golden -- --ignored`, and the diff is meant
to be read before it is committed.

## Requirements

- **Rust** ≥ 1.86 (workspace MSRV; the Node binding needs ≥ 1.88).
- Binding toolchains as needed: Node ≥ 22, Python ≥ 3.9, a C toolchain, .NET 8,
  JDK 22+, Go 1.23, R ≥ 2.10 — see each `bindings/<lang>/README.md`.

## Benchmarks

`crates/wickra-screener-bench` measures `scan_batch` scaling by universe size and
indicator count, parallel vs sequential. See [BENCHMARKS.md](BENCHMARKS.md).

## Ecosystem

Part of the [Wickra](https://github.com/wickra-lib/wickra) family — each one a
data-driven core with a CLI and the same ten-language binding surface:

- [**wickra**](https://github.com/wickra-lib/wickra) — main library (Rust core + Python / Node.js / WASM bindings + a C ABI for C / C++ / C# / Go / Java / R)
- [**wickra-playground**](https://github.com/wickra-lib/wickra-playground) — a polyglot strategy playground: one StrategySpec live side by side in Python, Rust, JS and Go, entirely in the browser
- [**wickra-exchange**](https://github.com/wickra-lib/wickra-exchange) — unified market-data + execution across ten crypto exchanges
- [**wickra-backtest**](https://github.com/wickra-lib/wickra-backtest) — event-driven backtester over the Wickra core
- [**wickra-terminal**](https://github.com/wickra-lib/wickra-terminal) — the trading terminal: a TUI and a browser renderer over the stack
- [**wickra-xray**](https://github.com/wickra-lib/wickra-xray) — market-microstructure explorer: footprint, order-book heatmap, liquidation map, funding/OI divergence
- [**wickra-radar**](https://github.com/wickra-lib/wickra-radar) — perp-universe alert radar: OI delta, funding flip, book imbalance, liquidation clusters, OI/price divergence
- [**wickra-copilot**](https://github.com/wickra-lib/wickra-copilot) — local market copilot grounded in real order-book, liquidation and funding microstructure
- [**wickra-shazam**](https://github.com/wickra-lib/wickra-shazam) — match an asset's current microstructure fingerprint against its entire history
- [**wickra-benchmark**](https://github.com/wickra-lib/wickra-benchmark) — reproducible, golden-verified benchmark suite — recompute any (strategy, dataset, report) in ten languages and confirm it byte-for-byte
- [**wickra-strategy-ci**](https://github.com/wickra-lib/wickra-strategy-ci) — Jest for trading strategies: golden-pin the report, catch regressions in CI, property-test against fuzzed data
- [**wickra-verify**](https://github.com/wickra-lib/wickra-verify) — confirm or refute a claimed backtest report against its strategy and data, in ten languages
- [**wickra-proof**](https://github.com/wickra-lib/wickra-proof) — Proof-of-Backtest: deterministic (spec, data) → report + blake3 hash, recomputable byte-for-byte in ten languages
- [**wickra-zk**](https://github.com/wickra-lib/wickra-zk) — prove a backtest zero-knowledge — on-chain-verifiable performance without revealing the data or the strategy
- [**wickra-impact**](https://github.com/wickra-lib/wickra-impact) — the backtester that knows you would have moved the market: agent-based fills on the real historical L2 order book
- [**wickra-darwin**](https://github.com/wickra-lib/wickra-darwin) — evolutionary strategy search at millions of backtests per second, mutating and crossing JSON specs across the 514-indicator space
- [**wickra-gym**](https://github.com/wickra-lib/wickra-gym) — a Gymnasium-compatible, microstructure-aware backtest environment with O(1) steps for deterministic RL rollouts
- [**wickra-feature-store**](https://github.com/wickra-lib/wickra-feature-store) — OHLCV and microstructure streams into ML-ready feature matrices over 514 O(1) streaming indicators
- [**wickra-genome**](https://github.com/wickra-lib/wickra-genome) — a vector database of the whole market: every asset a 514-dim live vector, for similarity search, clustering and anomaly detection
- [**wickra-timemachine**](https://github.com/wickra-lib/wickra-timemachine) — scrub the whole market like a video — every symbol, full order book, rewound to any moment via deterministic re-fold
- [**wickra-synth**](https://github.com/wickra-lib/wickra-synth) — deterministic synthetic market microstructure: OHLCV, order book, trades and funding from a single seed
- [**wickra-compile**](https://github.com/wickra-lib/wickra-compile) — compile a strategy spec into a standalone deployable: a WASM module, a self-contained binary, or a `no_std` artifact
- [**wickra-embed**](https://github.com/wickra-lib/wickra-embed) — allocation-free, `no_std` streaming indicators for bare-metal and HFT, byte-for-byte identical to the core
- [**wickra-pico**](https://github.com/wickra-lib/wickra-pico) — the O(1) indicator core running bare-metal on a $5 Raspberry Pi Pico — the LED blinks on the EMA cross

The screener's own guides live in [`docs/`](docs/) beside the code; its site,
with the in-browser demo and the benchmark figures, is at
[screener.wickra.org](https://screener.wickra.org). The indicator library's
reference is at [docs.wickra.org](https://docs.wickra.org) and the org landing
page at [wickra.org](https://wickra.org).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
Commits are signed and in English; open a PR against `main`.

## Security

See [SECURITY.md](SECURITY.md) and [THREAT_MODEL.md](THREAT_MODEL.md). Report
vulnerabilities privately — never in a public issue.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.

## Disclaimer

Wickra Screener is analysis software: it computes indicator values and evaluates
conditions over historical and live market data. It is provided "as is", without
warranty of any kind, and is **not financial advice** — it places no orders.
Trading carries risk of loss; review the code and use at your own discretion.

---

<p align="center">
  <a href="https://github.com/wickra-lib/wickra-screener">
    <img alt="GitHub stars" src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/stars.svg">
  </a>
  <a href="https://github.com/wickra-lib/wickra-screener/network/members">
    <img alt="GitHub forks" src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/forks.svg">
  </a>
  <a href="https://github.com/wickra-lib/wickra-screener/issues">
    <img alt="GitHub issues" src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/issues.svg">
  </a>
</p>

<p align="center">
  Built on <a href="https://github.com/wickra-lib/wickra">Wickra</a>. If it saved you time, the cheapest way to say thanks is to ⭐ the repo.
</p>

<p align="center">
  <img alt="wickra-screener star history" width="640"
       src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/star-history.svg">
</p>
