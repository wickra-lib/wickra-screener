// A runnable Go example: scan a small universe through the binding.
//
//	cargo build --release -p wickra-screener-c
//	# stage the library under bindings/go/lib/<goos>_<goarch>/ (CI does this)
//	cd examples/go && go run .
package main

import (
	"fmt"

	wickra "github.com/wickra-lib/wickra-screener-go"
)

const spec = `{"universe":["AAA","BBB"],"condition":{"type":"cmp",` +
	`"left":{"kind":"price","field":"close"},"op":"gt",` +
	`"right":{"kind":"const","value":10.0}}}`

const cmd = `{"cmd":"scan","data":{` +
	`"AAA":[{"time":1,"open":5,"high":5,"low":5,"close":5,"volume":1}],` +
	`"BBB":[{"time":1,"open":15,"high":15,"low":15,"close":15,"volume":1}]}}`

func main() {
	screener, err := wickra.New(spec)
	if err != nil {
		panic(err)
	}
	defer screener.Close()

	report, err := screener.Command(cmd)
	if err != nil {
		panic(err)
	}

	fmt.Println("wickra-screener", wickra.Version())
	fmt.Println(report)
}
