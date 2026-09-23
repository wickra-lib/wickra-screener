# Wickra Screener examples

A runnable "scan a small universe" example in every language. Each one builds a
screener from the same spec (`close > 10`), runs a scan over a two-symbol inline
universe (`AAA` at 5, `BBB` at 15) and prints the report — so `BBB` matches. The
examples are self-contained: the spec and candles are inline, so there is no
shared `data/` directory to load (the golden fixtures live in [`../golden/`](../golden)).

## What every example prints

Every example prints the version and the scan report, for example:

```text
wickra-screener 0.1.7
{"matches":[{"symbol":"BBB","matched":true,"values":{"price.close":15.0}}],"scanned":2}
```

## Rust — `examples/rust/`

As the CI examples job runs it, from the repository root:

```bash
cargo run -q -p wickra-screener-example
```

| Example | What it does |
| --- | --- |
| `src/main.rs` | A runnable Rust example: scan a small universe with the native `scan_batch` API and print the report. |

## C / C++ — `examples/c/`

Build the library first (`cargo build -p wickra-screener-c --release`), then build and run
the examples via CMake, as the CI C ABI job does:

```bash
cmake -S examples/c -B examples/c/build
cmake --build examples/c/build --config Release
ctest --test-dir examples/c/build -C Release --output-on-failure
```

| Example | What it does |
| --- | --- |
| `scan.c` | A minimal C example: run a scan through the wickra-screener C ABI. |
| `scan.cpp` | A minimal C++ example: run a scan through the wickra-screener C++ wrapper. |

## C# — `examples/csharp/`

As the CI examples job runs it, from the repository root:

```bash
dotnet run --project examples/csharp/Scan
```

| Example | What it does |
| --- | --- |
| `Scan/Program.cs` | A runnable .NET example: scan a small universe through the binding. |

## Go — `examples/go/`

As the CI examples job runs it, from the repository root:

```bash
cd examples/go && go run .
```

| Example | What it does |
| --- | --- |
| `scan.go` | A runnable Go example: scan a small universe through the binding. |

## R — `examples/r/`

As the CI examples job runs it, from the repository root:

```bash
R CMD INSTALL bindings/r
Rscript examples/r/scan.R
```

| Example | What it does |
| --- | --- |
| `scan.R` | A runnable R example: scan a small universe through the binding. |

## Java — `examples/java/`

As the CI examples job runs it, from the repository root:

```bash
mvn -f bindings/java/pom.xml -q package -DskipTests
javac -cp bindings/java/target/classes examples/java/Scan.java -d examples/java/out
java --enable-native-access=ALL-UNNAMED  -Dnative.lib.dir="$PWD/target/release"  -cp "bindings/java/target/classes:examples/java/out" Scan
```

| Example | What it does |
| --- | --- |
| `Scan.java` | A runnable Java example: scan a small universe through the binding. |

## Python — `examples/python/`

As the CI examples job runs it, from the repository root:

```bash
python -m pip install --require-hashes -r .github/requirements/ci-dev-py3.txt
( cd bindings/python && maturin build --release --out dist )
python -m pip install --no-index --find-links bindings/python/dist wickra-screener
python examples/python/scan.py
```

| Example | What it does |
| --- | --- |
| `scan.py` | A runnable Python example: scan a small universe through the binding. |

## Node.js — `examples/node/`

As the CI examples job runs it, from the repository root:

```bash
( cd bindings/node && npm install --no-audit --no-fund && npx napi build --platform --release )
( cd examples/node && npm install --no-audit --no-fund )
node examples/node/scan.js
```

| Example | What it does |
| --- | --- |
| `scan.js` | A runnable Node.js example: scan a small universe through the binding. |

## WASM — `examples/wasm/`

Build the WASM package, serve the repository root, and open the page in a browser;
the module script inside it is what runs (CI parses it with `node --check`):

```bash
wasm-pack build bindings/wasm --target web
python -m http.server 8000     # then open http://localhost:8000/examples/wasm/
```

| Example | What it does |
| --- | --- |
| `scan.html` | A runnable example against this binding. |

## Example datasets

The examples are self-contained: the spec and the input are inline, so there is
no shared `data/` directory to load. The cross-language golden fixtures, which
every binding is checked against byte for byte, live in [`../golden/`](../golden).
