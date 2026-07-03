# Cookbook

Ready-to-adapt `ScanSpec` recipes. Each is a complete spec — drop it in a file
and run it with the CLI:

```bash
cargo run -p wickra-screener -- --spec recipe.json --data golden/data
```

The `golden/specs/` directory holds these as runnable, byte-pinned fixtures.

## Oversold reversal (mean reversion)

RSI is oversold **or** price closed below the lower Bollinger band:

```json
{
  "universe": ["AAA", "BBB", "CCC"],
  "condition": {
    "type": "any",
    "conditions": [
      { "type": "cmp",
        "left":  { "kind": "indicator", "name": "Rsi", "params": [14] },
        "op":    "lt",
        "right": { "kind": "const", "value": 30.0 } },
      { "type": "cmp",
        "left":  { "kind": "price", "field": "close" },
        "op":    "lt",
        "right": { "kind": "indicator", "name": "BollingerBands", "params": [20, 2], "field": "lower" } }
    ]
  }
}
```

## Golden-cross momentum

A fast EMA crossing above a slow EMA on the latest bar:

```json
{
  "universe": ["AAA", "BBB", "CCC"],
  "condition": {
    "type": "cmp",
    "left":  { "kind": "indicator", "name": "Ema", "params": [7] },
    "op":    "crosses_above",
    "right": { "kind": "indicator", "name": "Ema", "params": [19] }
  }
}
```

## Top-decile relative strength (cross-section)

Keep only the strongest 10-bar `Roc` symbols, ranked, best first:

```json
{
  "universe": ["AAA", "BBB", "CCC", "DDD", "EEE"],
  "condition": {
    "type": "cross_section",
    "expr":   { "kind": "indicator", "name": "Roc", "params": [10] },
    "metric": "percentile_rank",
    "op":     "ge",
    "value":  0.9
  },
  "rank": { "by": { "kind": "indicator", "name": "Roc", "params": [10] }, "desc": true },
  "limit": 10
}
```

## Breadth-gated longs

Only take individual RSI-momentum signals when the market is broadly healthy —
at least 60% of the universe above its `Sma(200)`:

```json
{
  "universe": ["AAA", "BBB", "CCC", "DDD"],
  "condition": {
    "type": "all",
    "conditions": [
      { "type": "cmp",
        "left":  { "kind": "indicator", "name": "Rsi", "params": [14] },
        "op":    "gt",
        "right": { "kind": "const", "value": 55.0 } },
      { "type": "breadth",
        "inner": { "type": "cmp",
                   "left":  { "kind": "price", "field": "close" },
                   "op":    "gt",
                   "right": { "kind": "indicator", "name": "Sma", "params": [200] } },
        "op":    "ge",
        "ratio": 0.6 }
    ]
  }
}
```

## Live scan (streaming)

The same spec, driven bar by bar instead of over history — see
[STREAMING.md](STREAMING.md):

```python
from wickra_screener import Screener
s = Screener(open("recipe.json").read())
for symbol, candle in live_feed:
    s.command('{"cmd":"feed","symbol":"%s","candle":%s}' % (symbol, candle))
print(s.command('{"cmd":"evaluate"}'))   # JSON ScanReport of the current bar
```

## See also

- [CONDITIONS.md](CONDITIONS.md) · [INDICATORS.md](INDICATORS.md) · [CROSS_SECTION.md](CROSS_SECTION.md) · [STREAMING.md](STREAMING.md)
