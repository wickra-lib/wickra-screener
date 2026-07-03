# Indicators

An `indicator` expression resolves to the output of a [Wickra](https://github.com/wickra-lib/wickra)
streaming indicator, computed over each symbol's candle history and read at the
evaluated bar. Screener does not reimplement any indicator — it drives the shared
registry, so all 514 O(1) streaming indicators are available to a scan.

## Referencing an indicator

```json
{ "kind": "indicator", "name": "Rsi", "params": [14] }
```

| Field | Meaning |
|-------|---------|
| `name` | the **PascalCase** registry name (`Rsi`, `Ema`, `Roc`, `Sma`, `Atr`, `Macd`, `BollingerBands`, …) |
| `params` | the parameter list, in the indicator's own order (`[14]`, `[12, 26, 9]`, `[20, 2]`) |
| `field` | optional — a named sub-output of a multi-output indicator |

`name` is case-sensitive and must match the registry exactly; an unknown name is
a spec error, surfaced in-band as `{"ok":false,"error":...}`.

## Single-output vs multi-output

Most indicators emit one value per bar (`Rsi`, `Ema`, `Roc`). A multi-output
indicator emits several named fields; select one with `field`:

```json
{ "kind": "indicator", "name": "Macd", "params": [12, 26, 9], "field": "hist" }
{ "kind": "indicator", "name": "BollingerBands", "params": [20, 2], "field": "upper" }
```

Omitting `field` on a multi-output indicator picks the registry's primary field.

## Keys

An indicator value keys as `Name(p,p,...)` — e.g. `Rsi(14)`, `Roc(10)` — and a
multi-output field as `Name(p,...).field`, e.g. `Macd(12,26,9).hist` or
`BollingerBands(20,2).lower`. Whole-valued parameters render as integers. These
keys appear verbatim in every binding's `ScanReport`.

## Warmup

Each indicator needs a warmup window before it produces a finite value (e.g.
`Rsi(14)` needs 14 bars). During warmup the indicator contributes no match; a
symbol only matches once every referenced indicator is warm at the evaluated bar.
Provide enough history per symbol for the longest window your spec references.

## Data input

Indicators consume `Candle { time, open, high, low, close, volume }`. The CLI
reads them from per-symbol CSV files (`<SYMBOL>.csv`) or a JSON dataset on stdin;
the bindings pass candles as JSON in the `command` payload.

## See also

- [CONDITIONS.md](CONDITIONS.md) · [CROSS_SECTION.md](CROSS_SECTION.md) · [STREAMING.md](STREAMING.md) · [Cookbook.md](Cookbook.md)
