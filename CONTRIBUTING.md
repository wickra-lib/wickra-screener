# Contributing to wickra-screener

Thanks for your interest. Issues, bug reports, ideas and pull requests are all
welcome at <https://github.com/wickra-lib/wickra-screener>. For larger changes,
open an issue first so we can agree on the approach.

## License of contributions

wickra-screener is dual-licensed under the [MIT](LICENSE-MIT) and
[Apache-2.0](LICENSE-APACHE) licenses; users may choose either. Unless you
explicitly state otherwise, any contribution you intentionally submit for
inclusion in the work, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.

## Orientation

- The core — the `ScanSpec` condition tree, the per-symbol `SymbolState` fold,
  the `Universe`, condition evaluation and `scan_batch` — lives in
  `crates/wickra-screener-core`. Conditions are **data, not code**: a serde tree, so the
  same screen crosses the C ABI and WASM unchanged.
- The reference consumer is `crates/screener-cli` (the `wickra-screener` binary).
- Every language binding lives under `bindings/<lang>/` and exposes the same
  data-driven surface: a `Screener` handle plus `command(json) -> json` and
  `version`. Bindings must preserve the **golden-parity invariant**: given the
  spec + universe in `golden/{specs,data}/`, the same command produces the
  byte-identical report in `golden/expected/`.

## Project layout

| Path | Contents |
| --- | --- |
| `crates/wickra-screener-core` | The scan engine — the `ScanSpec` tree, the per-symbol fold, evaluation, `scan_batch` and the streaming `Screener`. |
| `crates/screener-cli` | The reference `wickra-screener` binary: a spec plus a universe of CSV files or a JSON dataset on stdin. |
| `crates/wickra-screener-bench` | Criterion benchmarks (`publish = false`). |
| `bindings/c` | C ABI — `cdylib` + `staticlib` + generated `include/wickra_screener.h`, plus the header-only C++ hull `wickra_screener.hpp`. The hub every C-capable language links against. |
| `bindings/python` | PyO3 bindings (`wickra-screener` on PyPI). |
| `bindings/node` | napi-rs bindings (`wickra-screener` on npm). |
| `bindings/wasm` | wasm-bindgen bindings (`wickra-screener-wasm` on npm). |
| `bindings/csharp` | C# binding over the C ABI (`Wickra.Screener` on NuGet), P/Invoke against `wickra_screener.h`. |
| `bindings/go` | Go binding over the C ABI via cgo, mirrored to `wickra-screener-go`. |
| `bindings/java` | Java binding over the C ABI via the FFM API (Panama, Maven Central). |
| `bindings/r` | R binding over the C ABI via `.Call` (`wickrascreener`). |
| `golden/` | The cross-language corpus: specs, committed universes and byte-exact expected reports. See [`golden/README.md`](golden/README.md). |
| `examples/` | One runnable example per language, each built and run in CI. |
| `docs/` | The guides that live beside the code, because they describe how this repository behaves. |
| `scripts/` | The static checks CI runs, and `update-lockfiles.sh`. |

## Building and testing

### Rust

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

The minimum supported Rust version is **1.86** for the workspace crates and
**1.88** for `bindings/node` (napi 3); the `msrv` and `msrv-node` CI jobs enforce
both. Please don't raise either without a dependency that actually requires it.

The `fuzz` crate is a **detached workspace**, so `cargo build --workspace` and
`cargo clippy --workspace` never compile it. Run
`cargo check --manifest-path fuzz/Cargo.toml` before any push that changes a
public signature in the core.

The four static checks in `scripts/` run in the `binding-surface` CI job and
locally the same way:

```bash
python scripts/check_binding_surface.py   # every binding matches the C header
python scripts/check_version_sync.py      # all version declarations agree
python scripts/check_readme_links.py      # binding READMEs link absolutely
python scripts/check_license_copies.py    # published packages carry their licences
```

### Python

```bash
cd bindings/python
maturin build --release --out dist
pip install --force-reinstall --no-index --find-links dist wickra-screener
pytest ../../bindings/python/tests -q
```

### Node

```bash
cd bindings/node
npm install
npx napi build --platform --release   # regenerates index.js / index.d.ts — commit them
node --test __tests__/
```

### WASM

```bash
cd bindings/wasm
wasm-pack build --target web            # what ships
wasm-pack build --target nodejs --out-dir pkg-node
node --test tests/*.test.js
```

### C and C++

```bash
cargo build -p wickra-screener-c --release
cmake -S examples/c -B examples/c/build
cmake --build examples/c/build --config Release
ctest --test-dir examples/c/build -C Release --output-on-failure
```

On Windows, configure with `-G "Visual Studio 17 2022" -A x64`: the default
generator picks MinGW gcc, which cannot link the MSVC-built Rust library.

### C#, Go, Java, R

Each links the C ABI, so build it first with
`cargo build -p wickra-screener-c --release`.

```bash
dotnet test bindings/csharp/WickraScreener.Tests/WickraScreener.Tests.csproj -c Release
(cd bindings/go && go test ./...)
mvn -B -f bindings/java test
R CMD INSTALL bindings/r && Rscript bindings/r/tests/run_tests.R
```

The Go and R builds need the native library on the loader path
(`bindings/go/lib/<os>_<arch>/`, and `WKSCREEN_INC` / `WKSCREEN_LIB` for R); the
`go` and `r` CI jobs show the exact wiring per platform.

## Lockfile policy

