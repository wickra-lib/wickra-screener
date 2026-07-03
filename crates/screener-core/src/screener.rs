//! The `Screener` handle and the JSON `command_json` boundary — the single FFI
//! entry point exposed in every language binding (§6.9).

use crate::error::{Error, Result};
use crate::scan::{evaluate_universe, scan_batch, ScanReport};
use crate::spec::ScanSpec;
use crate::universe::Universe;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use wickra_backtest_core::Candle;

/// A screener over a fixed spec, holding a streaming universe of fed candles.
pub struct Screener {
    spec: ScanSpec,
    universe: Universe,
}

impl Screener {
    /// Build a screener from a spec JSON string (validated).
    pub fn new(spec_json: &str) -> Result<Self> {
        let spec = ScanSpec::from_json(spec_json)?;
        Ok(Self {
            spec,
            universe: Universe::new(),
        })
    }

    /// The crate version string.
    #[must_use]
    pub fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Replace the spec and clear the streaming universe (the indicator set
    /// changes with the spec).
    pub fn set_spec(&mut self, spec: ScanSpec) {
        self.spec = spec;
        self.universe = Universe::new();
    }

    /// Feed one candle for a symbol into the streaming universe.
    pub fn feed(&mut self, symbol: &str, candle: &Candle) -> Result<()> {
        self.universe.ensure(symbol, &self.spec)?;
        self.universe.fold(symbol, candle);
        Ok(())
    }

    /// Evaluate the current streaming universe.
    pub fn evaluate(&self) -> ScanReport {
        evaluate_universe(&self.universe, &self.spec, self.universe.symbols.len())
    }

    /// Clear the streaming universe, keeping the spec.
    pub fn reset(&mut self) {
        self.universe = Universe::new();
    }

    /// The single JSON-in / JSON-out command boundary. Never returns `Err` for a
    /// well-formed call: internal errors come back as `{"ok":false,"error":...}`.
    pub fn command_json(&mut self, cmd_json: &str) -> Result<String> {
        Ok(self
            .dispatch(cmd_json)
            .unwrap_or_else(|e| error_json(&e.to_string())))
    }

    fn dispatch(&mut self, cmd_json: &str) -> Result<String> {
        let value: Value = serde_json::from_str(cmd_json)?;
        let cmd = value
            .get("cmd")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::BadSpec("missing \"cmd\"".into()))?;
        match cmd {
            "set_spec" => {
                let spec: ScanSpec = serde_json::from_value(field(&value, "spec")?)?;
                spec.validate()?;
                self.set_spec(spec);
                Ok(ok_json())
            }
            "feed" => {
                let symbol = str_field(&value, "symbol")?.to_string();
                let candle: Candle = serde_json::from_value(field(&value, "candle")?)?;
                self.feed(&symbol, &candle)?;
                Ok(ok_json())
            }
            "feed_batch" => {
                let symbol = str_field(&value, "symbol")?.to_string();
                let candles: Vec<Candle> = serde_json::from_value(field(&value, "candles")?)?;
                for candle in &candles {
                    self.feed(&symbol, candle)?;
                }
                Ok(ok_json())
            }
            "evaluate" => Ok(serde_json::to_string(&self.evaluate())?),
            "scan" => {
                let data: BTreeMap<String, Vec<Candle>> =
                    serde_json::from_value(field(&value, "data")?)?;
                Ok(serde_json::to_string(&scan_batch(&data, &self.spec)?)?)
            }
            "reset" => {
                self.reset();
                Ok(ok_json())
            }
            "version" => Ok(json!({ "version": Self::version() }).to_string()),
            other => Err(Error::BadSpec(format!("unknown cmd: {other}"))),
        }
    }
}

/// Clone a named field out of the envelope, erroring if absent.
fn field(value: &Value, name: &str) -> Result<Value> {
    value
        .get(name)
        .cloned()
        .ok_or_else(|| Error::BadSpec(format!("missing \"{name}\"")))
}

/// Read a named string field out of the envelope.
fn str_field<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::BadSpec(format!("missing string \"{name}\"")))
}

fn ok_json() -> String {
    json!({ "ok": true }).to_string()
}

fn error_json(message: &str) -> String {
    json!({ "ok": false, "error": message }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"{"universe":["A","B","C"],"condition":{"type":"cmp","left":{"kind":"price","field":"close"},"op":"gt","right":{"kind":"const","value":15}}}"#;

    fn candle_json(close: f64) -> String {
        format!(
            r#"{{"time":0,"open":{close},"high":{close},"low":{close},"close":{close},"volume":0}}"#
        )
    }

    #[test]
    fn version_command() {
        let mut s = Screener::new(SPEC).unwrap();
        let r = s.command_json(r#"{"cmd":"version"}"#).unwrap();
        assert_eq!(r, format!(r#"{{"version":"{}"}}"#, Screener::version()));
    }

    #[test]
    fn streaming_equals_batch() {
        let mut s = Screener::new(SPEC).unwrap();
        for (sym, close) in [("A", 10.0), ("B", 20.0), ("C", 30.0)] {
            s.command_json(&format!(
                r#"{{"cmd":"feed","symbol":"{sym}","candle":{}}}"#,
                candle_json(close)
            ))
            .unwrap();
        }
        let streamed = s.command_json(r#"{"cmd":"evaluate"}"#).unwrap();
        let batched = s
            .command_json(&format!(
                r#"{{"cmd":"scan","data":{{"A":[{}],"B":[{}],"C":[{}]}}}}"#,
                candle_json(10.0),
                candle_json(20.0),
                candle_json(30.0)
            ))
            .unwrap();
        assert_eq!(streamed, batched);
    }

    #[test]
    fn unknown_cmd_returns_error_json() {
        let mut s = Screener::new(SPEC).unwrap();
        let r = s.command_json(r#"{"cmd":"nope"}"#).unwrap();
        assert!(r.contains(r#""ok":false"#));
    }

    #[test]
    fn reset_clears_the_universe() {
        let mut s = Screener::new(SPEC).unwrap();
        s.command_json(&format!(
            r#"{{"cmd":"feed","symbol":"B","candle":{}}}"#,
            candle_json(20.0)
        ))
        .unwrap();
        s.command_json(r#"{"cmd":"reset"}"#).unwrap();
        let eval = s.command_json(r#"{"cmd":"evaluate"}"#).unwrap();
        assert!(eval.contains(r#""scanned":0"#));
    }

    #[test]
    fn bad_json_returns_error_json() {
        let mut s = Screener::new(SPEC).unwrap();
        let r = s.command_json("not json").unwrap();
        assert!(r.contains(r#""ok":false"#));
    }
}
