"use strict";

// Parity guard: the Node binding must expose the full public surface of the
// screener, so an export dropped in a refactor fails loudly here (mirrors the
// completeness check in the main wickra repo).

const { test } = require("node:test");
const assert = require("node:assert");
const wickra = require("../index.js");

test("module exposes Screener and version", () => {
  assert.strictEqual(typeof wickra.Screener, "function");
  assert.strictEqual(typeof wickra.version, "function");
});

test("Screener exposes command and version", () => {
  for (const name of ["command", "version"]) {
    assert.strictEqual(
      typeof wickra.Screener.prototype[name],
      "function",
      `Screener is missing ${name}`,
    );
  }
});
