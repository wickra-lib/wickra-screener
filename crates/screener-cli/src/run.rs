//! Load the spec and universe, run the scan, and render the report.

use crate::args::{Args, Format};
use screener_core::{scan_batch, Candle, Config, ScanReport, ScanSpec};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Load the inputs, run the scan and return the rendered output.
pub fn run(args: &Args) -> Result<String, String> {
    let mut spec = load_spec(&args.spec)?;
    if let Some(limit) = args.limit {
        spec.limit = Some(limit);
    }

    let data = if args.stdin {
        load_stdin()?
    } else if let Some(dir) = &args.data {
        load_data_dir(dir)?
    } else {
        return Err("no data source (pass --data or --stdin)".to_string());
    };

    let report = scan_batch(&data, &spec).map_err(|e| e.to_string())?;

    Ok(match args.format {
        Format::Json => {
            let mut json = serde_json::to_string(&report).map_err(|e| e.to_string())?;
            json.push('\n');
            json
        }
        Format::Text => render_text(&report),
    })
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
fn load_data_dir(dir: &Path) -> Result<BTreeMap<String, Vec<Candle>>, String> {
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
        let content =
            fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        data.insert(symbol, parse_csv(&content)?);
    }
    Ok(data)
}

/// Load a universe as a JSON dataset (`{"SYMBOL": [candle, ...]}`) from stdin.
fn load_stdin() -> Result<BTreeMap<String, Vec<Candle>>, String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&buf).map_err(|e| format!("parse stdin dataset: {e}"))
}

/// Parse OHLCV rows (`ts,open,high,low,close,volume`) into candles; a
/// non-numeric first row is treated as a header and skipped.
fn parse_csv(content: &str) -> Result<Vec<Candle>, String> {
    let mut candles = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split(',').map(str::trim).collect();
        if cols.len() < 6 {
            return Err(format!(
                "CSV line {}: expected 6 columns, got {}",
                idx + 1,
                cols.len()
            ));
        }
        let time = match cols[0].parse::<i64>() {
            Ok(t) => t,
            Err(_) if idx == 0 => continue, // header row
            Err(e) => return Err(format!("CSV line {}: bad timestamp: {e}", idx + 1)),
        };
        let field = |i: usize, name: &str| {
            cols[i]
                .parse::<f64>()
                .map_err(|e| format!("CSV line {}: {name}: {e}", idx + 1))
        };
        candles.push(Candle {
            time,
            open: field(1, "open")?,
            high: field(2, "high")?,
            low: field(3, "low")?,
            close: field(4, "close")?,
            volume: field(5, "volume")?,
        });
    }
    Ok(candles)
}

/// Render a report as an aligned text table.
fn render_text(report: &ScanReport) -> String {
    if report.matches.is_empty() {
        return format!("no matches ({} scanned)\n", report.scanned);
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
        "\n{} match(es), {} scanned\n",
        report.matches.len(),
        report.scanned
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv_with_a_header() {
        let csv = "ts,open,high,low,close,volume\n1,10,11,9,10.5,100\n2,10.5,12,10,11,200\n";
        let candles = parse_csv(csv).unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].time, 1);
        assert!((candles[1].close - 11.0).abs() < 1e-9);
    }

    #[test]
    fn parse_csv_rejects_a_short_row() {
        assert!(parse_csv("1,2,3\n").is_err());
    }

    #[test]
    fn render_text_reports_no_matches() {
        let report = ScanReport {
            matches: vec![],
            scanned: 5,
        };
        assert!(render_text(&report).contains("no matches"));
    }
}
