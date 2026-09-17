<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![npm](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/npm.svg)](https://www.npmjs.com/package/wickra-screener)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — Node.js

---

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators — for Node.js. `npm install wickra-screener` — prebuilt native binary, no system dependencies.**

Node.js bindings for the `wickra-screener` data-driven core (napi-rs). Build a
`Screener` from a spec JSON, drive it with command JSON, read back scan
reports — the same protocol as the native CLI and every other binding.

## Install

```bash
npm install wickra-screener
```

The native addon ships as a prebuilt binary per platform (Linux, macOS,
Windows — x64 and arm64), selected automatically through optional
dependencies. There is nothing to compile.

### Building from this repository (contributors)

```bash
npm install
npm run build   # napi build --platform --release → index.js + index.d.ts + *.node
npm test        # node --test
```

## Quick start

```js
const { Screener, version } = require("wickra-screener");

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
| `screener.command(cmdJson)` | Apply a command JSON, return the response JSON. Commands: `set_spec`, `feed`, `feed_batch`, `evaluate`, `scan`, `reset`, `version`. |
| `screener.version()` / `version()` | The library version. |

## Benchmark

Every binding forwards to the same data-driven Rust core, so what this one adds is
the call overhead of napi-rs, not a different result. The core's throughput is
measured by the repository's benchmark suite and the nightly `bench.yml` run; the
numbers, the machine and how to reproduce them are in the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-screener/blob/main/BENCHMARKS.md).

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-screener>
- **Docs** (guides, spec reference, cookbook): <https://screener.wickra.org>
- **Runnable example:** [`examples/node/`](https://github.com/wickra-lib/wickra-screener/tree/main/examples/node)

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
