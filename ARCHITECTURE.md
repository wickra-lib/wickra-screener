# Architecture

`wickra-screener` is one data-driven core with many thin consumers. A screen is a
piece of **data** — a serde condition tree (`ScanSpec`) — that is folded over each
symbol's history with the [Wickra](https://github.com/wickra-lib/wickra) library
of 514 O(1) streaming indicators and evaluated across the whole universe. Because
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
resolver the backtester uses), so the screener inherits all 514 indicators and
any future additions for free. Price fields read straight from the candle.

## Integration with the rest of Wickra

`wickra-screener` sits beside the other Wickra consumers — the terminal, the
backtester and the exchange layer — over the same core. It depends on
`wickra-core` (indicators) and `wickra-data` (`Candle` + CSV); the optional
`live` feature pulls `wickra-exchange` to source a live symbol universe. It never
places orders and holds no order-secret material.
