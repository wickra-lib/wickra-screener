//! Load the spec and universe, run the scan, and render the report.

use crate::args::{Args, Format};
use screener_core::{scan_batch, Candle, Config, ScanReport, ScanSpec, SymbolInput};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::Path;
use wickra_data::csv::CandleReader;

/// Load the inputs, run the scan and return the rendered output.
pub fn run(args: &Args) -> Result<String, String> {
    let mut spec = load_spec(&args.spec)?;
    if let Some(limit) = args.limit {
        spec.limit = Some(limit);
    }

    let data = load_universe(args, &spec)?;

    let report = scan_batch(data, &spec).map_err(|e| e.to_string())?;

    Ok(match args.format {
        Format::Json => {
            let mut json = serde_json::to_string(&report).map_err(|e| e.to_string())?;
            json.push('\n');
            json
        }
        Format::Text => render_text(&report),
    })
}

/// Load the universe from whichever source the arguments name.
fn load_universe(args: &Args, spec: &ScanSpec) -> Result<BTreeMap<String, SymbolInput>, String> {
    if args.stdin {
        return load_stdin();
    }
    if let Some(dir) = &args.data {
        return load_data_dir(dir);
    }
    load_live(args, spec)
}

/// Pull the spec's universe from an exchange.
#[cfg(feature = "live")]
fn load_live(args: &Args, spec: &ScanSpec) -> Result<BTreeMap<String, SymbolInput>, String> {
    use screener_core::{ExchangeOptions, LiveUniverse};

    let Some(venue) = &args.live else {
        return Err("no data source (pass --data, --stdin or --live)".to_string());
    };
    let mut universe = LiveUniverse::connect(
        venue,
        args.interval.clone(),
        args.bars,
        &ExchangeOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    universe.fetch(&spec.universe).map_err(|e| e.to_string())
}

/// Without the `live` feature there is no third source to fall through to.
#[cfg(not(feature = "live"))]
fn load_live(_args: &Args, _spec: &ScanSpec) -> Result<BTreeMap<String, SymbolInput>, String> {
    Err("no data source (pass --data or --stdin)".to_string())
}

/// Read and parse a spec file, choosing JSON or TOML by extension.
fn load_spec(path: &Path) -> Result<ScanSpec, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("read spec {}: {e}", path.display()))?;
    let is_toml = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
    let cfg = if is_toml {
        Config::from_toml(&content)
    } else {
        Config::from_json(&content)
    };
    cfg.map(|c| c.spec).map_err(|e| e.to_string())
}

/// Load a universe from a directory of `<SYMBOL>.csv` files.
fn load_data_dir(dir: &Path) -> Result<BTreeMap<String, SymbolInput>, String> {
    let mut data = BTreeMap::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("csv") {
            continue;
        }
        let symbol = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("bad file name: {}", path.display()))?
            .to_string();
        data.insert(symbol, read_candles(&path)?.into());
    }
    Ok(data)
}

/// Load a universe as a JSON dataset from stdin.
///
/// Each symbol is either a bare candle array (`{"SYMBOL": [candle, ...]}`) or a
/// series carrying that symbol's side feeds
/// (`{"SYMBOL": {"candles": [...], "books": [...]}}`), which is how a scan over
/// the order-book, derivatives, trade-flow or breadth families is supplied — a
/// CSV directory can only carry candles.
fn load_stdin() -> Result<BTreeMap<String, SymbolInput>, String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&buf).map_err(|e| format!("parse stdin dataset: {e}"))
}

