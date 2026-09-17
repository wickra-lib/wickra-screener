# Wickra Screener — C / C++ examples

The Wickra Screener C ABI is a single shared/static library plus a generated header
([`bindings/c/include/wickra_screener.h`](../../bindings/c/include/wickra_screener.h)). Any C-capable
language links against the same artifact; these examples show the plain-C path
and, through [`wickra_screener.hpp`](../../bindings/c/include/wickra_screener.hpp), the C++ one.

## Build the library

From the workspace root:

```sh
cargo build -p wickra-screener-c --release
```

This produces, in `target/release/`:

| Platform | Shared library | Link target |
|----------|----------------|-------------|
| Linux    | `libwickra_screener.so`     | `-lwickra_screener` |
| macOS    | `libwickra_screener.dylib`  | `-lwickra_screener` |
| Windows (MSVC) | `wickra_screener.dll` | `wickra_screener.dll.lib` (import lib) |

A static library (`libwickra_screener.a` / `wickra_screener.lib`) is emitted alongside.

## Build and run the examples

### With CMake (portable, used by CI)

```sh
cmake -S examples/c -B examples/c/build
cmake --build examples/c/build --config Release
ctest --test-dir examples/c/build -C Release --output-on-failure
```

### Directly with a compiler

```sh
# Linux / macOS
cc examples/c/scan.c -I bindings/c/include -L target/release -lwickra_screener -lm -o scan
LD_LIBRARY_PATH=target/release ./scan        # macOS: DYLD_LIBRARY_PATH

# Windows (MinGW gcc, linking the DLL directly)
gcc examples/c/scan.c -I bindings/c/include target/release/wickra_screener.dll -lm -o scan.exe
```

## The examples

| Example | What it does |
|---------|--------------|
| `scan.c` | A minimal C example: run a scan through the wickra-screener C ABI. |
| `scan.cpp` | A minimal C++ example: run a scan through the wickra-screener C++ wrapper. |

## Usage shape

Every call follows the same handle discipline: construct from a spec JSON, drive
with command JSON, read the response, free the handle exactly once. `wickra_screener.h` is
the whole contract; the C++ header, where one ships, wraps the handle in a
move-only RAII type. See [`bindings/c/README.md`](../../bindings/c/README.md).
