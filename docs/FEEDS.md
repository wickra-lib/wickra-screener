# Feeds

Most indicators need only a candle. A large part of the registry needs something
else as well — a reference series, a derivatives tick, an order-book snapshot,
the trades that printed in the bar, or the market cross-section. This page is
the whole feed model: which indicator needs what, how you supply it in a batch
scan and in a streaming one, and what the screener builds for you.

Related: [INDICATORS.md](INDICATORS.md) names an indicator and its feed;
[CROSS_SECTION.md](CROSS_SECTION.md) covers rank, percentile, z-score and
breadth; [STREAMING.md](STREAMING.md) covers batch against streaming.

## The seven feed families

`screener_core::feed_kind(name)` reports which family an indicator belongs to,
and `ScanSpec::required_feeds()` reports what a whole spec needs before you
assemble a dataset.

| Family | What the indicator is given besides the bar | Example |
| --- | --- | --- |
| `Candle` | nothing | `Sma`, `Rsi`, `Atr` |
| `Pair` | the reference series' close at the same bar | `Beta`, `PearsonCorrelation`, `Cointegration` |
| `Derivatives` | a derivatives tick — funding, open interest, mark, index | funding and basis indicators |
| `OrderBook` | an order-book snapshot | spread and depth indicators |
| `Trades` | the trades that printed within the bar | trade-flow indicators |
| `TradeQuote` | trades quoted against the book mid — needs **both** | quote-relative flow |
| `CrossSection` | the market panel for that bar | `AdvanceDecline`, `Trin`, `McClellanOscillator` |

A spec whose indicators need a feed the scan cannot supply is **refused at
validation**, naming the feed. That is deliberate: an indicator that silently
ticks and returns nothing for every bar looks like a screen that matches nothing,
which is indistinguishable from a screen that is working.

## Batch: parallel arrays beside the candles

In a batch scan each symbol is either a bare candle array or an object carrying
its side feeds. Both forms may appear in the same dataset.

```jsonc
{
  "AAA": [ {"time": 1, "open": 10, "high": 11, "low": 9, "close": 10.5, "volume": 100} ],

  "BBB": {
    "candles":   [ /* the bars */ ],
    "reference": [ /* candles of the reference series, 1:1 with `candles` */ ],
    "derivs":    [ /* one DerivativesTick per bar */ ],
    "books":     [ /* one OrderBook per bar */ ],
    "trades":    [ [ /* the TradePrints of bar 0 */ ], [ /* bar 1 */ ] ],
    "sections":  [ /* one CrossSection per bar */ ]
  }
}
```

Every side array is optional and, when present, must be the same length as
`candles`. A mismatch is an error naming the feed and both lengths, rather than a
scan that quietly reads past the end.

`trades` is an array **of arrays**: a bar can carry any number of prints,
including none.

## Streaming: one bar at a time

The streaming command carries the same information for a single bar, under
`feeds`:

```jsonc
{
  "cmd": "feed",
  "symbol": "BBB",
  "candle": {"time": 1, "open": 10, "high": 11, "low": 9, "close": 10.5, "volume": 100},
  "feeds": {
    "reference":     10.25,      // a close, not a candle
    "deriv":         { /* one DerivativesTick */ },
    "orderbook":     { /* one OrderBook */ },
    "trades":        [ /* the TradePrints of this bar */ ],
    "cross_section": { /* the market panel for this bar */ }
  }
}
```

Note the two differences from the batch shape, both because a step describes one
bar rather than a series: `reference` is a single close rather than a candle
array, and the keys are singular (`deriv`, `orderbook`, `cross_section`) where
the batch form is plural.

Omit `feeds` entirely for a candle-only symbol; the fields are individually
optional too.

## What the screener builds for you

### The market panel

A batch scan holds the whole universe, so it **assembles the cross-section
itself** and the breadth family needs no second data source. Each symbol
contributes a member built from its own bar: the change against its previous
close, its bar volume, whether it made a new high or low over `breadth.period`,
whether its close is above an `Sma(breadth.ma_period)`, and whether it is on a
point-and-figure buy signal.

An explicit `sections` feed still wins where one is given.

A **streaming** screener cannot do this. It sees one symbol's bar at a time and
cannot know which other symbols will print at that timestamp, so a breadth spec
driven through `feed` still needs an explicit `cross_section`. This is the one
place where streaming and batch are not interchangeable, and it is why the
streaming-equals-batch tests cover the candle-only specs.

### The reference series

Pairwise indicators need a second series. There are two ways to give them one:

1. **`spec.reference`** names a symbol already in the universe as the benchmark.
   Its close at the aligned bar feeds every other symbol's pairwise indicators —
   so an index or a lead symbol is written once rather than repeated under every
   entry in the dataset.
2. **A per-symbol `reference` array** in that symbol's feeds, which is the
   explicit form and mirrors how the backtester takes one.

Where both are given, the explicit per-symbol series wins.

Naming a benchmark also changes how the universe is folded: the scan walks the
timeline one timestamp at a time so the reference close is read at the *same* bar
as the symbol it is paired with, rather than each symbol being folded
independently.

## Configuring the derived panel

`spec.breadth` tunes what the screener builds:

| Field | Default | What it sets |
| --- | --- | --- |
| `period` | 52 | the lookback for new highs and new lows |
| `ma_period` | 200 | the moving average behind `above_ma` |
| `pnf_box` | 1% of the symbol's first close | the point-and-figure box size |
| `pnf_reversal` | 3 boxes | the point-and-figure reversal |

## Checking a spec before assembling data

```rust
use screener_core::{feed_kind, ScanSpec};

let spec: ScanSpec = serde_json::from_str(spec_json)?;
spec.validate()?;                    // refuses a spec whose feeds cannot be met
let needed = spec.required_feeds();  // what to go and fetch
```

`feed_kind("Beta")` answers the same question for a single indicator name, which
is what a caller assembling a dataset from an exchange usually wants.

## The corpus

`golden/data-feeds.json` is a committed universe carrying every side feed, and
the `feeds_*` specs in `golden/specs/` scan it — one per family. Every binding
runs them, so the feed shapes above are checked in ten languages rather than
described here and trusted. See [`../golden/README.md`](../golden/README.md).
