#!/usr/bin/env bash
# Fail if the committed C ABI header has drifted from the cbindgen output, or if
# the vendored Go copy is stale — so the hand-usable header can never fall out of
# sync with the Rust surface, and every C-capable language sees the same one.
#
# The cbindgen check skips (exit 0) when cbindgen is not installed, so a checkout
# without the tool does not spuriously fail; CI installs cbindgen and runs it. The
# Go-copy check is a plain diff and always runs.
set -euo pipefail

c_header="bindings/c/include/wickra_screener.h"
go_header="bindings/go/include/wickra_screener.h"

# 1) The committed Go header must be a byte-for-byte copy of the C one.
if ! diff -u "$c_header" "$go_header"; then
    echo ""
    echo "ERROR: $go_header is stale."
    echo "Copy the C header over it:"
    echo "  cp $c_header $go_header"
    exit 1
fi
echo "Go C ABI header copy is in sync."

# 2) The committed C header must match fresh cbindgen output.
if ! command -v cbindgen >/dev/null 2>&1; then
    echo "cbindgen not installed — skipping C ABI header sync check."
    exit 0
fi

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

cbindgen --config bindings/c/cbindgen.toml --crate wickra-screener-c --output "$tmp" --quiet

if ! diff -u "$c_header" "$tmp"; then
    echo ""
    echo "ERROR: $c_header is out of sync with cbindgen output."
    echo "Regenerate it with:"
    echo "  cbindgen --config bindings/c/cbindgen.toml --crate wickra-screener-c --output $c_header"
    exit 1
fi

echo "C ABI header is in sync with cbindgen."
