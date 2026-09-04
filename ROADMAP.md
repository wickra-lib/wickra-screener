# Roadmap

`wickra-screener` is built out in phases, mirroring the proven structure of the
Wickra exchange, backtester and terminal repos. Each phase lands as reviewed,
CI-green pull requests. Status below is updated as phases complete.

## Phases

0. **Scaffold** — workspace, governance, supply-chain config, `.github`
   scaffolding. *Landed.*
1. **`wickra-screener-core`** — the `ScanSpec` condition tree, the per-symbol
   `SymbolState` fold, the `Universe`, condition evaluation and `scan_batch`,
   with near-total coverage via inline tests. *Landed.*
2. **`screener-cli`** — the reference `wickra-screener` binary: load a spec and a
   universe directory, run a scan, render the report as text or JSON. *Landed.*
3. **Bindings** — the C ABI hub first, then native Python, Node and WASM, then C,
   C++, C#, Go, Java and R over the hub; each exposes the `Screener` handle +
   `command` + `version`, with a completeness guard. *Landed.*
4. **Golden harness** — a fixed deterministic universe and canonical specs whose
   blessed reports are the byte-exact, cross-language parity corpus. *Landed.*
5. **Test rigor** — conformance, golden, streaming-equals-batch equivalence,
   property tests, fuzz targets and a criterion benchmark suite. *Landed.*
6. **ABI harness + examples** — cbindgen header sync-check and one runnable
   example per language, with a C/C++ CMake harness. *Landed.*
7. **CI/CD** — the full workflow matrix (all languages), OpenSSF Scorecard, Best
   Practices, link check, and the release workflow. *Landed; the release
   workflow has never run, see Publication.*
8. **README, badges, docs** — the banner + badge treatment and the docs guides
   (conditions, indicators, cross-section, streaming, cookbook). *Landed.*

## Beyond 1.0

- Richer condition kinds and cross-section reductions as the corpus grows.
- A buffered streaming step, so a streaming screener can assemble the market
  cross-section the way a batch scan does. Today it sees one symbol's bar at a
  time and cannot know which other symbols will print at that timestamp, so a
  breadth spec fed through `feed` needs an explicit `sections` feed.

## Publication

0.1.0 is published. Getting there took one thing that was not in this
repository, and it is worth leaving written down because the same shape will
recur.

The optional `live` feature depends on `wickra-exchange`, and cargo refuses a
git dependency in a published package whether or not the feature is on. So the
crate could not reach crates.io until that sibling had its own first release —
and neither could anything else, because `github-release` lists `cargo-publish`
among its `needs`, deliberately, so that a release page can never read as
complete while one registry is empty. A tag pushed before that point would have
produced a failed `cargo-publish`, a skipped `github-release`, and no release
assets at all — including the `wickra-screener-c-<triple>.tar.gz` archives an R
`configure` downloads and r-universe needs before it can build the R package.

One unpublished sibling crate therefore gated the whole first release rather
than the crates.io half of it. That was the right trade: the alternative,
decoupling `github-release` from `cargo-publish`, buys a release page by
partially reversing the rule that job exists for.

## Non-goals

- **Indicator code in this repository.** Indicators come from the `wickra-core`
  registry; the screener composes them, it does not reimplement them.
- **Conditions as code.** A screen is a serde `ScanSpec`, never a Rust closure,
  so it crosses the C ABI and WASM unchanged.
- **A hosted service or stored credentials.** The screener runs locally; it holds
  no order-secret material and places no orders.
