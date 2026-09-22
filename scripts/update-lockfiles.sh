#!/usr/bin/env bash
#
# Regenerate every committed lockfile in the repository:
#   - Rust:   Cargo.lock                          (cargo update)
#   - Node:   bindings/node/package-lock.json      (npm install --package-lock-only)
#             examples/node/package-lock.json
#   - Python: .github/requirements/*.txt           (uv pip compile --generate-hashes)
#
# Run from anywhere; the script cd's to the repository root itself:
#
#     ./scripts/update-lockfiles.sh
#
# fuzz/Cargo.lock is deliberately not here: the fuzz crate is a detached
# workspace and its lock is gitignored, so there is nothing committed to refresh.
#
# The Python locks are hash-pinned (OpenSSF Scorecard PinnedDependencies) and
# generated with uv rather than pip-tools because uv can resolve a *target*
# Python version's full transitive closure -- with hashes -- without that
# interpreter being installed locally. That is what makes the 3.9 lock possible
# on a machine running 3.11: pytest >= 9.0 and iniconfig >= 2.3 both declare
# requires-python >= 3.10, so the 3.9 matrix row needs its own resolution.
#
# If uv is not on PATH the script stops and tells you to install it;
# WICKRA_BOOTSTRAP_UV=1 opts into fetching one pinned, checksum-verified release
# into a temporary directory instead. It is opt-in because the alternative --
# piping https://astral.sh/uv/install.sh into a shell -- runs whatever is behind
# that URL at that moment, with your privileges, on the machine of everyone who
# regenerates a lockfile.
#
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "==> Rust (Cargo.lock)"
cargo update

echo "==> Node (bindings/node, examples/node)"
(cd bindings/node && npm install --package-lock-only --no-audit --no-fund)
(cd examples/node && npm install --package-lock-only --no-audit --no-fund)

echo "==> Python (.github/requirements/*.txt via uv)"
UV_VERSION="0.12.18"
# sha256 of the release archive for each supported host, taken from
# https://github.com/astral-sh/uv/releases/download/<version>/<archive>.sha256
uv_sha256() {
  case "$1" in
    x86_64-unknown-linux-gnu)  echo "89eadd7c76fc063887959510d5ba0ab1264dfd5f1143b925ddb73021a40acf16" ;;
    aarch64-unknown-linux-gnu) echo "afb6291f3f0a6b4521fc67b947822506c41dde5b60d2189dd8f3695b2ac8c9e7" ;;
    aarch64-apple-darwin)      echo "cf40e0c6a202190ccd9e0406dcfdd5b2d6668a9a5c779b17948963df32aafe5b" ;;
    x86_64-apple-darwin)       echo "2e4108f5395397c8bc5d43bf83d3bdbb2d0e92b90d0efa607756be704905fa33" ;;
    *)                         echo "" ;;
  esac
}

if ! command -v uv >/dev/null 2>&1; then
  if [ "${WICKRA_BOOTSTRAP_UV:-0}" != "1" ]; then
    echo "    uv is not on PATH." >&2
    echo "    Install it (https://docs.astral.sh/uv/getting-started/installation/)," >&2
    echo "    or re-run with WICKRA_BOOTSTRAP_UV=1 to fetch uv ${UV_VERSION} here." >&2
    exit 1
  fi

  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)   uv_target="x86_64-unknown-linux-gnu" ;;
    Linux-aarch64)  uv_target="aarch64-unknown-linux-gnu" ;;
    Darwin-arm64)   uv_target="aarch64-apple-darwin" ;;
    Darwin-x86_64)  uv_target="x86_64-apple-darwin" ;;
    *)
      echo "    No pinned uv build for $(uname -s)-$(uname -m); install uv yourself." >&2
      exit 1
      ;;
  esac
  uv_expected="$(uv_sha256 "$uv_target")"

  echo "    bootstrapping uv ${UV_VERSION} (${uv_target})..."
  uv_dir="$(mktemp -d)"
  trap 'rm -rf "$uv_dir"' EXIT
  uv_archive="uv-${uv_target}.tar.gz"
  curl -fsSL --retry 5 --retry-all-errors -o "${uv_dir}/${uv_archive}" \
    "https://github.com/astral-sh/uv/releases/download/${UV_VERSION}/${uv_archive}"
  echo "${uv_expected}  ${uv_dir}/${uv_archive}" | sha256sum -c -
  tar -xzf "${uv_dir}/${uv_archive}" -C "$uv_dir" --strip-components=1
  export PATH="${uv_dir}:$PATH"
fi

req=".github/requirements"
cc="./scripts/update-lockfiles.sh"
uv pip compile --quiet --python-version 3.9  --generate-hashes --custom-compile-command "$cc" "$req/ci-dev-py39.in" -o "$req/ci-dev-py39.txt"
uv pip compile --quiet --python-version 3.11 --generate-hashes --custom-compile-command "$cc" "$req/ci-dev-py3.in"  -o "$req/ci-dev-py3.txt"

echo "==> Done. Review 'git diff' before committing."
