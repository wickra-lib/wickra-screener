<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514-7" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![Go module](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/go.svg)](https://pkg.go.dev/github.com/wickra-lib/wickra-screener-go)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — Go

---

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators — for Go. `go get github.com/wickra-lib/wickra-screener-go` — over the C ABI via cgo, prebuilt library bundled in the module.**

[Wickra Screener](https://github.com/wickra-lib/wickra-screener) folds a serde condition tree over each symbol's history against the Wickra library of 497 O(1) streaming indicators and scans the whole universe in parallel. This package is the Go binding: it consumes the C ABI hub through cgo and exposes the `Screener` handle with the same JSON protocol as every other binding.

## Install

Use the published **`wickra-screener-go`** module, which bundles the prebuilt C ABI
library for every platform, so `go get` + `go build` works with no extra steps
(a C compiler is still required, as the binding uses cgo):

```bash
go get github.com/wickra-lib/wickra-screener-go
```

`wickra-screener-go` is generated from this directory by the release pipeline: it mirrors
the Go sources, the vendored C ABI header (`include/wickra_screener.h`) and the prebuilt
libraries under `lib/<goos>_<goarch>/`. On Linux/macOS the library path is baked
in via rpath; on Windows the DLL must be discoverable at run time (next to the
executable or on `PATH`).

### Building from this repository (contributors)

This `bindings/go` directory is the development source. To build it directly,
compile the C ABI hub and stage the library into the per-platform directory cgo
links against:

```bash
cargo build -p wickra-screener-c --release
mkdir -p bindings/go/lib/linux_amd64                    # match your GOOS_GOARCH
cp target/release/libwickra_screener.so    bindings/go/lib/linux_amd64/   # Linux
cp target/release/libwickra_screener.dylib bindings/go/lib/darwin_arm64/  # macOS (arm64)
cp target/release/wickra_screener.dll      bindings/go/lib/windows_amd64/ # Windows
```

Then, with the library on the loader path, run `go test ./...` from this directory.

## Quick start

```go
package main

import (
	"fmt"

	wickra "github.com/wickra-lib/wickra-screener-go"
)

func main() {
	spec := `{"universe":["AAA","BBB"],"condition":{"type":"cmp",` +
		`"left":{"kind":"price","field":"close"},"op":"gt",` +
		`"right":{"kind":"const","value":10.0}}}`

	s, err := wickra.New(spec)
	if err != nil {
		panic(err)
	}
	defer s.Close()

	cmd := `{"cmd":"scan","data":{` +
		`"AAA":[{"time":1,"open":5,"high":5,"low":5,"close":5,"volume":1}],` +
		`"BBB":[{"time":1,"open":15,"high":15,"low":15,"close":15,"volume":1}]}}`

	report, err := s.Command(cmd)
	if err != nil {
		panic(err)
	}
	fmt.Println(report)         // {"matches":[{"symbol":"BBB",...}],"scanned":2}
	fmt.Println(wickra.Version())
}
```

## Benchmark

Every binding forwards to the same data-driven Rust core, so what this one adds is
the call overhead of cgo over the C ABI, not a different result. The core's throughput is
measured by the repository's benchmark suite and the nightly `bench.yml` run; the
numbers, the machine and how to reproduce them are in the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-screener/blob/main/BENCHMARKS.md).

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-screener>
- **Docs** (guides, spec reference, cookbook): <https://screener.wickra.org>
- **Runnable example:** [`examples/go/`](https://github.com/wickra-lib/wickra-screener/tree/main/examples/go)

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
