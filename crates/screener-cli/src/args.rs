//! CLI argument parsing.

use clap::{ArgGroup, Parser, ValueEnum};
use std::path::PathBuf;

/// Run a screener scan over a spec and a universe of candles.
#[derive(Parser, Debug)]
#[command(name = "wickra-screener", version, about)]
#[command(group(ArgGroup::new("source").required(true).args(["data", "stdin"])))]
pub struct Args {
    /// Path to the scan spec (JSON or TOML, chosen by file extension).
    #[arg(long)]
    pub spec: PathBuf,

    /// Directory of per-symbol CSV candle files (`<SYMBOL>.csv`).
    #[arg(long)]
    pub data: Option<PathBuf>,

    /// Read the universe as a JSON dataset from standard input instead.
    #[arg(long)]
    pub stdin: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,

    /// Override the spec's match limit.
    #[arg(long)]
    pub limit: Option<usize>,
}

/// The report output format.
#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum Format {
    /// A human-readable aligned table.
    Text,
    /// The raw `ScanReport` JSON.
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn arg_config_is_valid() {
        Args::command().debug_assert();
    }

    #[test]
    fn parses_a_data_source() {
        let args =
            Args::try_parse_from(["wickra-screener", "--spec", "s.json", "--data", "dir"]).unwrap();
        assert_eq!(args.data, Some(PathBuf::from("dir")));
        assert_eq!(args.format, Format::Text);
        assert!(!args.stdin);
    }

    #[test]
    fn data_and_stdin_conflict() {
        assert!(Args::try_parse_from([
            "wickra-screener",
            "--spec",
            "s.json",
            "--data",
            "d",
            "--stdin"
        ])
        .is_err());
    }

    #[test]
    fn a_source_is_required() {
        assert!(Args::try_parse_from(["wickra-screener", "--spec", "s.json"]).is_err());
    }

    #[test]
    fn format_json_parses() {
        let args = Args::try_parse_from([
            "wickra-screener",
            "--spec",
            "s.json",
            "--stdin",
            "--format",
            "json",
        ])
        .unwrap();
        assert_eq!(args.format, Format::Json);
    }
}
