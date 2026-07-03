//! The `wickra-screener` reference CLI.
//!
//! Loads a `ScanSpec` and a universe of candles (a directory of `<SYMBOL>.csv`
//! files or a JSON dataset on stdin), runs a scan through `screener-core`, and
//! prints the report as text or JSON.

mod args;
mod run;

use args::Args;
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    match run::run(&args) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("wickra-screener: {err}");
            ExitCode::FAILURE
        }
    }
}
