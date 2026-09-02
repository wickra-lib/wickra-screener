# Roadmap

`wickra-screener` is built out in phases, mirroring the proven structure of the
Wickra exchange, backtester and terminal repos. Each phase lands as reviewed,
CI-green pull requests. Status below is updated as phases complete.

## Phases

0. **Scaffold** — workspace, governance, supply-chain config, `.github`
   scaffolding. *In progress.*
1. **`screener-core`** — the `ScanSpec` condition tree, the per-symbol
   `SymbolState` fold, the `Universe`, condition evaluation and `scan_batch`,
   with near-total coverage via inline tests.
2. **`screener-cli`** — the reference `wickra-screener` binary: load a spec and a
   universe directory, run a scan, render the report as text or JSON.
3. **Bindings** — the C ABI hub first, then native Python, Node and WASM, then C,
   C++, C#, Go, Java and R over the hub; each exposes the `Screener` handle +
   `command` + `version`, with a completeness guard.
4. **Golden harness** — a fixed deterministic universe and canonical specs whose
   blessed reports are the byte-exact, cross-language parity corpus.
5. **Test rigor** — conformance, golden, streaming-equals-batch equivalence,
   property tests, fuzz targets and a criterion benchmark suite.
6. **ABI harness + examples** — cbindgen header sync-check and one runnable
   example per language, with a C/C++ CMake harness.
7. **CI/CD** — the full workflow matrix (all languages), OpenSSF Scorecard, Best
   Practices, link check, and the release workflow.
8. **README, badges, docs** — the banner + badge treatment and the docs guides
   (conditions, indicators, cross-section, streaming, cookbook).

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
Everything else — the Python, Node, WASM and C ABI artefacts — has no such
constraint.

## Non-goals

- **Indicator code in this repository.** Indicators come from the `wickra-core`
  registry; the screener composes them, it does not reimplement them.
- **Conditions as code.** A screen is a serde `ScanSpec`, never a Rust closure,
  so it crosses the C ABI and WASM unchanged.
- **A hosted service or stored credentials.** The screener runs locally; it holds
  no order-secret material and places no orders.
