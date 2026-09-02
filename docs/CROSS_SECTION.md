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

Each symbol contributes a member built from its own bar:

| Member signal | How it is derived | Configured by |
|---------------|-------------------|---------------|
| `change` | close minus the previous close | — |
| `volume` | the bar volume | — |
| `new_high` / `new_low` | the bar's high or low against the window before it | `breadth.period` (default 52) |
| `above_ma` | close above `Sma(n)`; false until the average is warm | `breadth.ma_period` (default 200) |
| `on_buy_signal` | on a point-and-figure double-top breakout, off on a double-bottom breakdown | `breadth.pnf_box` (default 0.01), `breadth.pnf_reversal` (default 3) |

`on_buy_signal` is what `BullishPercentIndex` counts. `PointAndFigureBars` decides
where a column ends and hands each one over as it completes; the breakout is then
the close against the last completed column of that direction. The box size is a
**fraction of the symbol's first close**, not an absolute price, because one
absolute box cannot serve a forty-dollar name and a forty-thousand-dollar one at
the same time.

A symbol whose price has never reversed has completed no column, so no double top
has been made and it stands on no signal. That is a reading, not a gap.

Supplying an explicit `sections` feed overrides the derived panel — an explicit
panel is a statement about the data, the derived one a convenience.

One limit, a refusal rather than a quiet wrong answer: a **streaming** screener
sees one symbol's bar at a time and cannot know which other symbols will print at
that timestamp, so it cannot derive the panel. A breadth spec fed through `feed`
needs an explicit `sections` feed per bar.

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
