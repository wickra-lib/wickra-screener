<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Screener — parallel multi-symbol screening for Go" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![Go module](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/go.svg)](https://pkg.go.dev/github.com/wickra-lib/wickra-screener-go)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — Go

---

**The data-driven multi-symbol screener core for Go, over the Wickra C ABI hub via cgo.**

[Wickra Screener](https://github.com/wickra-lib/wickra-screener) folds a serde condition tree over each symbol's history against the Wickra library of 514 O(1) streaming indicators and scans the whole universe in parallel. This package is the Go binding: it consumes the C ABI hub through cgo and exposes the `Screener` handle with the same JSON protocol as every other binding.

## Install

Use the published **`wickra-screener-go`** module, which bundles the prebuilt C ABI library
for every platform, so `go get` + `go build` works with no extra steps (a C
compiler is still required, as the binding uses cgo):

```bash
go get github.com/wickra-lib/wickra-screener-go
```

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


`wickra-screener-go` is generated from this directory by the release pipeline: it mirrors the
Go sources, the vendored C ABI header (`include/wickra_screener.h`) and the prebuilt
libraries under `lib/<goos>_<goarch>/`. On Windows the DLL must be discoverable at
run time (next to the executable or on `PATH`).

## Building from this repository (contributors)

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

## License

Dual-licensed under [MIT](https://github.com/wickra-lib/wickra-screener/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/wickra-lib/wickra-screener/blob/main/LICENSE-APACHE), at your option.
