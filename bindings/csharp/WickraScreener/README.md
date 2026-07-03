# Wickra Screener — .NET

.NET bindings for the `wickra-screener` data-driven core over its C ABI hub
(source-generated P/Invoke). Build a `Screener` from a spec JSON, drive it with
command JSON, read back scan reports — the same protocol as every other binding.

## Install

```bash
dotnet add package Wickra.Screener
```

## Usage

```csharp
using Wickra.Screener;

const string spec = """
{"universe":["AAA","BBB"],"condition":{"type":"cmp",
"left":{"kind":"price","field":"close"},"op":"gt",
"right":{"kind":"const","value":10.0}}}
""";

using var screener = new Screener(spec);

const string cmd = """
{"cmd":"scan","data":{
"AAA":[{"time":1,"open":5,"high":5,"low":5,"close":5,"volume":1}],
"BBB":[{"time":1,"open":15,"high":15,"low":15,"close":15,"volume":1}]}}
""";

string report = screener.Command(cmd);
Console.WriteLine(report);            // {"matches":[{"symbol":"BBB",...}],"scanned":2}
Console.WriteLine(Screener.Version());
```

## API

| Member | Description |
|--------|-------------|
| `new Screener(string specJson)` | Build a screener from a spec JSON (throws `ArgumentException` on an invalid spec). |
| `string Command(string cmdJson)` | Apply a command JSON, return the response JSON. |
| `static string Version()` | The library version. |
| `Dispose()` | Free the native handle (via `IDisposable`). |

The native library is located by a `DllImportResolver` that probes the default
search paths and the Cargo `target/` directory, validating each candidate with a
sentinel export check.

## License

`MIT OR Apache-2.0`.
