<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Screener — parallel multi-symbol screening over 514 streaming indicators" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![Status](https://img.shields.io/badge/status-pre--release-orange)](https://github.com/wickra-lib/wickra-screener)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

<!-- Skeleton README (P-SCR-0.12). The full ~20-badge block (CI, CodeQL, codecov,
     crates.io/PyPI/npm/NuGet/Maven/Go/R-universe, Scorecard, Best-Practices,
     Provenance, Docs, Verified) and the finished sections are assembled in
     P-SCR-8.1, once the per-product badge SVGs are generated in the .github repo
     (P-SCR-8.2). Until then this stays link-clean (no 404s on the repo page). -->

---

# Wickra Screener

**Scan thousands of symbols in parallel against data-driven conditions over 514 O(1) streaming indicators.**

Wickra Screener is one data-driven core, `screener-core`: a serde **condition tree**
(`ScanSpec`) is folded over each symbol's history with the
[Wickra](https://github.com/wickra-lib/wickra) library of 514 O(1) streaming
indicators, evaluated at the latest bar, and scanned across the whole universe in
parallel (rayon) or sequentially (the WASM fallback) — **byte-for-byte identical**.

Because conditions are **data, not code**, the exact same scan crosses the C ABI
and WASM unchanged. The core is exposed as a **JSON-over-C-ABI data API**
(`Screener::command`) in **Rust, Python, Node.js, WASM, C, C++, C#, Go, Java and
R**, so a developer in any language runs the same screen.

- **Batch** — `scan_batch(universe, spec)` folds every symbol over its full history and evaluates at the last bar.
- **Streaming** — `feed(symbol, candle)` + `evaluate()`, O(1) per tick, for a live scan over the current state.
- **Cross-section & breadth** — rank, percentile, z-score and market-breadth conditions that see every symbol of a bar at once.

## Status

**Pre-release — under active construction.** This repository is being built out
phase by phase (scaffold → core → CLI → eight language bindings → golden corpus →
property/fuzz tests → CI → docs). It is not yet published to any registry.

## Documentation

The full documentation — the ScanSpec / condition reference, cross-section and
breadth semantics, per-binding quickstarts and benchmarks — is finalized in this
README and under `docs/` during the documentation phase.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Disclaimer

Wickra Screener is analysis software: it computes indicator values and evaluates
conditions over historical and live market data. It does not provide financial
advice and places no orders. Trading carries risk; use at your own discretion.
