<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514-7" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![GitHub release](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/release.svg)](https://github.com/wickra-lib/wickra-screener/releases/latest)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — C / C++

---

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators — for C / C++. `cargo build -p wickra-screener-c --release` — a prebuilt shared/static library plus a generated `wickra_screener.h`, no system dependencies.**

The C ABI is the hub every C-capable language (C, C++, C#, Go, Java, R) links
against. It exposes `wickra-screener-core` as a tiny, JSON-shaped surface built as both
a `cdylib` (dynamic library) and a `staticlib`.

## Install

Grab the prebuilt header + library for your platform from the
[GitHub releases](https://github.com/wickra-lib/wickra-screener/releases) — each archive
has `wickra_screener.h`, the C++ wrapper where the binding ships one, and the shared/static
library — or build from source:

```bash
cargo build -p wickra-screener-c --release
# -> target/release/libwickra_screener.{so,dylib} or wickra_screener.dll (+ import lib) + a staticlib
```

Then compile against the header and link the library.

## Quick start

[`examples/c/scan.c`](https://github.com/wickra-lib/wickra-screener/blob/main/examples/c/scan.c) is the runnable example the CI smoke job executes; in full:

```c
/* A minimal C example: run a scan through the wickra-screener C ABI. */
#include <stdio.h>
#include <stdlib.h>

#include "wickra_screener.h"

static const char *SPEC =
    "{\"universe\":[\"AAA\",\"BBB\"],\"condition\":{\"type\":\"cmp\","
    "\"left\":{\"kind\":\"price\",\"field\":\"close\"},\"op\":\"gt\","
    "\"right\":{\"kind\":\"const\",\"value\":10.0}}}";

static const char *CMD =
    "{\"cmd\":\"scan\",\"data\":{"
    "\"AAA\":[{\"time\":1,\"open\":5,\"high\":5,\"low\":5,\"close\":5,\"volume\":1}],"
    "\"BBB\":[{\"time\":1,\"open\":15,\"high\":15,\"low\":15,\"close\":15,\"volume\":1}]}}";

int main(void) {
    WickraScreener *screener = wickra_screener_new(SPEC);
    if (!screener) {
        fprintf(stderr, "failed to build screener\n");
        return 1;
    }

    /* Length-out protocol: learn the length, then read into a caller buffer. */
    int len = wickra_screener_command(screener, CMD, NULL, 0);
    if (len < 0) {
        fprintf(stderr, "command failed: code %d\n", len);
        wickra_screener_free(screener);
        return 1;
    }
    char *buf = (char *)malloc((size_t)len + 1);
    if (!buf) {
        wickra_screener_free(screener);
        return 1;
    }
    wickra_screener_command(screener, CMD, buf, (size_t)len + 1);

    printf("wickra-screener %s\n", wickra_screener_version());
    printf("scan: %s\n", buf);

    free(buf);
    wickra_screener_free(screener);
    return 0;
}
```

### Surface

```c
#include "wickra_screener.h"

WickraScreener *wickra_screener_new(const char *spec_json);
void            wickra_screener_free(WickraScreener *handle);
int32_t         wickra_screener_command(WickraScreener *handle,
                                        const char *cmd_json,
                                        char *out, size_t cap);
const char     *wickra_screener_version(void);
```

- **`wickra_screener_new`** builds a screener from a spec JSON. Returns `NULL`
  if the argument is null, not UTF-8, or not a valid spec.
- **`wickra_screener_free`** destroys a handle (null is a no-op).
- **`wickra_screener_command`** applies a command JSON and writes the response
  JSON into the caller's buffer using a length-out protocol (below).
- **`wickra_screener_version`** returns a static, NUL-terminated version string
  (do not free).

### Command / response protocol

Everything after construction goes through `wickra_screener_command`. Commands
are JSON objects with a `"cmd"` field: `set_spec`, `feed`, `feed_batch`,
`evaluate`, `scan`, `reset`, `version`. Responses are JSON, e.g.
`{"matches":[...],"scanned":N}` for a scan or `{"ok":true}` for a mutation.

The response is returned via a caller-owned buffer with a length-out protocol —
the callee never allocates memory the caller must free:

1. Call with `out = NULL`, `cap = 0` to learn the response length `len`
   (excluding the terminating NUL).
2. Allocate `len + 1` bytes and call again; the response plus a NUL is written.

Whenever `len < cap`, the response is written on that call, so a
sufficiently-large buffer needs only one call.

Return codes:

| Return   | Meaning                                             |
|----------|-----------------------------------------------------|
| `>= 0`   | Response length in bytes (excluding the NUL).       |
| `-1`     | A required pointer (`handle` or `cmd_json`) is null. |
| `-2`     | `cmd_json` is not valid UTF-8.                       |
| `-3`     | A panic was caught at the boundary.                 |

Domain errors (a bad spec, an unknown command) are **not** negative — they come
back in-band as `{"ok":false,"error":...}` JSON in the buffer.

### Header generation

`include/wickra_screener.h` is generated with [cbindgen] and committed; CI fails
if it drifts from the source. Regenerate after changing the ABI:

```sh
cbindgen --config cbindgen.toml --crate wickra-screener-c --output include/wickra_screener.h
```

## Benchmark

Every binding forwards to the same data-driven Rust core, so what this one adds is
the call overhead of the C ABI itself, not a different result. The core's throughput is
measured by the repository's benchmark suite and the nightly `bench.yml` run; the
numbers, the machine and how to reproduce them are in the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-screener/blob/main/BENCHMARKS.md).

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-screener>
- **Docs** (guides, spec reference, cookbook): <https://screener.wickra.org>
- **Runnable example:** [`examples/c/`](https://github.com/wickra-lib/wickra-screener/tree/main/examples/c)

Wickra Screener ships native bindings for Python, Node.js, WASM and Rust, plus a C ABI hub that any
C-capable language (C, C++, C#, Go, Java, R) links against — all forwarding to the
same data-driven, `unsafe`-forbidden Rust core.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org** with a subject line starting `[wickra security]`. Full
policy: <https://github.com/wickra-lib/wickra-screener/blob/main/SECURITY.md>.

## Disclaimer

Wickra Screener is analysis software: it computes indicator values and evaluates
conditions over historical and live market data. It is provided "as is", without
warranty of any kind, and is **not financial advice** — it places no orders.
Trading carries risk of loss; review the code and use at your own discretion.

## License

Licensed under either of [Apache-2.0](https://github.com/wickra-lib/wickra-screener/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/wickra-lib/wickra-screener/blob/main/LICENSE-MIT) at your option.

[cbindgen]: https://github.com/mozilla/cbindgen
