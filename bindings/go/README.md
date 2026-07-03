# Wickra Screener — Go

Idiomatic Go bindings for the `wickra-screener` data-driven core over its C ABI
hub (cgo). Build a `Screener` from a spec JSON, drive it with command JSON, read
back scan reports — the same protocol as every other binding.

## Install

```bash
go get github.com/wickra-lib/wickra-screener-go
```

The binding links the prebuilt C ABI library, staged per platform under
`./lib/<goos>_<goarch>/`, with the header vendored under `./include`.

## Usage

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

## API

| Symbol | Description |
|--------|-------------|
| `New(specJSON string) (*Screener, error)` | Build a screener from a spec JSON. |
| `(*Screener) Command(cmdJSON string) (string, error)` | Apply a command JSON, return the response JSON. |
| `(*Screener) Close()` | Free the handle (idempotent; a finalizer also frees it). |
| `Version() string` | The library version. |

## License

`MIT OR Apache-2.0`.
