# Wickra Screener — Node.js

Node.js bindings for the `wickra-screener` data-driven core (napi-rs). Build a
`Screener` from a spec JSON, drive it with command JSON, read back scan
reports — the same protocol as the native CLI and every other binding.

## Install

```bash
npm install wickra-screener
```

## Usage

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

## API

| Member | Description |
|--------|-------------|
| `new Screener(specJson)` | Build a screener from a spec JSON (throws on an invalid spec). |
| `screener.command(cmdJson)` | Apply a command JSON, return the response JSON. Commands: `set_spec`, `feed`, `feed_batch`, `evaluate`, `scan`, `reset`, `version`. |
| `screener.version()` / `version()` | The library version. |

## Build from source

```bash
npm install
npm run build   # napi build --platform --release → index.js + index.d.ts + *.node
npm test        # node --test
```

## License

`MIT OR Apache-2.0`.