/// Read one symbol's candles from a CSV file through `wickra-data`.
///
/// The header must name `timestamp,open,high,low,close,volume`; extra columns
/// are ignored and the order does not matter, because the columns are matched by
/// name. That is stricter than the parser this replaced, which read the first six
/// columns positionally and accepted any header, or none -- so a file whose
/// columns were in a different order was screened as if they were not.
///
/// `wickra-data` also strips a leading UTF-8 BOM (spreadsheet exports carry one,
/// and it otherwise becomes part of the first column name) and rejects a bar
/// whose OHLC values are not finite or whose high is below its low.
fn read_candles(path: &Path) -> Result<Vec<Candle>, String> {
    let mut reader =
        CandleReader::open(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let candles = reader
        .read_all()
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    // `wickra-data` yields the core candle; the scan takes the backtester's,
    // which is a distinct type with the same six fields.
    Ok(candles
        .into_iter()
        .map(|c| Candle {
            time: c.timestamp,
            open: c.open,
            high: c.high,
            low: c.low,
            close: c.close,
            volume: c.volume,
        })
        .collect())
}

/// Render a report as an aligned text table.
fn render_text(report: &ScanReport) -> String {
    if report.matches.is_empty() {
        return format!(
            "no matches ({} scanned{})\n",
            report.scanned,
            missing_note(report)
        );
    }

    let mut keys: BTreeSet<String> = BTreeSet::new();
    for m in &report.matches {
        keys.extend(m.values.keys().cloned());
    }
    let keys: Vec<String> = keys.into_iter().collect();

    let mut header = vec!["symbol".to_string(), "score".to_string()];
    header.extend(keys.iter().cloned());

    let mut rows: Vec<Vec<String>> = Vec::new();
    for m in &report.matches {
        let mut row = vec![
            m.symbol.clone(),
            m.score.map_or_else(|| "-".to_string(), |s| format!("{s}")),
        ];
        for k in &keys {
            row.push(
                m.values
                    .get(k)
                    .map_or_else(|| "-".to_string(), |v| format!("{v}")),
            );
        }
        rows.push(row);
    }

    let mut widths: Vec<usize> = header.iter().map(String::len).collect();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }

    let format_row = |cells: &[String]| -> String {
        cells
            .iter()
            .enumerate()
            .map(|(i, cell)| format!("{cell:<width$}", width = widths[i]))
            .collect::<Vec<_>>()
            .join("  ")
    };

    let mut out = String::new();
    out.push_str(&format_row(&header));
    out.push('\n');
    for row in &rows {
        out.push_str(&format_row(row));
        out.push('\n');
    }
    let _ = write!(
        out,
        "\n{} match(es), {} scanned{}\n",
        report.matches.len(),
        report.scanned,
        missing_note(report)
    );
    out
}

/// The trailing note naming the universe symbols no data arrived for.
///
/// A scan that silently left symbols out reads exactly like one that saw the
/// whole universe, so the omission is stated rather than left to be inferred
/// from a count. Empty when nothing is missing.
fn missing_note(report: &ScanReport) -> String {
    if report.missing.is_empty() {
        return String::new();
    }
    format!(
        ", {} missing ({})",
        report.missing.len(),
        report.missing.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write `body` to a temporary `.csv` and read it back through the loader.
    fn read_csv(name: &str, body: &str) -> Result<Vec<Candle>, String> {
        let path = std::env::temp_dir().join(format!("wickra-screener-{name}.csv"));
        fs::write(&path, body).unwrap();
        let result = read_candles(&path);
        let _ = fs::remove_file(&path);
        result
    }

    #[test]
    fn reads_a_csv_with_the_named_columns() {
        let candles = read_csv(
            "named",
            "timestamp,open,high,low,close,volume\n1,10,11,9,10.5,100\n2,10.5,12,10,11,200\n",
        )
        .unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].time, 1);
        assert!((candles[1].close - 11.0).abs() < 1e-9);
    }

    /// Columns are matched by name, so their order in the file does not matter.
    /// The parser this replaced read the first six positionally and would have
    /// read this file as open=100, high=10.5 rather than by their names.
    #[test]
    fn column_order_does_not_matter() {
        let candles = read_csv(
            "reordered",
            "volume,close,low,high,open,timestamp\n100,10.5,9,11,10,1\n",
        )
        .unwrap();
        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0].time, 1);
        assert!((candles[0].open - 10.0).abs() < 1e-9);
        assert!((candles[0].high - 11.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_a_header_missing_a_column() {
        let err = read_csv("short", "timestamp,open,high,low,close\n1,10,11,9,10.5\n").unwrap_err();
        assert!(
            err.contains("volume"),
            "error should name the column: {err}"
        );
    }

    #[test]
    fn rejects_a_bar_whose_high_is_below_its_low() {
        assert!(read_csv(
            "inverted",
            "timestamp,open,high,low,close,volume\n1,10,5,9,10.5,100\n",
        )
        .is_err());
    }

    /// Spreadsheet exports prefix the file with a UTF-8 BOM, which would
    /// otherwise become part of the first column name and fail the header check.
    #[test]
    fn tolerates_a_leading_byte_order_mark() {
        let candles = read_csv(
            "bom",
            "\u{feff}timestamp,open,high,low,close,volume\n1,10,11,9,10.5,100\n",
        )
        .unwrap();
        assert_eq!(candles.len(), 1);
    }

    #[test]
    fn render_text_reports_no_matches() {
        let report = ScanReport {
            matches: vec![],
            scanned: 5,
            missing: vec![],
            stale: vec![],
            timeframe: None,
        };
        let text = render_text(&report);
        assert!(text.contains("no matches"));
        assert!(!text.contains("missing"));
    }

    #[test]
    fn render_text_names_the_missing_symbols() {
        let report = ScanReport {
            matches: vec![],
            scanned: 1,
            missing: vec!["BBB".to_string(), "CCC".to_string()],
            stale: vec![],
            timeframe: None,
        };
        let text = render_text(&report);
        assert!(text.contains("2 missing (BBB, CCC)"), "{text}");
    }
}
