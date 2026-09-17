<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![npm](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/npm.svg)](https://www.npmjs.com/package/wickra-screener-wasm)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — WASM

---

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators — for WASM. `npm install wickra-screener-wasm` — pure WebAssembly, runs anywhere a modern JS engine does.**

WASM bindings for the `wickra-screener` data-driven core, compiled to WebAssembly
with wasm-bindgen.
Build a `Screener` from a spec JSON, drive it with command JSON, read back scan
reports — the same protocol as every other binding, running in the browser.

The core is built with `--no-default-features`, so the scan is **sequential**
(no rayon thread pool in the browser sandbox) and byte-identical to the native
parallel scan.

## Install

```bash
npm install wickra-screener-wasm
```

### Building from this repository (contributors)

```bash
wasm-pack build --target web
```

This emits `pkg/` with the `.wasm` module and JS glue.

## Quick start

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

### API

| Member | Description |
|--------|-------------|
| `new Screener(specJson)` | Build a screener from a spec JSON (throws on an invalid spec). |
| `screener.command(cmdJson)` | Apply a command JSON, return the response JSON. |
| `screener.version()` / `version()` | The library version. |

## Benchmark

Every binding forwards to the same data-driven Rust core, so what this one adds is
the call overhead of wasm-bindgen, not a different result. The core's throughput is
measured by the repository's benchmark suite and the nightly `bench.yml` run; the
numbers, the machine and how to reproduce them are in the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-screener/blob/main/BENCHMARKS.md).

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-screener>
- **Docs** (guides, spec reference, cookbook): <https://screener.wickra.org>
- **Runnable example:** [`examples/wasm/`](https://github.com/wickra-lib/wickra-screener/tree/main/examples/wasm)

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
