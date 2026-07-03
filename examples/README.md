# Examples

A runnable "scan a small universe" example in every language. Each one builds a
screener from the same spec (`close > 10`), runs a scan over a two-symbol inline
universe (`AAA` at 5, `BBB` at 15) and prints the report — so `BBB` matches. The
examples are self-contained: the spec and candles are inline, so there is no
shared `data/` directory to load (the golden fixtures live in [`../golden/`](../golden)).

| Language | Path | Run |
|----------|------|-----|
| Rust | [`rust/`](rust/) | `cargo run -p wickra-screener-example` |
| Python | [`python/scan.py`](python/scan.py) | `pip install wickra-screener && python examples/python/scan.py` |
| Node.js | [`node/`](node/) | `cd examples/node && npm install && node scan.js` |
| C / C++ | [`c/`](c/) | see below |
| Go | [`go/`](go/) | `cd examples/go && go run .` |
| .NET | [`csharp/Scan/`](csharp/Scan/) | `dotnet run --project examples/csharp/Scan` |
| Java | [`java/Scan.java`](java/Scan.java) | see the header comment |
| R | [`r/scan.R`](r/scan.R) | `Rscript examples/r/scan.R` |

The native bindings (Python, Node.js) load their own compiled library. The bindings
that go through the C ABI (Go, .NET, Java, R, and the C/C++ example itself) need the
C ABI library built first:

```bash
cargo build --release -p wickra-screener-c
```

## C / C++

The C and C++ examples build with CMake and run under ctest:

```bash
cargo build --release -p wickra-screener-c
cmake -S examples/c -B examples/c/build
cmake --build examples/c/build --config Release
ctest --test-dir examples/c/build -C Release --output-on-failure
```

On Windows the build copies `wickra_screener.dll` next to each executable, since
there is no rpath.

## Expected output

Every example prints the version and the scan report, for example:

```text
wickra-screener 0.1.0
{"matches":[{"symbol":"BBB","matched":true,"values":{"price.close":15.0}}],"scanned":2}
```
