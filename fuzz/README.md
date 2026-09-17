# Fuzzing Wickra Screener

[`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) harnesses for the parsing and stateful entry points of Wickra Screener. Fuzzing requires a nightly Rust toolchain; CI runs every target for 30 seconds on the family's pinned `nightly-2026-07-01`.

## Setup

```bash
cargo install cargo-fuzz
rustup toolchain install nightly-2026-07-01
```

The date is the family's fuzz nightly, pinned in `ci.yml`: a rolling `nightly`
regressed with a codegen ICE unrelated to this code, so every repository moves
the date together, on purpose.

## Targets

| Target | What it exercises |
| --- | --- |
| `spec_parse` | The spec-parsing path: arbitrary bytes are parsed as a scan spec (JSON and TOML) and as a config. |
| `condition_eval` | The condition/indicator evaluation: an arbitrary spec (any condition tree, any indicator name/params) is scanned over a fixed, bounded universe. |
| `scan_batch` | The full batch scan: a `{spec, data}` object is parsed and scanned. |
| `symbol_fold` | The streaming symbol fold: an arbitrary candle sequence is fed into a screener and evaluated. |
| `feed_payload` | The streaming feed payload: an arbitrary `feed` command, including the side feeds a bar may carry, is driven through the JSON boundary every binding uses. |

## Run

```bash
# From the repository root:
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu spec_parse
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu condition_eval
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu scan_batch
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu symbol_fold
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu feed_payload
```

Each run continues until a crash is found or it is interrupted. A short
time-boxed smoke run is what CI does:

```bash
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu spec_parse -- -max_total_time=30
```

The expectation for every target is that it never panics: malformed or
adversarial input must surface as an `Err` or an in-band error, never a crash.
