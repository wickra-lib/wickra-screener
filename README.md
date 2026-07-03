<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Screener — parallel multi-symbol screening over 514 streaming indicators" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![Status](https://img.shields.io/badge/status-pre--release-orange)](https://github.com/wickra-lib/wickra-screener)
[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![CodeQL](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codeql.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/codeql.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](#license)
[![OpenSSF Scorecard](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/scorecard.svg)](https://scorecard.dev/viewer/?uri=github.com/wickra-lib/wickra-screener)
[![OpenSSF Best Practices](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/best-practices.svg)](https://www.bestpractices.dev/)
[![Build provenance](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/provenance.svg)](https://github.com/wickra-lib/wickra-screener/attestations)
[![Docs](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/docs.svg)](https://wickra.org)

---

# Wickra Screener

**Scan thousands of symbols in parallel against data-driven conditions over 514 O(1) streaming indicators.**

Wickra Screener is one data-driven core, [`screener-core`](crates/screener-core):
a serde **condition tree** (`ScanSpec`) is folded over each symbol's history with
the [Wickra](https://github.com/wickra-lib/wickra) library of 514 O(1) streaming
indicators, evaluated at the latest bar, and scanned across the whole universe in
parallel (rayon) or sequentially (the WASM fallback) — **byte-for-byte identical**.

Because conditions are **data, not code**, the exact same scan crosses the C ABI
and WASM unchanged. The core is exposed as a **JSON-over-C-ABI data API**
(`Screener::command`) in **Rust, Python, Node.js, WASM, C, C++, C#, Go, Java and
R**, so a developer in any language runs the same screen.

- **Batch** — `scan_batch(universe, spec)` folds every symbol over its full history and evaluates at the last bar.
- **Streaming** — `feed(symbol, candle)` + `evaluate()`, O(1) per tick, for a live scan over the current state.
- **Cross-section & breadth** — rank, percentile, z-score and market-breadth conditions that see every symbol of a bar at once.

## Status

**Pre-release — functionally complete, CI-verified, not yet published.** The core,
the CLI, all ten language bindings, the byte-exact golden corpus, property + fuzz
tests, benchmarks and one runnable example per language are in place and green
across the full CI matrix (10 languages × 3 OS). Not yet released to any
registry — track progress in [ROADMAP.md](ROADMAP.md).

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

Each `<SYMBOL>.csv` in the `--data` directory is one symbol's candle history; or
pass a JSON dataset on standard input with `--stdin`.

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
crates/screener-core    the data-driven core (ScanSpec, Expr, Condition, scan_batch, streaming)
crates/screener-cli     the CLI (bin: wickra-screener)
crates/screener-bench   criterion benchmarks
bindings/{python,node,wasm,c,go,csharp,java,r}   the ten-language surface
golden/                 CSV + JSON universes, specs, and byte-exact expected reports
fuzz/                   cargo-fuzz targets (spec_parse, condition_eval, scan_batch, symbol_fold)
examples/               one runnable "scan a small universe" example per language
```

## Building from source

```bash
cargo build --workspace
cargo test  --workspace --all-features
cargo test  --workspace --no-default-features   # sequential scan path
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p wickra-screener -- --spec golden/specs/momentum.json --data golden/data
```

## Requirements

- **Rust** ≥ 1.86 (workspace MSRV; the Node binding needs ≥ 1.88).
- Binding toolchains as needed: Node ≥ 22, Python ≥ 3.9, a C toolchain, .NET 8,
  JDK 22+, Go 1.23, R — see each `bindings/<lang>/README.md`.

## Benchmarks

`crates/screener-bench` measures `scan_batch` scaling by universe size and
indicator count, parallel vs sequential. See [BENCHMARKS.md](BENCHMARKS.md).

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
