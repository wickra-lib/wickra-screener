# Golden fixtures

The golden fixtures pin the screener's output byte-for-byte. They are generated
once and replayed everywhere: the Rust core, the CLI and every language binding
must reproduce `expected/<spec>.json` **exactly**. Byte equality holds across all
ten languages because each binding returns the core's `command_json` string
verbatim — there is no per-language JSON re-formatting.

> **Do not edit any file under `golden/` by hand.** Regenerate them with the
> bless command below and commit the result.

## Layout

| Path | What |
|------|------|
| `data/sym-01.csv` … `sym-06.csv` | The canonical universe (per-symbol OHLCV). |
| `data.json` | The same universe as a JSON dataset — the input the bindings feed to a `scan` command. |
| `specs/*.json` | The five canonical scan specs. |
| `expected/<spec>.json` | The byte-exact `ScanReport` for each spec. |

## Data formula

Each symbol has 48 bars. The close of bar `i` (1-based) is

```
close(i) = base + amp * sin(i / k) + drift * i
```

with fixed per-symbol constants:

| symbol | base | amp | k  | drift | shape |
|--------|-----:|----:|---:|------:|-------|
| sym-01 | 100  | 10  | 8  |  0.5  | uptrend + oscillation |
| sym-02 | 100  | 15  | 6  | -0.3  | downtrend |
| sym-03 | 50   | 5   | 10 |  1.0  | strong uptrend |
| sym-04 | 200  | 20  | 5  |  0.0  | pure oscillation (mean-reverting) |
| sym-05 | 80   | 8   | 12 |  0.2  | mild uptrend |
| sym-06 | 150  | 12  | 7  | -0.6  | downtrend |

The remaining fields are derived deterministically:

```
open(i)   = close(i-1)                    (open(1) = base)
high(i)   = max(open, close) + amp * 0.05
low(i)    = min(open, close) - amp * 0.05
volume(i) = 1000 + 5 * i
ts(i)     = 1_700_000_000 + (i-1) * 3600  (hourly)
```

Every value is written with exactly four decimals (`{:.4f}`), so the CSV text and
the JSON dataset parse to the **identical** `f64` — this is what makes the golden
byte-identical across languages.

## Specs

| spec | condition | matches |
|------|-----------|---------|
| `momentum` | `all[Rsi(14) > 50, cross-section Roc(10) rank <= 3]`, rank Roc desc, limit 3 | sym-02, sym-01, sym-03 |
| `mean_reversion` | `any[Rsi(14) < 30, close < BollingerBands(20,2).lower]`, rank Rsi asc | sym-05 |
| `cross_section_rank` | `cross-section Roc(10) rank <= 3` | sym-02, sym-01, sym-03 |
| `breadth` | `all[breadth(Rsi(14) > 50) ratio >= 0.4, close > 100]` | sym-04, sym-06, sym-01, sym-02 |
| `crossover` | `Ema(7) crosses_above Ema(19)` | sym-06 |

Indicators use the backtest registry's PascalCase kinds (`Rsi`, `Roc`, `Ema`,
`BollingerBands`). `mean_reversion` uses `any` rather than `and`: the smooth
sinusoidal universe never drives `close` below the lower band, so the RSI branch
is the one that fires.

## Bless (regenerate)

`expected/*.json` is the core's `command_json` output for `{"cmd":"scan",...}`
over `data.json`, byte-for-byte. The Rust golden test writes any missing file and
otherwise asserts byte equality:

```bash
cargo test -p screener-core --test golden -- --ignored --nocapture
```

Run it once to bless (writes the missing `expected/*.json`), review the diff, and
commit. Regenerate the CSV universe and `data.json` only from the formula above —
never by editing the files.

## Cross-language verification

Every binding replays the same fixtures and asserts byte equality against
`expected/*.json`, so the golden is the cross-language contract:

| Binding | Test | Verified |
|---------|------|----------|
| Rust core | `tests/golden.rs` | parallel + sequential |
| Python | `tests/test_golden.py` | locally |
| Node.js | `__tests__/golden.test.js` | locally |
| Go | `golden_test.go` | locally |
| .NET | `GoldenTests.cs` | locally |
| Java | `GoldenTest.java` | locally |
| R | `tests/run_tests.R` | CI |

A binding feeds `data.json` to a `scan` command and compares the returned string
to `expected/<spec>.json`. Byte equality holds regardless of how each language
serializes the input, because the core normalizes the scan and returns its
`command_json` string verbatim.
