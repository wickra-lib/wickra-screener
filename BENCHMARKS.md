# Benchmarks

A screener's cost is dominated by folding every symbol's history through its
indicators and evaluating the condition tree at each bar. The benchmarks here
measure that **core scan work**, so throughput scales predictably with the
universe size and the number of indicators a spec references.

## What is measured

The `screener-bench` crate (criterion) covers `scan_batch` across a matrix of:

- **Universe size** — 100, 1 000 and 10 000 symbols.
- **Indicator count** — specs referencing roughly 5 and 20 indicators.
- **Execution path** — the default rayon `parallel` feature vs
  `--no-default-features` (the sequential / WASM path), which must produce a
  byte-identical report.

## Methodology

Run against fixed, in-process synthetic universes so the numbers are reproducible
and contain no I/O variance:

```bash
cargo bench -p screener-bench
```

## Results

_To be filled in from the criterion run in P-SCR-5.6._ Figures will be the median
estimate on a single machine; treat them as orders of magnitude, not guarantees —
they vary with CPU core count and toolchain.

## Caveats

These figures bound the screener's own scan overhead only. End-to-end time in a
real run also depends on loading the universe from disk or a live feed, which
these in-process benchmarks do not capture.
