#!/usr/bin/env python3
"""Binding READMEs must not use repository-relative links.

Each binding README is, or is one workflow line away from being, the long
description of a published package: PyPI renders the Python one, NuGet the C#
one, pkg.go.dev the Go one, r-universe the R one. A link like
`../../docs/COOKBOOK.md` resolves on GitHub and nowhere else -- on a registry
page it is simply broken, and nothing in the build says so, because the file it
points at does exist in the repository.

So the rule is: anything that ships as package metadata links absolutely. The
repository's own README is exempt and deliberately keeps relative links -- it is
read on GitHub far more than anywhere else, and that is the convention the main
wickra repository uses too.

The READMEs are found by walking `bindings/`, not by a fixed-depth glob. The
sibling repositories keep every binding README at `bindings/<lang>/README.md`,
but this one ships the C# text from `bindings/csharp/WickraScreener/README.md`,
beside the csproj that names it in `<PackageReadmeFile>`. A `bindings/*/README.md`
glob skips exactly that file -- the NuGet case this script's own docstring
promises to cover -- and reports success while doing it.

Run from the repository root:  python scripts/check_readme_links.py
"""

from __future__ import annotations

import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

# Build output and tool caches carry READMEs of their own (`.pytest_cache` writes
# one explaining itself). None of them ship, so none of them are checked.
SKIP_DIRS = {
    ".pytest_cache",
    "__pycache__",
    ".venv",
    "bin",
    "build",
    "dist",
    "node_modules",
    "obj",
    "pkg",
    "target",
}

# A markdown link target that is neither absolute nor a same-page anchor. Also
# catches HTML `src=`/`href=` attributes, which the banner markup uses.
LINK = re.compile(r"\]\(\s*(?!https?://|#|mailto:)([^)\s]+)")
ATTR = re.compile(r"(?:src|href)=\"(?!https?://|#|mailto:)([^\"]+)\"")


def relative_targets(text: str) -> list[str]:
    return [m.group(1) for m in LINK.finditer(text)] + [m.group(1) for m in ATTR.finditer(text)]


def shipping_readmes() -> list[str]:
    """Every README under `bindings/` that is not build output."""
    found = []
    for current, dirs, files in os.walk(os.path.join(ROOT, "bindings")):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS and not d.startswith(".")]
        if "README.md" in files:
            found.append(os.path.join(current, "README.md"))
    return sorted(found)


def main() -> int:
    paths = shipping_readmes()
    if not paths:
        print("no binding READMEs found", file=sys.stderr)
        return 1

    failures = []
    width = max(len(os.path.relpath(p, ROOT)) for p in paths) + 2
    for path in paths:
        rel = os.path.relpath(path, ROOT).replace(os.sep, "/")
        with open(path, encoding="utf-8") as handle:
            found = relative_targets(handle.read())
        if found:
            failures.append(f"{rel}: {', '.join(sorted(set(found)))}")
        state = f"relative links: {len(found)}" if found else "all links absolute"
        print(f"  {rel:<{width}} {state}")

    if failures:
        print(
            "\nthese READMEs ship as package long descriptions, where a relative "
            "link is dead:",
            file=sys.stderr,
        )
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        print(
            "\nuse https://github.com/wickra-lib/wickra-screener/blob/main/<path> "
            "instead.",
            file=sys.stderr,
        )
        return 1

    print(f"\nall {len(paths)} binding READMEs link absolutely.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
