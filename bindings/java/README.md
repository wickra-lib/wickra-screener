# Wickra Screener — Java

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

## Usage

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

## API

| Member | Description |
|--------|-------------|
| `new Screener(String specJson)` | Build a screener from a spec JSON (throws `IllegalArgumentException` on an invalid spec). |
| `String command(String cmdJson)` | Apply a command JSON, return the response JSON. |
| `static String version()` | The library version. |
| `close()` | Free the native handle (via `AutoCloseable`). |

## License

`MIT OR Apache-2.0`.
