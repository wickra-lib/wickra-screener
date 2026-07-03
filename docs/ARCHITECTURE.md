# Architecture (internals)

The top-level [ARCHITECTURE.md](../ARCHITECTURE.md) gives the high-level shape;
this page covers how the core actually evaluates a scan. The whole product is
**one data-driven core** (`screener-core`) and N thin consumers — the CLI and the
ten language bindings — each of which only ships a spec and reads back a report.

## The pipeline

```
ScanSpec (JSON/TOML)
   │  parse + validate (non-empty universe, positive limit, no nested breadth)
   ▼
IndicatorSet   one O(1) streaming instance per (indicator, params) in the spec,
   │           shared across the symbols so each is built once
   ▼
per symbol:  fold candles → SymbolState (advances every indicator, keeps last 2
   │                                      values for crossing operators)
   ▼
evaluate condition tree at the last bar → bool + the referenced expression values
   ▼
cross-section / breadth passes (see CROSS_SECTION.md) over the whole bar
   ▼
rank + limit → ScanReport
```

## Key types

- **`Expr`** (tag `kind`) — a scalar: `const`, `price`, `indicator`. `Expr::key()`
  produces the stable result key (`Rsi(14)`, `price.close`, `Macd(12,26,9).hist`).
- **`Condition`** (tag `type`) — `cmp`, `cross_section`, `breadth`, `all`, `any`,
  `not`. Validation rejects a breadth nested inside another breadth.
- **`IndicatorSet`** — resolves each distinct `(name, params)` once from the
  Wickra registry and advances them together, so two conditions referencing
  `Ema(20)` share a single instance.
- **`SymbolState`** — a symbol's live indicator outputs plus the previous bar's
  values, which is what makes `crosses_above` / `crosses_below` O(1).

## Parallel vs sequential

`scan_batch` folds symbols independently, so the universe is scanned with rayon
by default (`parallel` feature). The WASM build and `--no-default-features` use
the identical sequential fold. Cross-section and breadth are computed after the
per-symbol fold, over the collected bar, so they see the same population either
way. The `golden` corpus pins that both paths emit **byte-identical** JSON.

## Boundary: JSON in, JSON out

The public surface is a JSON-over-C-ABI data API. `Screener::command_json` (and
each binding's `command`) takes a command string and returns a response string;
domain errors travel in-band as `{"ok":false,"error":...}` rather than as
transport failures. Because every binding returns the core's response **verbatim**,
the output is byte-identical in all ten languages — there is no per-language JSON
reformatting to drift.

## Integration

The indicator registry, the `Candle` type and the O(1) indicator implementations
come from the Wickra ecosystem (`wickra-backtest-core`'s registry over the
`wickra` indicator library); `screener-core` adds only the spec model, the fold,
and the cross-section / breadth / rank layers.

## See also

- [CONDITIONS.md](CONDITIONS.md) · [INDICATORS.md](INDICATORS.md) · [CROSS_SECTION.md](CROSS_SECTION.md) · [STREAMING.md](STREAMING.md) · [Cookbook.md](Cookbook.md)
