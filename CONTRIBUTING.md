# Contributing to wickra-screener

Thanks for your interest. Issues, bug reports, ideas and pull requests are all
welcome at <https://github.com/wickra-lib/wickra-screener>. For larger changes,
open an issue first so we can agree on the approach.

## Orientation

- The core — the `ScanSpec` condition tree, the per-symbol `SymbolState` fold,
  the `Universe`, condition evaluation and `scan_batch` — lives in
  `crates/screener-core`. Conditions are **data, not code**: a serde tree, so the
  same screen crosses the C ABI and WASM unchanged.
- The reference consumer is `crates/screener-cli` (the `wickra-screener` binary).
- Every language binding lives under `bindings/<lang>/` and exposes the same
  data-driven surface: a `Screener` handle plus `command(json) -> json` and
  `version`. Bindings must preserve the **golden-parity invariant**: given the
  spec + universe in `golden/{specs,data}/`, the same command produces the
  byte-identical report in `golden/expected/`.

## The dev loop

Every change runs green locally before a commit:

```bash
cargo fmt --all
cargo test --workspace --all-features
cargo test --workspace --no-default-features   # sequential path == parallel path
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
```

`cargo fmt --all` and the `clippy -D warnings` gate are enforced in CI on three
operating systems, across both the default (rayon `parallel`) and
`--no-default-features` (sequential / WASM) feature sets — a scan must produce a
byte-identical report either way.

## Conventions

- **Commits are signed** and follow Conventional Commits (`feat:`, `fix:`,
  `chore:`, `docs:`…). One logical change per commit. Open a PR against `main`;
  do not push to `main` directly.
- **All public artifacts are in English** — code, comments, commit messages, PR
  titles and bodies, issues and docs.
- **No secrets, ever** — not in code, tests, fixtures, logs, issues or PRs. Any
  live-universe path is opt-in behind the `live` feature and never uses real
  keys in tests.
- **Production code only** — no mocks outside `#[cfg(test)]`, no TODO stubs, and
  no defensive branches that can never run (they fail coverage).

## Adding a condition or a metric

Conditions are a serde enum, so extending the screen means adding a variant, not
a closure. A new comparator, cross-section metric or breadth condition is added
to `crates/screener-core/src/spec.rs` and handled in `src/eval.rs`, with a serde
round-trip test and a golden fixture. Indicators themselves come from the
[Wickra](https://github.com/wickra-lib/wickra) core registry by name and
parameters — no indicator code lives here. See `docs/CONDITIONS.md` and
`docs/INDICATORS.md`.

## Developer Certificate of Origin

Contributions are accepted under the [DCO](DCO); sign off your commits with
`git commit -s`. By contributing you agree your work is dual-licensed under
`MIT OR Apache-2.0`.
