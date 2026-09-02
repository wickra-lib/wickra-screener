//! Scalar expressions: a constant, a candle price field, an indicator output, a
//! value from earlier bars, or arithmetic over any of those.
//!
//! The set mirrors `wickra_backtest_core::spec::OperandExpr`, so a screen can say
//! what a strategy can say. Without it a spec cannot express "the gap between
//! price and its average", "higher than ten bars ago", or any ratio — the kinds
//! of thing a screen is usually written around.

use serde::{Deserialize, Serialize};

/// A scalar value referenced by a condition.
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
    /// The value an expression had `bars` bars ago.
    Prev {
        /// The expression to look back on.
        of: Box<Expr>,
        /// How many bars back. Zero is the current bar.
        bars: u32,
    },
    /// `left + right`.
    Add {
        /// Left-hand expression.
        left: Box<Expr>,
        /// Right-hand expression.
        right: Box<Expr>,
    },
    /// `left - right`.
    Sub {
        /// Left-hand expression.
        left: Box<Expr>,
        /// Right-hand expression.
        right: Box<Expr>,
    },
    /// `left * right`.
    Mul {
        /// Left-hand expression.
        left: Box<Expr>,
        /// Right-hand expression.
        right: Box<Expr>,
    },
    /// `left / right`. A division that is not finite has no value, so a
    /// condition over it is false rather than surprising.
    Div {
        /// Left-hand expression.
        left: Box<Expr>,
        /// Right-hand expression.
        right: Box<Expr>,
    },
}

/// A field of an OHLCV candle, or a standard average of several.
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
    /// `(high + low + close) / 3`, the typical price.
    Hlc3,
    /// `(open + high + low + close) / 4`, the average price.
    Ohlc4,
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
            PriceField::Hlc3 => "hlc3",
            PriceField::Ohlc4 => "ohlc4",
        }
    }
}

impl Expr {
    /// The canonical string key for this expression, used as the deterministic
    /// key in `ScanResult.values` and by the indicator set.
    ///
    /// Format: `const(<v>)`, `price.<field>`, `<name>(<p,p,...>)`, or
    /// `<name>(<p,...>).<field>` for a multi-output field; compound forms nest
    /// as `prev(<inner>,<n>)` and `add(<l>,<r>)`. Whole-valued numbers print
    /// without a decimal point (`14`, not `14.0`).
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
            Expr::Prev { of, bars } => format!("prev({},{bars})", of.key()),
            Expr::Add { left, right } => binary_key("add", left, right),
            Expr::Sub { left, right } => binary_key("sub", left, right),
            Expr::Mul { left, right } => binary_key("mul", left, right),
            Expr::Div { left, right } => binary_key("div", left, right),
        }
    }

    /// Visit this expression and every expression nested inside it.
    ///
    /// Every walk over a spec goes through here, so a compound expression cannot
    /// hide an indicator from the registry check, the feed check or the
    /// indicator set the way a hand-rolled match over the top level would let it.
    pub fn visit(&self, seen: &mut impl FnMut(&Expr)) {
        seen(self);
        match self {
            Expr::Const { .. } | Expr::Price { .. } | Expr::Indicator { .. } => {}
            Expr::Prev { of, .. } => of.visit(seen),
            Expr::Add { left, right }
            | Expr::Sub { left, right }
            | Expr::Mul { left, right }
            | Expr::Div { left, right } => {
                left.visit(seen);
                right.visit(seen);
            }
        }
    }

    /// The deepest lookback this expression reaches, in bars.
    #[must_use]
    pub fn lookback(&self) -> usize {
        match self {
            Expr::Const { .. } | Expr::Price { .. } | Expr::Indicator { .. } => 0,
            Expr::Prev { of, bars } => *bars as usize + of.lookback(),
            Expr::Add { left, right }
            | Expr::Sub { left, right }
            | Expr::Mul { left, right }
            | Expr::Div { left, right } => left.lookback().max(right.lookback()),
        }
    }
}

/// The key of a binary form.
fn binary_key(op: &str, left: &Expr, right: &Expr) -> String {
    format!("{op}({},{})", left.key(), right.key())
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

    fn close() -> Expr {
        Expr::Price {
            field: PriceField::Close,
        }
    }

    fn sma(period: f64) -> Expr {
        Expr::Indicator {
            name: "Sma".into(),
            params: vec![period],
            field: None,
        }
    }

    #[test]
    fn key_formats() {
        assert_eq!(Expr::Const { value: 30.0 }.key(), "const(30)");
        assert_eq!(Expr::Const { value: 1.5 }.key(), "const(1.5)");
        assert_eq!(close().key(), "price.close");
        assert_eq!(
            Expr::Price {
                field: PriceField::Hlc3
            }
            .key(),
            "price.hlc3"
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
    fn compound_keys_nest() {
        let gap = Expr::Sub {
            left: Box::new(close()),
            right: Box::new(sma(20.0)),
        };
        assert_eq!(gap.key(), "sub(price.close,Sma(20))");
        let back = Expr::Prev {
            of: Box::new(gap),
            bars: 5,
        };
        assert_eq!(back.key(), "prev(sub(price.close,Sma(20)),5)");
        assert_eq!(
            Expr::Div {
                left: Box::new(close()),
                right: Box::new(Expr::Const { value: 2.0 }),
            }
            .key(),
            "div(price.close,const(2))"
        );
    }

    #[test]
    fn visit_reaches_every_nested_expression() {
        let expr = Expr::Add {
            left: Box::new(Expr::Prev {
                of: Box::new(sma(20.0)),
                bars: 3,
            }),
            right: Box::new(Expr::Mul {
                left: Box::new(close()),
                right: Box::new(Expr::Const { value: 2.0 }),
            }),
        };
        let mut keys = Vec::new();
        expr.visit(&mut |e| keys.push(e.key()));
        assert!(keys.contains(&"Sma(20)".to_string()));
        assert!(keys.contains(&"price.close".to_string()));
        assert!(keys.contains(&"const(2)".to_string()));
        assert_eq!(keys.len(), 6, "self plus five descendants: {keys:?}");
    }

    #[test]
    fn lookback_is_the_deepest_reach() {
        assert_eq!(close().lookback(), 0);
        assert_eq!(
            Expr::Prev {
                of: Box::new(close()),
                bars: 7,
            }
            .lookback(),
            7
        );
        // Nested lookbacks add: three bars before the value five bars ago.
        assert_eq!(
            Expr::Prev {
                of: Box::new(Expr::Prev {
                    of: Box::new(close()),
                    bars: 5,
                }),
                bars: 3,
            }
            .lookback(),
            8
        );
        // A binary form reaches as far as its deeper side.
        assert_eq!(
            Expr::Sub {
                left: Box::new(Expr::Prev {
                    of: Box::new(close()),
                    bars: 4,
                }),
                right: Box::new(close()),
            }
            .lookback(),
            4
        );
    }

    #[test]
    fn expr_json_roundtrip() {
        let e = Expr::Div {
            left: Box::new(Expr::Indicator {
                name: "roc".into(),
                params: vec![20.0],
                field: None,
            }),
            right: Box::new(Expr::Prev {
                of: Box::new(close()),
                bars: 2,
            }),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(serde_json::from_str::<Expr>(&json).unwrap(), e);
        assert!(json.contains(r#""kind":"div""#), "{json}");
    }
}
