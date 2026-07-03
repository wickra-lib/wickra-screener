# Wickra Screener — Python

Python bindings for [wickra-screener](https://github.com/wickra-lib/wickra-screener),
the data-driven multi-symbol scan core. Build a `Screener` from a spec JSON,
drive it with command JSONs, and read back scan reports — the same command
protocol every language binding speaks.

## Install

```sh
pip install wickra-screener
```

## Usage

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

## API

| Method | Description |
|--------|-------------|
| `Screener(spec_json)` | Build a screener from a spec JSON (raises `ValueError` if invalid). |
| `screener.command(cmd_json) -> str` | Apply a command JSON, return the response JSON. Commands: `set_spec`, `feed`, `feed_batch`, `evaluate`, `scan`, `reset`, `version`. |
| `Screener.version() -> str` | The library version. |

## Build from source

```sh
maturin develop --release
pytest -q
```

## License

`MIT OR Apache-2.0`.
