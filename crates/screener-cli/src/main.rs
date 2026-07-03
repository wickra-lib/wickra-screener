//! The `wickra-screener` reference CLI.
//!
//! Loads a `ScanSpec` and a universe of candles (a directory of CSV files or a
//! JSON dataset on stdin), runs a scan through `screener-core`, and prints the
//! report as text or JSON.

mod args;
mod run;

use args::Args;
use clap::Parser;

fn main() {
    let args = Args::parse();
    match run::run(&args) {
        Ok(output) => print!("{output}"),
        Err(err) => {
            eprintln!("wickra-screener: {err}");
            std::process::exit(1);
        }
    }
}
