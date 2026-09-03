# wickra-screener (C#)

.NET bindings for [`wickra-screener`](https://github.com/wickra-lib/wickra-screener)
over the C ABI hub, via source-generated P/Invoke. Build a `Screener` from a spec
JSON, drive it with command JSON and read back scan reports — the same protocol
the CLI and every other binding speak.

```csharp
using Wickra.Screener;

const string spec = """
{"universe":["AAA","BBB"],"condition":{"type":"cmp",
"left":{"kind":"price","field":"close"},"op":"gt",
"right":{"kind":"const","value":10.0}}}
""";

using var screener = new Screener(spec);
string report = screener.Command("""{"cmd":"scan","data":{ … }}""");
```

Requires .NET 8+. The native library (`wickra_screener`) must be resolvable on the
loader path — `PATH` on Windows, `LD_LIBRARY_PATH` on Linux, `DYLD_LIBRARY_PATH`
on macOS. Licensed under `MIT OR Apache-2.0`.

## Layout

| Path | What it is |
| --- | --- |
| `WickraScreener/` | The published package. Its own `README.md` is the long description NuGet renders. |
| `WickraScreener.Tests/` | xUnit suite: golden parity against the shared fixtures, the screener protocol, and the streaming path. |

See [`WickraScreener/README.md`](https://github.com/wickra-lib/wickra-screener/blob/main/bindings/csharp/WickraScreener/README.md) for the full API walk-through,
and [`examples/csharp/`](https://github.com/wickra-lib/wickra-screener/blob/main/examples/csharp) for a runnable program.
