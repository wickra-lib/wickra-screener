<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![NuGet](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/nuget.svg)](https://www.nuget.org/packages/Wickra.Screener)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — C#

---

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators — for C#. `dotnet add package Wickra.Screener` — prebuilt native library, no system dependencies.**

.NET bindings for [`wickra-screener`](https://github.com/wickra-lib/wickra-screener)
over the C ABI hub, via source-generated P/Invoke. Build a `Screener` from a spec
JSON, drive it with command JSON and read back scan reports — the same protocol
the CLI and every other binding speak.

## Install

```bash
dotnet add package Wickra.Screener
```

The native library ships prebuilt per platform under `runtimes/<rid>/native/`,
selected automatically. There is nothing to compile. Targets .NET 8 and later.

Requires .NET 8+. The native library (`wickra_screener`) must be resolvable on the
loader path — `PATH` on Windows, `LD_LIBRARY_PATH` on Linux, `DYLD_LIBRARY_PATH`
on macOS. Licensed under `MIT OR Apache-2.0`.

### Building from this repository (contributors)

| Path | What it is |
| --- | --- |
| `WickraScreener/` | The published package. Its own `README.md` is the long description NuGet renders. |
| `WickraScreener.Tests/` | xUnit suite: golden parity against the shared fixtures, the screener protocol, and the streaming path. |

See [`WickraScreener/README.md`](https://github.com/wickra-lib/wickra-screener/blob/main/bindings/csharp/WickraScreener/README.md) for the full API walk-through,
and [`examples/csharp/`](https://github.com/wickra-lib/wickra-screener/blob/main/examples/csharp) for a runnable program.

## Quick start

```csharp
using Wickra.Screener;

const string spec = """
{"universe":["AAA","BBB"],"condition":{"type":"cmp",
"left":{"kind":"price","field":"close"},"op":"gt",
"right":{"kind":"const","value":10.0}}}
""";

using var screener = new Screener(spec);
string report = screener.Command("""{"cmd":"scan","data":{ … }}""");
```

## Benchmark

Every binding forwards to the same data-driven Rust core, so what this one adds is
the call overhead of `[LibraryImport]` P/Invoke over the C ABI, not a different result. The core's throughput is
measured by the repository's benchmark suite and the nightly `bench.yml` run; the
numbers, the machine and how to reproduce them are in the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-screener/blob/main/BENCHMARKS.md).

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-screener>
- **Docs** (guides, spec reference, cookbook): <https://screener.wickra.org>
- **Runnable example:** [`examples/csharp/`](https://github.com/wickra-lib/wickra-screener/tree/main/examples/csharp)

Wickra Screener ships native bindings for Python, Node.js, WASM and Rust, plus a C ABI hub that any
C-capable language (C, C++, C#, Go, Java, R) links against — all forwarding to the
same data-driven, `unsafe`-forbidden Rust core.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org** with a subject line starting `[wickra security]`. Full
policy: <https://github.com/wickra-lib/wickra-screener/blob/main/SECURITY.md>.

## Disclaimer

Wickra Screener is analysis software: it computes indicator values and evaluates
conditions over historical and live market data. It is provided "as is", without
warranty of any kind, and is **not financial advice** — it places no orders.
Trading carries risk of loss; review the code and use at your own discretion.

## License

Licensed under either of [Apache-2.0](https://github.com/wickra-lib/wickra-screener/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/wickra-lib/wickra-screener/blob/main/LICENSE-MIT) at your option.
