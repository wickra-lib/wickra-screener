"use strict";

// The WASM binding's only test.
//
// Until now the `wasm` CI job ran `wasm-pack build --target web` and stopped.
// The binding was compiled on every push and executed on none of them, in a
// repository whose README advertises a live in-browser demo and whose core
// documents the sequential WASM path as byte-identical to the parallel one.
// Nothing checked either claim.
//
// This runs the same two checks every other binding makes:
//
//   golden    — each committed spec scanned over the committed dataset must
//               equal golden/expected/<spec>.json byte-for-byte, which is what
//               makes "byte-identical across ten languages" a fact rather than
//               a sentence.
//   streaming — feeding candle by candle and evaluating must equal one batch
//               scan, driven through the same JSON command boundary.
//
// It runs under `--target nodejs`, so no browser and no headless runner are
// needed. The published artifact stays the `web` build; this one exists to be
// executed.

const { test } = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const { Screener, version } = require("../pkg-node/wickra_screener_wasm.js");

const STREAMING_SPECS = [
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

const golden = findGolden();

test("the module exposes its version", () => {
  assert.ok(golden, "golden fixtures not found; the checks below would test nothing");
  assert.match(version(), /^\d+\.\d+\.\d+/);
});

test("golden scans are byte-identical", () => {
  assert.ok(golden, "golden fixtures not found");
  const datasets = {
    "data.json": fs.readFileSync(path.join(golden, "data.json"), "utf8"),
    "data-feeds.json": fs.readFileSync(path.join(golden, "data-feeds.json"), "utf8"),
  };
  const specDir = path.join(golden, "specs");
  const files = fs.readdirSync(specDir).filter((f) => f.endsWith(".json"));
  assert.ok(files.length > 0, "the spec directory is empty");

  for (const file of files) {
    const spec = fs.readFileSync(path.join(specDir, file), "utf8");
    const expected = fs
      .readFileSync(path.join(golden, "expected", file), "utf8")
      .trim();
    const dataset =
      datasets[file.startsWith("feeds_") ? "data-feeds.json" : "data.json"];
    const screener = new Screener(spec);
    const response = screener.command(
      JSON.stringify({ cmd: "scan", data: JSON.parse(dataset) }),
    );
    assert.strictEqual(response.trim(), expected, `mismatch for ${file}`);
  }
});

test("streaming equals batch over the candle-only specs", () => {
  assert.ok(golden, "golden fixtures not found");
  const data = JSON.parse(
    fs.readFileSync(path.join(golden, "data.json"), "utf8"),
  );

  let compared = 0;
  for (const name of STREAMING_SPECS) {
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
  assert.strictEqual(compared, STREAMING_SPECS.length, "not every spec was compared");
});
