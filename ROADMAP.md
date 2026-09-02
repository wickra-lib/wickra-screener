# Roadmap

`wickra-screener` is built out in phases, mirroring the proven structure of the
Wickra exchange, backtester and terminal repos. Each phase lands as reviewed,
CI-green pull requests. Status below is updated as phases complete.

## Phases

0. **Scaffold** — workspace, governance, supply-chain config, `.github`
   scaffolding. *Landed.*
1. **`screener-core`** — the `ScanSpec` condition tree, the per-symbol
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

The crate is not on crates.io yet, and cannot be until `wickra-exchange`
publishes its first release: the optional `live` feature depends on it from git,
and cargo refuses a git dependency in a published package whether or not the
feature is on. `wickra-terminal` is in the same position for the same reason.
Today that reads:

```
$ cargo publish --dry-run -p screener-core
error: failed to prepare local package for uploading
  no matching package named `wickra-exchange` found
  location searched: crates.io index
```

The other artefacts — the Python, Node, WASM, NuGet, Maven, Go and C ABI ones —
carry no such constraint *in themselves*, but they do not escape it either:
`github-release` lists `cargo-publish` among its `needs`, deliberately, so that a
release page can never read as complete while one registry is empty. The
consequence is that a `v*` tag pushed today produces a failed `cargo-publish`, a
skipped `github-release`, and therefore no release assets at all — including the
`wickra-screener-c-<triple>.tar.gz` archives that an R `configure` would download
and that r-universe needs before it can build the R package.

So one unpublished sibling crate gates the whole first release, not just the
crates.io half of it. Publishing `wickra-exchange` is the fix that matches the
cause; decoupling `github-release` from `cargo-publish` would work too, at the
price of partially reversing the rule that job was given.

## Non-goals

- **Indicator code in this repository.** Indicators come from the `wickra-core`
  registry; the screener composes them, it does not reimplement them.
- **Conditions as code.** A screen is a serde `ScanSpec`, never a Rust closure,
  so it crosses the C ABI and WASM unchanged.
- **A hosted service or stored credentials.** The screener runs locally; it holds
  no order-secret material and places no orders.
