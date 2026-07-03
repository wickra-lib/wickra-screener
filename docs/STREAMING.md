# Streaming vs batch

Screener runs a spec two ways over the same core. Both produce the identical
`ScanReport` for the same data — the streaming path is the batch path fed one bar
at a time.

## Batch

`scan_batch(universe, spec)` folds every symbol over its full candle history and
evaluates the condition tree at the last bar:

```rust
use screener_core::{ScanSpec, scan_batch};
let spec: ScanSpec = serde_json::from_str(spec_json)?;
let report = scan_batch(&data, &spec)?;   // data: BTreeMap<symbol, Vec<Candle>>
```

This is the one-shot form used by the CLI and by each binding's `scan` command.
Over the whole universe it parallelises with rayon by default; the WASM build
(and `--no-default-features`) runs the same fold sequentially. The two are
**byte-for-byte identical** — proven by the golden corpus.

## Streaming

For a live scan, feed candles as they arrive and evaluate on demand — O(1) per
tick, no re-scan of history:

```rust
let mut s = Screener::new(spec_json)?;
for (symbol, candle) in feed {
    s.feed(&symbol, &candle)?;   // update that symbol's O(1) indicator state
}
let report = s.evaluate();       // scan the current cross-section
s.reset();                       // clear all per-symbol state, keep the spec
```

`feed` advances one symbol's streaming indicator state; `evaluate` runs the
condition tree over the current state of every symbol; `reset` clears state
without reparsing the spec.

## Equivalence

Feeding a symbol's full history bar by bar and calling `evaluate` yields the same
report as `scan_batch` on that history — the `streaming_eq_batch` test pins this,
and the proptest suite checks it over random universes and conditions. So you can
develop a spec in batch against recorded data and deploy it unchanged on a live
feed.

## JSON-over-C-ABI

Every binding reaches this through one entry point, `command(json) -> json`:
`{"cmd":"scan","data":{...}}` runs a batch scan; the streaming verbs feed and
evaluate incrementally. The binding returns the core's response string verbatim,
which is why the output is byte-identical across all ten languages.

## See also

- [CONDITIONS.md](CONDITIONS.md) · [INDICATORS.md](INDICATORS.md) · [CROSS_SECTION.md](CROSS_SECTION.md) · [Cookbook.md](Cookbook.md)
