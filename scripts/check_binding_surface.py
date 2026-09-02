#!/usr/bin/env python3
"""Assert that every binding exposes the surface the C ABI declares.

Ten language reaches sit on one C ABI. Each has its own test suite and each is
written separately, so a reach that falls behind fails nowhere: the golden corpus
compares *values*, and a binding that never grew a method simply has no test to
run. A language that silently lost `version` would still pass every golden test
it has, because no golden fixture asks for a version.

The header is the source of truth. Every export in it is a promise the bindings
make, so this reads `wickra_screener_<name>` out of
`bindings/c/include/wickra_screener.h` and checks each language's public surface
for that name, spelled the way that language spells it.

One export is deliberately not part of the language surface:

  free    a memory-management detail of the ABI. Go spells releasing a handle
          `Close`, C# `Dispose`, Java `close`, R registers a finalizer, and
          Python, Node and WASM let the runtime do it. Demanding one spelling
          across all of them would report idiom as drift, so the release path is
          each language's own contract and is not checked here.

Extras run the other way: a binding method with no export behind it is reported
as a note, not a failure. That is how a language gets *ahead* of the ABI, which
is worth seeing but is not drift in the dangerous direction.

Declarations are matched, not occurrences. A doc comment naming the function, or
an internal call site, must not let a renamed export pass unnoticed.

Run from the repository root:  python scripts/check_binding_surface.py
"""

from __future__ import annotations

import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
HEADER = os.path.join(ROOT, "bindings", "c", "include", "wickra_screener.h")

# Exports that are ABI plumbing rather than a promise to callers.
ABI_ONLY = {"free"}

EXPORT = re.compile(r"\bwickra_screener_([a-z0-9_]+)\s*\(")

# How each language spells the three promised exports, and where its public
# surface lives. `new` is a constructor in every language but Go and R, so its
# pattern is the constructor rather than a free function -- a class API is the
# capability, not a hole.
#
# Python is checked in its PyO3 source rather than in `__init__.py`: the package
# file only re-exports `Screener` from the compiled module, so the methods
# themselves are declared in Rust. Checking the re-export alone would pass a
# binding that had lost `command`.
BINDINGS: dict[str, tuple[list[str], dict[str, str]]] = {
    "python": (
        ["bindings/python/src/lib.rs", "bindings/python/python/wickra_screener/__init__.py"],
        {
            "new": r"(?m)^\s*fn new\s*\(",
            "command": r"(?m)^\s*fn command\s*\(",
            "version": r"(?m)^\s*fn version\s*\(",
        },
    ),
    "node": (
        ["bindings/node/index.d.ts"],
        {
            "new": r"(?m)^\s*constructor\s*\(",
            "command": r"(?m)^\s*command\s*\(",
            "version": r"(?m)^\s*(?:export declare function )?version\s*\(",
        },
    ),
    "wasm": (
        ["bindings/wasm/src/lib.rs"],
        {
            "new": r"(?m)^\s*pub fn new\s*\(",
            "command": r"(?m)^\s*pub fn command\s*\(",
            # The instance method is renamed by attribute, so the promise is kept
            # by `js_name = version` as much as by a free `pub fn version`.
            "version": r"(?m)js_name = version|^\s*pub fn version\s*\(",
        },
    ),
    "csharp": (
        ["bindings/csharp/WickraScreener/Screener.cs"],
        {
            "new": r"(?m)^\s*public Screener\s*\(",
            "command": r"(?m)^\s*public [^\n]*\bCommand\s*\(",
            "version": r"(?m)^\s*public static [^\n]*\bVersion\s*\(",
        },
    ),
    "go": (
        ["bindings/go/wickra.go"],
        {
            "new": r"(?m)^func New\s*\(",
            "command": r"(?m)^func \([^)]*\*Screener\) Command\s*\(",
            "version": r"(?m)^func Version\s*\(",
        },
    ),
    "java": (
        ["bindings/java/src/main/java/org/wickra/screener/Screener.java"],
        {
            "new": r"(?m)^\s*public Screener\s*\(",
            "command": r"(?m)^\s*public [^\n]*\bcommand\s*\(",
            "version": r"(?m)^\s*public static [^\n]*\bversion\s*\(",
        },
    ),
    "r": (
        ["bindings/r/R/screener.R", "bindings/r/NAMESPACE"],
        {
            "new": r"(?m)^wkscreen_new\s*<-\s*function",
            "command": r"(?m)^wkscreen_command\s*<-\s*function",
            "version": r"(?m)^wkscreen_version\s*<-\s*function",
        },
    ),
    # The C++ hull is a separate reach over the same header, and it is the one
    # the C example is written against, so it carries the same promise.
    "cpp": (
        ["bindings/c/include/wickra_screener.hpp"],
        {
            "new": r"\bScreener\s*\(\s*(?:const\s+)?std::string",
            "command": r"\bcommand\s*\(",
            "version": r"\bversion\s*\(",
        },
    ),
}


def read(paths: list[str]) -> str | None:
    out = []
    for rel in paths:
        path = os.path.join(ROOT, rel)
        if not os.path.isfile(path):
            return None
        with open(path, encoding="utf-8") as handle:
            out.append(handle.read())
    return "\n".join(out)


def main() -> int:
    if not os.path.isfile(HEADER):
        print(f"header not found: {HEADER}", file=sys.stderr)
        return 1
    with open(HEADER, encoding="utf-8") as handle:
        exports = sorted(set(EXPORT.findall(handle.read())))
    if not exports:
        print(f"no wickra_screener_* exports found in {HEADER}", file=sys.stderr)
        return 1

    promised = [e for e in exports if e not in ABI_ONLY]
    print(f"header declares {len(exports)} exports; "
          f"{len(promised)} are a promise to callers: {', '.join(promised)}\n")

    failures: list[str] = []
    notes: list[str] = []
    for lang, (paths, patterns) in sorted(BINDINGS.items()):
        text = read(paths)
        if text is None:
            failures.append(f"{lang}: surface file missing ({', '.join(paths)})")
            print(f"  {lang:<8} surface file missing")
            continue

        unlisted = [e for e in promised if e not in patterns]
        if unlisted:
            # A pattern table that has fallen behind the header is the same
            # silence this script exists to remove, so it fails rather than
            # skipping the export.
            failures.append(
                f"{lang}: header exports {', '.join(unlisted)} but this script "
                f"declares no pattern for them"
            )

        missing = [e for e in promised
                   if e in patterns and re.search(patterns[e], text) is None]
        extra = [name for name in patterns if name not in promised]
        if missing:
            failures.append(f"{lang}: does not declare {', '.join(missing)}")
        if extra:
            notes.append(f"{lang}: declares {', '.join(extra)} with no export behind it")
        state = "complete" if not missing and not unlisted else f"missing {len(missing) + len(unlisted)}"
        print(f"  {lang:<8} {state}")

    if notes:
        print("\nnotes (a binding ahead of the ABI, not drift):")
        for note in notes:
            print(f"  {note}")

    if failures:
        print("\nbindings have fallen behind the C ABI:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1

    print(f"\nall {len(BINDINGS)} reaches carry the {len(promised)} promised exports.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
