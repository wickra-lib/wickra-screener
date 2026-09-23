<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514-7" alt="Wickra Screener — parallel multi-symbol screening over 497 streaming indicators" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/ci.svg)](https://github.com/wickra-lib/wickra-screener/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-screener)
[![Maven Central](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/maven.svg)](https://central.sonatype.com/artifact/org.wickra/wickra-screener)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-screener/license.svg)](https://github.com/wickra-lib/wickra-screener#license)

# Wickra Screener — Java

---

> **▶ Live demo:** all 514 indicators over real Binance market data, computed live in your browser — **[live.wickra.org](https://live.wickra.org)** · zero backend, powered by `wickra-wasm`.

**Scan thousands of symbols in parallel against data-driven conditions over 497 O(1) streaming indicators — for Java. `org.wickra:wickra-screener` — prebuilt native library inside the jar, no JNI, no system dependencies.**

JVM bindings for the `wickra-screener` data-driven core over its C ABI hub
(FFM / Panama, `java.lang.foreign`). Build a `Screener` from a spec JSON, drive
it with command JSON, read back scan reports — the same protocol as every other
binding.

## Requirements

- Java 22+ (the Foreign Function & Memory API is stable since 22).
- Run with `--enable-native-access=ALL-UNNAMED`.
- The native library (`wickra_screener`) must be resolvable — either on the
  library path or via the `native.lib.dir` system property pointing at the
  directory that holds `libwickra_screener.{so,dylib}` / `wickra_screener.dll`.

## Install

Maven:

```xml
<dependency>
  <groupId>org.wickra</groupId>
  <artifactId>wickra-screener</artifactId>
  <version>0.1.7</version>
</dependency>
```

Gradle:

```kotlin
implementation("org.wickra:wickra-screener:0.1.7")
```

The native library ships prebuilt per platform inside the jar and is
extracted automatically on first use. There is nothing to compile.

## Quick start

```java
import org.wickra.screener.Screener;

String spec = """
    {"universe":["AAA","BBB"],"condition":{"type":"cmp",
    "left":{"kind":"price","field":"close"},"op":"gt",
    "right":{"kind":"const","value":10.0}}}""";

try (Screener screener = new Screener(spec)) {
    String cmd = """
        {"cmd":"scan","data":{
        "AAA":[{"time":1,"open":5,"high":5,"low":5,"close":5,"volume":1}],
        "BBB":[{"time":1,"open":15,"high":15,"low":15,"close":15,"volume":1}]}}""";
    System.out.println(screener.command(cmd)); // {"matches":[{"symbol":"BBB",...}],"scanned":2}
}
System.out.println(Screener.version());
```

### API

| Member | Description |
|--------|-------------|
| `new Screener(String specJson)` | Build a screener from a spec JSON (throws `IllegalArgumentException` on an invalid spec). |
| `String command(String cmdJson)` | Apply a command JSON, return the response JSON. |
| `static String version()` | The library version. |
| `close()` | Free the native handle (via `AutoCloseable`). |

## Benchmark

Every binding forwards to the same data-driven Rust core, so what this one adds is
the call overhead of the Java Foreign Function & Memory API over the C ABI, not a different result. The core's throughput is
measured by the repository's benchmark suite and the nightly `bench.yml` run; the
numbers, the machine and how to reproduce them are in the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-screener/blob/main/BENCHMARKS.md).

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-screener>
- **Docs** (guides, spec reference, cookbook): <https://screener.wickra.org>
- **Runnable example:** [`examples/java/`](https://github.com/wickra-lib/wickra-screener/tree/main/examples/java)

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
