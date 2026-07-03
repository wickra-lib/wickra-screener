# Conditions & the ScanSpec

A scan is described by a `ScanSpec` — a JSON (or TOML) document that is **data,
not code**, so the identical spec runs in every binding and crosses the C ABI and
WASM unchanged. This page is the reference for its shape.

## Top level

```json
{
  "universe": ["AAA", "BBB", "CCC"],
  "condition": { "type": "cmp", "...": "..." },
  "rank": { "by": { "kind": "price", "field": "close" }, "desc": true },
  "limit": 25
}
```

| Field | Required | Meaning |
|-------|----------|---------|
| `universe` | yes | the symbols to scan (non-empty) |
| `condition` | yes | the condition tree evaluated at the last bar |
| `rank` | no | order matches by an expression; `desc` picks direction |
| `limit` | no | keep at most N matches after ranking (positive) |

## Expressions (`kind`)

An expression reduces to one scalar per symbol at the evaluated bar. The tag is
`kind` (snake_case):

| `kind` | Fields | Value |
|--------|--------|-------|
| `const` | `value` | a literal number |
| `price` | `field` | a candle field: `open` / `high` / `low` / `close` / `volume` |
| `indicator` | `name`, `params`, `field?` | a Wickra indicator output |

Indicator `name` is the **PascalCase** registry name (`Rsi`, `Ema`, `Roc`,
`Sma`, `Macd`, `BollingerBands`, …); `params` is its parameter list; the optional
`field` selects a sub-output of a multi-output indicator (e.g. `Macd(12,26,9)`
with `field: "hist"`). See [INDICATORS.md](INDICATORS.md).

```json
{ "kind": "indicator", "name": "Rsi", "params": [14] }
{ "kind": "indicator", "name": "BollingerBands", "params": [20, 2], "field": "lower" }
```

## Conditions (`type`)

A condition returns true/false per symbol. The tag is `type` (snake_case):

### `cmp` — compare two expressions

```json
{ "type": "cmp",
  "left":  { "kind": "indicator", "name": "Rsi", "params": [14] },
  "op":    "lt",
  "right": { "kind": "const", "value": 30.0 } }
```

`op` is one of: `gt`, `lt`, `ge`, `le`, `eq`, `crosses_above`, `crosses_below`.
The two crossing operators compare the last two bars (the left series crossing
the right series between the previous and current bar).

### `cross_section` — rank within the universe

```json
{ "type": "cross_section",
  "expr":   { "kind": "indicator", "name": "Roc", "params": [10] },
  "metric": "rank",
  "op":     "le",
  "value":  3 }
```

`metric` is `rank`, `percentile_rank` or `z_score`; the metric of `expr` across
the whole universe is compared with `op` against `value`. See
[CROSS_SECTION.md](CROSS_SECTION.md).

### `breadth` — a market-wide gate

```json
{ "type": "breadth",
  "inner": { "type": "cmp",
             "left":  { "kind": "price", "field": "close" },
             "op":    "gt",
             "right": { "kind": "indicator", "name": "Sma", "params": [200] } },
  "op":    "ge",
  "ratio": 0.6 }
```

`breadth` measures the fraction of the universe for which `inner` holds and
compares it (`op`) against `ratio` in `[0, 1]` — e.g. "at least 60% of symbols
are above their `Sma(200)`". A breadth condition may **not** nest another breadth.

### `all` / `any` / `not` — combinators

```json
{ "type": "all", "conditions": [ { "...": "..." }, { "...": "..." } ] }
{ "type": "any", "conditions": [ { "...": "..." } ] }
{ "type": "not", "condition":  { "...": "..." } }
```

## Result keys

Each matched symbol reports its expression values under stable keys:
`const(30)`, `price.close`, `Rsi(14)`, `Macd(12,26,9).hist`; a cross-section
value keys as `Roc(10)#rank`. These keys are byte-identical in every binding.

## See also

- [INDICATORS.md](INDICATORS.md) · [CROSS_SECTION.md](CROSS_SECTION.md) · [STREAMING.md](STREAMING.md) · [Cookbook.md](Cookbook.md)
