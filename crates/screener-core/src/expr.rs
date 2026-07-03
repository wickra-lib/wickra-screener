//! Scalar expressions: a constant, a candle price field, or an indicator output.

use serde::{Deserialize, Serialize};

/// A scalar value referenced by a condition: a constant, a candle price field,
/// or the output of a `wickra-core` indicator.
///
/// Price fields are a dedicated variant (`Price`) rather than fake pass-through
/// indicators, so `open`/`high`/`low`/`close`/`volume` read straight from the
/// candle and never need a registry entry.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expr {
    /// A literal value.
    Const {
        /// The constant.
        value: f64,
    },
    /// A field of the current candle.
    Price {
        /// Which price field to read.
        field: PriceField,
    },
    /// The output of an indicator resolved from the `wickra-core` registry by
    /// name and parameters. `field` selects a sub-output of a multi-output
    /// indicator; `None` picks the registry's primary field.
    Indicator {
        /// Indicator name in the registry (e.g. `"rsi"`, `"macd"`).
        name: String,
        /// Indicator parameters (e.g. `[14]`, `[12, 26, 9]`).
        #[serde(default)]
        params: Vec<f64>,
        /// Optional sub-output field for multi-output indicators.
        #[serde(default)]
        field: Option<String>,
    },
}

/// A field of an OHLCV candle.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PriceField {
    /// Opening price.
    Open,
    /// High price.
    High,
    /// Low price.
    Low,
    /// Closing price.
    Close,
    /// Volume.
    Volume,
}

impl PriceField {
    /// The canonical lowercase name used in keys and JSON.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            PriceField::Open => "open",
            PriceField::High => "high",
            PriceField::Low => "low",
            PriceField::Close => "close",
            PriceField::Volume => "volume",
        }
    }
}

impl Expr {
    /// The canonical string key for this expression, used as the deterministic
    /// key in `ScanResult.values` and by the indicator set.
    ///
    /// Format: `const(<v>)`, `price.<field>`, `<name>(<p,p,...>)`, or
    /// `<name>(<p,...>).<field>` for a multi-output field. Whole-valued numbers
    /// print without a decimal point (`14`, not `14.0`).
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Expr::Const { value } => format!("const({})", fmt_num(*value)),
            Expr::Price { field } => format!("price.{}", field.as_str()),
            Expr::Indicator {
                name,
                params,
                field,
            } => {
                let params = params
                    .iter()
                    .map(|p| fmt_num(*p))
                    .collect::<Vec<_>>()
                    .join(",");
                let base = format!("{name}({params})");
                match field {
                    Some(f) => format!("{base}.{f}"),
                    None => base,
                }
            }
        }
    }
}

/// Format a parameter/constant for a key: whole values as integers, otherwise
/// the default float rendering.
fn fmt_num(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_formats() {
        assert_eq!(Expr::Const { value: 30.0 }.key(), "const(30)");
        assert_eq!(Expr::Const { value: 1.5 }.key(), "const(1.5)");
        assert_eq!(
            Expr::Price {
                field: PriceField::Close
            }
            .key(),
            "price.close"
        );
        assert_eq!(
            Expr::Indicator {
                name: "rsi".into(),
                params: vec![14.0],
                field: None,
            }
            .key(),
            "rsi(14)"
        );
        assert_eq!(
            Expr::Indicator {
                name: "macd".into(),
                params: vec![12.0, 26.0, 9.0],
                field: Some("hist".into()),
            }
            .key(),
            "macd(12,26,9).hist"
        );
    }

    #[test]
    fn expr_json_roundtrip() {
        let e = Expr::Indicator {
            name: "roc".into(),
            params: vec![20.0],
            field: None,
        };
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(serde_json::from_str::<Expr>(&json).unwrap(), e);
    }
}
