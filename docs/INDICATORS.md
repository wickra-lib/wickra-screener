# Indicators

An `indicator` expression resolves to the output of a [Wickra](https://github.com/wickra-lib/wickra)
streaming indicator, computed over each symbol's history and read at the
evaluated bar. Screener does not reimplement any indicator — it drives the shared
registry.

Most indicators are driven by the candle alone. The rest read a **side feed**:
a reference series, a derivatives tick, an order-book snapshot, the bar's trades,
or the market cross-section. Supply the feed and the indicator works like any
other; leave it out and the spec is **refused**, naming the feed it needs — see
[Data input](#data-input) below.

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

Every indicator consumes `Candle { time, open, high, low, close, volume }`. Some
also consume one side feed:

| Feed | Payload field | Families that need it |
|------|---------------|-----------------------|
| — | (none) | the bulk of the registry: `Rsi`, `Ema`, `Macd`, `BollingerBands`, … |
| reference series | `reference` | pairwise: `Beta`, `Alpha`, `PearsonCorrelation`, `Cointegration`, … |
| derivatives tick | `derivs` | `FundingRate`, `OpenInterestDelta`, `TakerBuySellRatio`, … |
| order book | `books` | `Microprice`, `DepthSlope`, `OrderFlowImbalance`, … |
| trades | `trades` | `Vpin`, `CumulativeVolumeDelta`, `TradeImbalance`, … |
| trades **and** order book | `trades` + `books` | `EffectiveSpread`, `KylesLambda`, `RealizedSpread` |
| market cross-section | `sections` | breadth: `AdvanceDecline`, `McClellanOscillator`, `Trin`, … |

A batch scan supplies them as parallel arrays beside the candles, one entry per
candle:

```json
{ "AAA": { "candles": [ … ],
           "books":   [ { "bids": [{"price": 99.5, "size": 12}],
                          "asks": [{"price": 100.5, "size": 9}] }, … ] } }
```

A symbol that needs no feed keeps the bare form, `{ "AAA": [ candle, … ] }`.
A streaming caller passes the same fields per bar under `feeds`:

```json
{ "cmd": "feed", "symbol": "AAA", "candle": { … },
  "feeds": { "orderbook": { "bids": [ … ], "asks": [ … ] } } }
```

A feed array that is not exactly as long as the candle array is an error, and so
is a spec naming an indicator whose feed the scan does not carry. Both are
refusals rather than an indicator that quietly returns nothing on every bar.

The CLI reads candle-only universes from per-symbol CSV files (`<SYMBOL>.csv`);
a universe with side feeds comes in as a JSON dataset on stdin (`--stdin`). The
bindings pass either shape as JSON in the `command` payload.

## See also

- [CONDITIONS.md](CONDITIONS.md) · [CROSS_SECTION.md](CROSS_SECTION.md) · [STREAMING.md](STREAMING.md) · [Cookbook.md](Cookbook.md)
