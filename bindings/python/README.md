<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514-7" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![PyPI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/pypi.svg)](https://pypi.org/project/wickra-screener/)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — Python

---

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators — for Python. `pip install wickra-screener` — prebuilt wheels for Linux, macOS and Windows, nothing to compile.**

Python bindings for [wickra-screener](https://github.com/wickra-lib/wickra-screener),
the data-driven multi-symbol scan core. Build a `Screener` from a spec JSON,
drive it with command JSONs, and read back scan reports — the same command
protocol every language binding speaks.

## Install

```bash
pip install wickra-screener
```

Pre-built wheels ship for Linux, macOS and Windows — there is nothing to
compile and no C library to track down.

### Building from this repository (contributors)

```sh
maturin develop --release
pytest -q
```

## Quick start

```python
import json
from wickra_screener import Screener

spec = json.dumps({
    "universe": ["AAA", "BBB"],
    "condition": {
        "type": "cmp",
        "left": {"kind": "price", "field": "close"},
        "op": "gt",
        "right": {"kind": "const", "value": 10.0},
    },
})

screener = Screener(spec)

def candle(close):
    return {"time": 1, "open": close, "high": close,
            "low": close, "close": close, "volume": 1.0}

response = screener.command(json.dumps({
    "cmd": "scan",
    "data": {"AAA": [candle(5.0)], "BBB": [candle(15.0)]},
}))

report = json.loads(response)
print([m["symbol"] for m in report["matches"]])  # ['BBB']
```

### API

| Method | Description |
|--------|-------------|
| `Screener(spec_json)` | Build a screener from a spec JSON (raises `ValueError` if invalid). |
| `screener.command(cmd_json) -> str` | Apply a command JSON, return the response JSON. Commands: `set_spec`, `feed`, `feed_batch`, `evaluate`, `scan`, `reset`, `version`. |
| `Screener.version() -> str` | The library version. |

## Benchmark

Every binding forwards to the same data-driven Rust core, so what this one adds is
the call overhead of PyO3, not a different result. The core's throughput is
measured by the repository's benchmark suite and the nightly `bench.yml` run; the
numbers, the machine and how to reproduce them are in the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-screener/blob/main/BENCHMARKS.md).

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-screener>
- **Docs** (guides, spec reference, cookbook): <https://screener.wickra.org>
- **Runnable example:** [`examples/python/`](https://github.com/wickra-lib/wickra-screener/tree/main/examples/python)

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
