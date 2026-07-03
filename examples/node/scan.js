// A runnable Node.js example: scan a small universe through the binding.
//
//   ( cd bindings/node && npm install && npm run build )
//   ( cd examples/node && npm install && node scan.js )

"use strict";

const { Screener, version } = require("wickra-screener");

const SPEC = JSON.stringify({
  universe: ["AAA", "BBB"],
  condition: {
    type: "cmp",
    left: { kind: "price", field: "close" },
    op: "gt",
    right: { kind: "const", value: 10.0 },
  },
});

const candle = (close) => ({
  time: 1,
  open: close,
  high: close,
  low: close,
  close,
  volume: 1.0,
});

const screener = new Screener(SPEC);
const response = screener.command(
  JSON.stringify({
    cmd: "scan",
    data: { AAA: [candle(5.0)], BBB: [candle(15.0)] },
  }),
);
const report = JSON.parse(response);

console.log("wickra-screener", version());
console.log(response);
for (const match of report.matches) {
  console.log(`  matched: ${match.symbol}`);
}
