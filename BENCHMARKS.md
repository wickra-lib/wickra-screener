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

Measured with `cargo bench -p screener-bench` (criterion) on a Windows x86-64
laptop, default `parallel` (rayon) path. Figures are the median estimate; treat
them as orders of magnitude, not guarantees — they vary with CPU core count and
toolchain.

| Benchmark | Universe × indicators | Median | Throughput |
|-----------|-----------------------|--------|------------|
| `scan_batch/100sym_5ind`     | 100 × ~5    | 1.34 ms | ~75 K sym/s |
| `scan_batch/100sym_20ind`    | 100 × ~20   | 5.32 ms | ~19 K sym/s |
| `scan_batch/1000sym_5ind`    | 1 000 × ~5  | 12.7 ms | ~79 K sym/s |
| `scan_batch/1000sym_20ind`   | 1 000 × ~20 | 51.3 ms | ~20 K sym/s |
| `scan_batch/10000sym_5ind`   | 10 000 × ~5 | 126 ms  | ~80 K sym/s |
| `scan_batch/10000sym_20ind`  | 10 000 × ~20 | 513 ms | ~19 K sym/s |

The takeaway: per-symbol throughput stays roughly constant as the universe grows
(~80 K symbols/s at 5 indicators, ~19 K at 20), so scan cost scales linearly with
universe size and with the number of distinct indicators a spec references — a
1 000-symbol, 5-indicator screen finishes in ~13 ms. The nightly `bench.yml`
workflow reruns this on a clean Linux runner for tracking over time.

## Caveats

These figures bound the screener's own scan overhead only. End-to-end time in a
real run also depends on loading the universe from disk or a live feed, which
these in-process benchmarks do not capture.
