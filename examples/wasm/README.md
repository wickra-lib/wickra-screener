# wickra-screener WASM examples

Browser demos for the `wickra-screener-wasm` binding.

The WASM build carries the whole scan core with `--no-default-features`: no
rayon, so the scan is sequential rather than parallel — and byte-for-byte
identical to the parallel one, which is what the golden fixtures pin down. A
condition is data, not code, so the spec bytes on this page are the same ones
`examples/node/scan.js` sends.

## Build

The module ships as a `wasm-pack` `--target web` bundle. Build it once from the
repository root:

```bash
wasm-pack build bindings/wasm --target web --release
```

That writes `bindings/wasm/pkg/` with the `.wasm` binary, the JS loader and the
TypeScript types. The demo imports the loader via
`../../bindings/wasm/pkg/wickra_screener_wasm.js`.

## Serve

ES-module imports need a real HTTP origin, not `file://`. Any static server from
the repository root works:

```bash
python -m http.server 8000
```

Then open `http://localhost:8000/examples/wasm/scan.html`.

## Demos

| File | What it does |
| --- | --- |
| `scan.html` | Builds a screener from the shared spec (`close > 10`), scans the two-symbol inline universe (`AAA` at 5, `BBB` at 15) and renders the matches with the expression values that explain them, plus the raw `ScanReport` JSON. The page counterpart of `examples/node/scan.js`. |

## See also

- [examples/README.md](../README.md) — the same scan in every other language.
- [bindings/wasm/README.md](../../bindings/wasm/README.md) — the module's API and
  what the sequential build does and does not carry.
