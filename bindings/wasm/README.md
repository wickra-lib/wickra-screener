# Wickra Screener — WASM

WebAssembly bindings for the `wickra-screener` data-driven core (wasm-bindgen).
Build a `Screener` from a spec JSON, drive it with command JSON, read back scan
reports — the same protocol as every other binding, running in the browser.

The core is built with `--no-default-features`, so the scan is **sequential**
(no rayon thread pool in the browser sandbox) and byte-identical to the native
parallel scan.

## Build

```bash
wasm-pack build --target web
```

This emits `pkg/` with the `.wasm` module and JS glue.

## Usage

```js
import init, { Screener, version } from "./pkg/wickra_screener_wasm.js";

await init();

const spec = JSON.stringify({
  universe: ["AAA", "BBB"],
  condition: {
    type: "cmp",
    left: { kind: "price", field: "close" },
    op: "gt",
    right: { kind: "const", value: 10.0 },
  },
});

const screener = new Screener(spec);

const candle = (close) => ({
  time: 1, open: close, high: close, low: close, close, volume: 1.0,
});

const report = JSON.parse(screener.command(JSON.stringify({
  cmd: "scan",
  data: { AAA: [candle(5.0)], BBB: [candle(15.0)] },
})));

console.log(report.matches.map((m) => m.symbol)); // [ 'BBB' ]
console.log(version());
```

## API

| Member | Description |
|--------|-------------|
| `new Screener(specJson)` | Build a screener from a spec JSON (throws on an invalid spec). |
| `screener.command(cmdJson)` | Apply a command JSON, return the response JSON. |
| `screener.version()` / `version()` | The library version. |

## License

`MIT OR Apache-2.0`.
