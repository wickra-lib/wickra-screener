"use strict";

// Streaming equals batch, driven through the JSON command boundary.
//
// screener-core proves this in Rust (crates/screener-core/tests/streaming_eq_batch.rs),
// but that says nothing about the boundary each language actually crosses. A
// binding reaches the core through `command`, and the golden corpus only ever
// sends `{"cmd":"scan"}` — so `feed` and `evaluate` were exercised in no
// language at all. A binding that mis-serialised a candle on the feed path, or
// dropped a field from the envelope, had no test to fail.
//
// Batch: one `scan` over the whole dataset.
// Streaming: `feed` each candle of each symbol in turn, then `evaluate`.
// Both return the core's compact JSON verbatim, so byte equality is the check.
//
// Only the candle-only specs are used. A `feeds_*` spec needs side feeds the
// streaming envelope carries per bar, and `derived_breadth` needs the market
// panel, which a streaming screener cannot derive: it sees one symbol's bar at a
// time and cannot know which other symbols print at that timestamp. That is a
// documented limitation (docs/CROSS_SECTION.md), not something this should hide.

const { test } = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const { Screener } = require("../index.js");

const SPECS = [
  "momentum",
  "mean_reversion",
  "cross_section_rank",
  "breadth",
  "crossover",
  "compound",
];

function findGolden() {
  let dir = __dirname;
  for (let i = 0; i < 8; i++) {
    const g = path.join(dir, "golden");
    if (fs.existsSync(path.join(g, "specs"))) {
      return g;
    }
    dir = path.dirname(dir);
  }
  return null;
}

test("streaming equals batch over the candle-only specs", (t) => {
  const golden = findGolden();
  if (!golden) {
    t.skip("golden fixtures not present yet");
    return;
  }
  const data = JSON.parse(
    fs.readFileSync(path.join(golden, "data.json"), "utf8"),
  );

  let compared = 0;
  for (const name of SPECS) {
    const specPath = path.join(golden, "specs", `${name}.json`);
    assert.ok(fs.existsSync(specPath), `spec ${name} is missing from the corpus`);
    const spec = fs.readFileSync(specPath, "utf8");

    const batch = new Screener(spec)
      .command(JSON.stringify({ cmd: "scan", data }))
      .trim();

    const streaming = new Screener(spec);
    for (const symbol of Object.keys(data).sort()) {
      for (const candle of data[symbol]) {
        streaming.command(JSON.stringify({ cmd: "feed", symbol, candle }));
      }
    }
    const streamed = streaming.command(JSON.stringify({ cmd: "evaluate" })).trim();

    assert.strictEqual(streamed, batch, `streaming != batch for spec ${name}`);
    compared += 1;
  }
  // A loop that compared nothing passes; say how many it actually did.
  assert.strictEqual(compared, SPECS.length, "not every spec was compared");
});

test("reset returns a screener to its pre-feed state", (t) => {
  const golden = findGolden();
  if (!golden) {
    t.skip("golden fixtures not present yet");
    return;
  }
  const data = JSON.parse(
    fs.readFileSync(path.join(golden, "data.json"), "utf8"),
  );
  const spec = fs.readFileSync(
    path.join(golden, "specs", "momentum.json"),
    "utf8",
  );

  const screener = new Screener(spec);
  const empty = screener.command(JSON.stringify({ cmd: "evaluate" }));

  for (const symbol of Object.keys(data).sort()) {
    for (const candle of data[symbol]) {
      screener.command(JSON.stringify({ cmd: "feed", symbol, candle }));
    }
  }
  assert.notStrictEqual(
    screener.command(JSON.stringify({ cmd: "evaluate" })),
    empty,
    "feeding the whole universe changed nothing",
  );

  screener.command(JSON.stringify({ cmd: "reset" }));
  assert.strictEqual(
    screener.command(JSON.stringify({ cmd: "evaluate" })),
    empty,
    "reset did not return the screener to its pre-feed state",
  );
});
