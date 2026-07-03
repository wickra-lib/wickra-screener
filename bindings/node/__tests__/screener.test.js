"use strict";

const { test } = require("node:test");
const assert = require("node:assert");
const { Screener, version } = require("../index.js");

const SPEC = JSON.stringify({
  universe: ["AAA", "BBB"],
  condition: {
    type: "cmp",
    left: { kind: "price", field: "close" },
    op: "gt",
    right: { kind: "const", value: 10.0 },
  },
});

function candle(close) {
  return { time: 1, open: close, high: close, low: close, close, volume: 1.0 };
}

test("scan roundtrip returns only the matching symbol", () => {
  const screener = new Screener(SPEC);
  const response = screener.command(
    JSON.stringify({
      cmd: "scan",
      data: { AAA: [candle(5.0)], BBB: [candle(15.0)] },
    }),
  );
  const report = JSON.parse(response);
  assert.strictEqual(report.scanned, 2);
  assert.deepStrictEqual(
    report.matches.map((m) => m.symbol),
    ["BBB"],
  );
});

test("version matches the module-level function", () => {
  const screener = new Screener(SPEC);
  assert.strictEqual(screener.version(), version());
});

test("a malformed spec throws", () => {
  assert.throws(() => new Screener("not json"));
});
