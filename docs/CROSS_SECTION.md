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

## The market panel the screener builds for itself

The `breadth` **condition** above is the screener's own construction. Separately,
`wickra-core` ships a family of breadth **indicators** — `AdvanceDecline`,
`McClellanOscillator`, `Trin`, `NewHighsNewLows`, `PercentAboveMa` and the rest —
which read a *cross-section*: a panel of the whole market at one bar.

A backtester has to be handed that panel, because it sees one instrument. A
screener already holds the universe it is scanning, so a batch scan assembles the
panel itself and a breadth screen needs no second data source:

```json
{ "kind": "indicator", "name": "McClellanOscillator", "params": [19, 39] }
```

Each symbol contributes a member built from its own bar: the change against its
previous close, the bar volume, whether the bar set a new high or low over
`breadth.period`, and whether the close is above an `Sma(breadth.ma_period)`.
Supplying an explicit `sections` feed overrides the derived panel — an explicit
panel is a statement about the data, the derived one a convenience.

Two limits, both refusals rather than quiet wrong answers:

- `BullishPercentIndex` reads a point-and-figure buy signal, which cannot be read
  off a candle. A derived panel would report it false for every symbol and answer
  with a confident zero, so a spec naming it against a derived panel is **refused**.
  Supply an explicit `sections` feed for it.
- A **streaming** screener sees one symbol's bar at a time and cannot know which
  other symbols will print at that timestamp, so it cannot derive the panel. A
  breadth spec fed through `feed` needs an explicit `sections` feed per bar.

## One bar means one bar

A scan that assembles the panel itself, or that names a `reference` symbol, folds
the universe **one timestamp at a time** rather than symbol by symbol. The
timeline is the union of the universe's bar timestamps, not the intersection, so
one halted symbol does not rewind the scan for everyone else: at each timestamp
only the symbols that printed advance, and the rest hold their state.

A symbol whose most recent bar is older than the last bar in the universe is
named in the report's `stale` list. It is still screened — its state is its last
bar — but a halted or delisted name no longer reads like a live one.

## See also

- [CONDITIONS.md](CONDITIONS.md) · [INDICATORS.md](INDICATORS.md) · [STREAMING.md](STREAMING.md) · [Cookbook.md](Cookbook.md)
