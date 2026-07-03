//! The `wickra-screener` reference CLI.
//!
//! Loads a `ScanSpec` and a directory of per-symbol CSV candle files, runs a
//! scan through `screener-core`, and prints the report as text or JSON. The run
//! logic and glue are filled in over P-SCR-2.3 to 2.4.

mod args;

use args::Args;
use clap::Parser;

fn main() {
    let _args = Args::parse();
    // The scan run and output rendering are wired in over P-SCR-2.3 to 2.4.
}
