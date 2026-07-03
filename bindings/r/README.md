# Wickra Screener — R

R bindings for the `wickra-screener` data-driven core, over its C ABI hub
(`.Call`). Build a screener from a spec JSON, drive it with command JSON, read
back scan reports — the same protocol as the CLI and every other binding.

## Usage

```r
library(wickrascreener)

spec <- paste0(
  '{"universe":["AAA","BBB"],"condition":{"type":"cmp",',
  '"left":{"kind":"price","field":"close"},"op":"gt",',
  '"right":{"kind":"const","value":10.0}}}'
)

screener <- wkscreen_new(spec)
cmd <- paste0(
  '{"cmd":"scan","data":{',
  '"AAA":[{"time":1,"open":5,"high":5,"low":5,"close":5,"volume":1}],',
  '"BBB":[{"time":1,"open":15,"high":15,"low":15,"close":15,"volume":1}]}}'
)
cat(wkscreen_command(screener, cmd), "\n")  # {"matches":[{"symbol":"BBB",...}],"scanned":2}
cat(wkscreen_version(), "\n")
```

## Build and test from source

The package links the `wickra_screener` C ABI, located out-of-tree via two
environment variables:

```bash
# Build the C ABI shared library first.
cargo build -p wickra-screener-c --release

export WKSCREEN_INC="$PWD/bindings/c/include"
export WKSCREEN_LIB="$PWD/target/release"
# The loader must also find the shared library at run time:
export LD_LIBRARY_PATH="$WKSCREEN_LIB:$LD_LIBRARY_PATH"   # PATH on Windows

R CMD INSTALL bindings/r
Rscript bindings/r/tests/run_tests.R
```

## API

| Function | Description |
|----------|-------------|
| `wkscreen_new(spec_json)` | Build a screener from a spec JSON (errors on an invalid spec). |
| `wkscreen_command(screener, cmd_json)` | Apply a command JSON, return the response JSON. |
| `wkscreen_version()` | The library version. |

## License

`MIT OR Apache-2.0`.
