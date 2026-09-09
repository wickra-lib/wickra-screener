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
UV_VERSION="0.12.12"
# sha256 of the release archive for each supported host, taken from
# https://github.com/astral-sh/uv/releases/download/<version>/<archive>.sha256
uv_sha256() {
  case "$1" in
    x86_64-unknown-linux-gnu)  echo "ab9b309d4586403f024e100abaceb396616e178a553e2500c36087d180f09509" ;;
    aarch64-unknown-linux-gnu) echo "fe08db50cc1b56cd1da7801065ed1103d27ed3f9571cd122386cfc7faf1b8df5" ;;
    aarch64-apple-darwin)      echo "46740540b63fdee9a6cb2e19baf3f1f475b850c440a33e63455087a6871263f1" ;;
    x86_64-apple-darwin)       echo "0dc8cd6c961582b0d140b5398f96b23502885277fb3464241456a2435e460dfa" ;;
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