| Component | Lockfile | Tracked? | Why |
| --- | --- | --- | --- |
| Workspace (Rust) | `Cargo.lock` | **yes** | The workspace ships binaries (the CLI, the examples), so the dependency graph is pinned for reproducible builds. |
| `bindings/node` | `package-lock.json` | **yes** | Reproducible `npm install` for the native binding. |
| `examples/node` | `package-lock.json` | **yes** | Same — the runnable Node example links the binding via a `file:` dependency. |
| `bindings/python` | — | n/a (no lockfile) | The published wheel has no Python runtime dependencies; its native code is pinned through the workspace `Cargo.lock`. The CI dev tooling it installs is hash-locked separately — see the `.github/requirements` row. |
| `.github/requirements` | `*.txt` (hash-pinned) | **yes** | CI Python tooling, locked with `uv pip compile --generate-hashes` (OpenSSF Scorecard PinnedDependencies). Split per interpreter — `ci-dev-py39.txt` and `ci-dev-py3.txt` — because pytest 9 and iniconfig 2.3 both require Python 3.10 while the matrix still runs 3.9. |
| `fuzz` | `fuzz/Cargo.lock` | **no** (ignored) | `fuzz/` is a detached crate; `cargo-fuzz init` generates a `.gitignore` that ignores its lock. The fuzz smoke job resolves fresh. |

When adding a new committed Node package, commit its `package-lock.json` too and
remove any matching ignore rule. Do **not** add a top-level `package-lock.json` —
the repository root is not an npm package.

To refresh every committed lockfile, run `./scripts/update-lockfiles.sh`. It uses
`uv` for the Python locks, because `uv pip compile --python-version` can resolve
a *target* interpreter's hashed closure without that interpreter installed, which
is what makes the 3.9 lock possible on a 3.11 machine. uv is required rather than
fetched; `WICKRA_BOOTSTRAP_UV=1` opts into a checksum-verified download.

Dependabot's version updates are **off** for `.github/requirements`: it edits the
`.txt` directly and does not know which interpreter each lock targets. Security
updates still arrive and are judged one at a time.

## Standards for a change

- **Formatting & lints.** `cargo fmt` must leave the tree unchanged and
  `cargo clippy ... -D warnings` must be clean, under both feature sets. CI gates
  both on three operating systems.
- **Tests.** New behaviour needs tests; bug fixes need a regression test. A test
  that cannot fail is not a test — check that a new one goes red against the
  unfixed code before you keep it.
- **Golden parity.** A change that alters a report must be blessed
  (`cargo test -p wickra-screener-core --test golden -- --ignored`), and the diff
  reviewed before it is committed. Every binding compares against the same files.
- **Streaming parity.** Feeding candle by candle and evaluating must equal a
  single `scan` over the same data, in every binding — not only in the core.
- **Bindings.** A change to the public surface must be mirrored across all of
  them. `bindings/c/include/wickra_screener.h` is generated by cbindgen and its
  Go copy must match byte-for-byte; `bindings/node/index.js` and `index.d.ts` are
  generated by napi and must be committed. `scripts/check_binding_surface.py`
  holds every language to the header.
- **Conditions are data.** No screen may become a Rust closure — it has to cross
  the C ABI and WASM as JSON.
- **All public artifacts are in English** — code, comments, commit messages, PR
  titles and bodies, issues and docs.
- **No secrets, ever** — not in code, tests, fixtures, logs, issues or PRs. Any
  live-universe path is opt-in behind the `live` feature and never uses real keys
  in tests.
- **Production code only** — no mocks outside `#[cfg(test)]`, no TODO stubs, and
  no defensive branches that can never run (they fail coverage).
- **Docs.** Update the page under `docs/` and the `README.md` when behaviour or
  the public API changes.
- **Changelog.** Add an entry under `## [Unreleased]` in `CHANGELOG.md`.

## Adding a condition or a metric

Conditions are a serde enum, so extending the screen means adding a variant, not
a closure. A new comparator, cross-section metric or breadth condition is added
to `crates/wickra-screener-core/src/spec.rs` and handled in `src/eval.rs`, with a serde
round-trip test and a golden fixture. Indicators themselves come from the
[Wickra](https://github.com/wickra-lib/wickra) core registry by name and
parameters — no indicator code lives here. See `docs/CONDITIONS.md` and
`docs/INDICATORS.md`.

## Commit and pull-request workflow

1. Branch off `main`.
2. Keep commits focused — one logical change per commit, with an imperative
   subject line and a body explaining *why*. Commits are signed and follow
   Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `ci:`…).
3. Open a pull request against `main` and fill in the template. Do not push to
   `main` directly.
4. CI must be green before review.

## Reporting bugs and proposing features

Use the issue templates under
[`.github/ISSUE_TEMPLATE`](.github/ISSUE_TEMPLATE). For security-sensitive
reports, follow [`SECURITY.md`](SECURITY.md) instead of opening a public issue.

## Developer Certificate of Origin (DCO)

All contributions are made under the [Developer Certificate of Origin (DCO)
1.1](DCO). By signing off on your commits you certify that you wrote the patch,
or otherwise have the right to submit it under the project's `MIT OR Apache-2.0`
license.

Sign off every commit by adding a `Signed-off-by` trailer with your real name and
email — Git adds it automatically with the `-s` flag:

```bash
git commit -s -m "your message"
```

This produces a trailer of the form:

```
Signed-off-by: Your Name <you@example.com>
```

The name and email must match the commit author. Commits without a valid sign-off
line cannot be merged. To sign off a commit you already made, amend it with
`git commit -s --amend`, or sign off a range with an interactive rebase.

## Governance

Decision-making and maintainership are described in
[`GOVERNANCE.md`](GOVERNANCE.md); the current maintainers are listed in
[`MAINTAINERS.md`](MAINTAINERS.md).
