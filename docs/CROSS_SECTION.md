# Cross-section & breadth

Per-symbol conditions (`cmp`) look at one symbol at a time. **Cross-section** and
**breadth** conditions look at *every symbol of the same bar at once* — the thing
a per-symbol loop cannot express. Both are computed once per bar over the full
cross-section and stay byte-identical between the parallel and sequential scan
paths.

## Cross-section metrics

A `cross_section` condition reduces an expression across the universe to a metric
per symbol, then compares it:

```json
{ "type": "cross_section",
  "expr":   { "kind": "indicator", "name": "Roc", "params": [10] },
  "metric": "rank",
  "op":     "le",
  "value":  3 }
```

Let `xᵢ` be `expr` for symbol *i*, over the *n* symbols of the universe at the
evaluated bar (symbols still in warmup are excluded from the population).

| `metric` | Definition | Range |
|----------|------------|-------|
| `rank` | 1-based position of `xᵢ` sorted **descending** (largest = rank 1) | `1..n` |
| `percentile_rank` | fraction of symbols with a strictly smaller value | `[0, 1]` |
| `z_score` | `(xᵢ − μ) / σ`, with `μ`, `σ` the universe mean and standard deviation | unbounded |

The metric is compared with `op` (`gt`/`lt`/`ge`/`le`/`eq`) against `value`. The
example above keeps the three highest 10-bar `Roc` symbols (`rank ≤ 3`).

`z_score` uses the population standard deviation; when `σ` is ~0 (a flat
cross-section) the z-score is reported as 0 rather than dividing by zero. Compare
z-scores with an epsilon (`1e-9`), not exact equality — the variance is computed
in floating point.

The cross-section value keys as `expr#metric`, e.g. `Roc(10)#rank`.

## Breadth

A `breadth` condition measures the fraction of the universe for which an inner
condition holds and uses it as a market-wide gate:

```json
{ "type": "breadth",
  "inner": { "type": "cmp",
             "left":  { "kind": "price", "field": "close" },
             "op":    "gt",
             "right": { "kind": "indicator", "name": "Sma", "params": [200] } },
  "op":    "ge",
  "ratio": 0.6 }
```

Let `k` be the number of symbols for which `inner` holds and `n` the universe
size. Breadth holds when `k / n  <op>  ratio` — here, "at least 60% of symbols
are above their `Sma(200)`". `ratio` is in `[0, 1]`.

Breadth is a **gate**: it evaluates to the same truth value for every symbol of
the bar, so it is normally combined (`all`) with a per-symbol condition to filter
individual matches. A breadth condition may **not** nest another breadth.

## See also

- [CONDITIONS.md](CONDITIONS.md) · [INDICATORS.md](INDICATORS.md) · [STREAMING.md](STREAMING.md) · [Cookbook.md](Cookbook.md)
