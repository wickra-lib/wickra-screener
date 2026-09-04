//! Resolves the indicators a spec references and folds candles through them.
//!
//! Indicators are resolved by name and parameters from the `wickra-core`
//! registry — reused through the `wickra-backtest-core` factory, the only
//! name -> indicator resolver in the ecosystem. Each resolved indicator is an
//! object-safe `EvalIndicator`, driven with a [`BarInput`] built from the bar's
//! candle and whatever side feeds the scan supplies.

use crate::error::{Error, Result};
use crate::expr::Expr;
use crate::feeds::{BarFeeds, FeedKind};
use std::collections::BTreeMap;
use wickra_backtest_core::registry::{build, feed_of, BarInput};
use wickra_backtest_core::spec::Feed;
use wickra_backtest_core::{Candle, EvalIndicator};

/// The pairwise indicators, which read the reference series' close alongside the
/// bar close.
///
/// This list exists because `registry::feed_of` cannot express the family:
/// upstream classifies every pairwise indicator as `Feed::Kline`, since the
/// candle is indeed one of its two inputs. The `pairwise_list_matches_behaviour`
/// test probes every registry name and fails if this list ever drifts from the
/// set that actually needs a reference, so a registry that grows a new pairwise
/// indicator cannot slip past silently.
const PAIRWISE: [&str; 24] = [
    "Alpha",
    "Beta",
    "BetaNeutralSpread",
    "Cointegration",
    "DistanceSsd",
    "GrangerCausality",
    "HasbrouckInformationShare",
    "InformationRatio",
    "KalmanHedgeRatio",
    "KendallTau",
    "LeadLagCrossCorrelation",
    "OuHalfLife",
    "PairSpreadZScore",
    "PairwiseBeta",
    "PearsonCorrelation",
    "RelativeStrengthAB",
    "RollingCorrelation",
    "RollingCovariance",
    "SpearmanCorrelation",
    "SpreadAr1Coefficient",
    "SpreadBollingerBands",
    "SpreadHurst",
    "TreynorRatio",
    "VarianceRatio",
];

/// Which feed an indicator consumes, or `None` if the registry does not know it.
///
/// Wraps `registry::feed_of` and refines its `Kline` answer with the pairwise
/// list, so a caller can tell "candle is enough" from "needs a reference".
#[must_use]
pub fn feed_kind(name: &str) -> Option<FeedKind> {
    let feed = feed_of(name)?;
    Some(match feed {
        Feed::Kline if PAIRWISE.contains(&name) => FeedKind::Pair,
        Feed::Kline => FeedKind::Candle,
        Feed::Trade => FeedKind::Trades,
        Feed::Orderbook => FeedKind::OrderBook,
        Feed::TradeQuote => FeedKind::TradeQuote,
        Feed::Derivatives => FeedKind::Derivatives,
        Feed::CrossSection => FeedKind::CrossSection,
    })
}

/// One resolved indicator plus its canonical base key (`<name>(<p,p>)`).
struct Entry {
    key: String,
    indicator: Box<dyn EvalIndicator>,
}

/// The set of indicators a scan spec needs, folded one candle at a time. Each
/// `update` records the primary value under the indicator's base key and every
/// named sub-output under `<base>.<field>`.
pub(crate) struct IndicatorSet {
    items: Vec<Entry>,
}

impl IndicatorSet {
    /// An empty set.
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Register every indicator an expression needs, including the ones nested
    /// inside a compound form (constants and price fields need none). Idempotent
    /// per base key. Errors if the registry does not know an indicator or rejects
    /// its parameters.
    pub(crate) fn required(&mut self, expr: &Expr) -> Result<()> {
        let mut found: Vec<(String, Vec<f64>)> = Vec::new();
        expr.visit(&mut |node| {
            if let Expr::Indicator { name, params, .. } = node {
                found.push((name.clone(), params.clone()));
            }
        });
        for (name, params) in found {
            let key = base_key(&name, &params);
            if self.items.iter().all(|e| e.key != key) {
                let indicator = build(&name, &params)
                    .map_err(|e| Error::UnknownIndicator(format!("{name}: {e}")))?;
                self.items.push(Entry { key, indicator });
            }
        }
        Ok(())
    }

    /// Fold one bar: every indicator ticks against the candle and the bar's
    /// feeds, and the primary value and each named field are collected into the
    /// map for this bar. The caller keeps the history; this keeps none.
    pub(crate) fn update(&mut self, candle: &Candle, feeds: BarFeeds<'_>) -> BTreeMap<String, f64> {
        let mut values = BTreeMap::new();
        let bar = BarInput {
            candle,
            reference: feeds.reference,
            deriv: feeds.deriv,
            orderbook: feeds.orderbook,
            trades: feeds.trades,
            cross_section: feeds.cross_section,
        };
        for entry in &mut self.items {
            if let Some(value) = entry.indicator.update(&bar) {
                values.insert(entry.key.clone(), value);
                for (field, field_value) in entry.indicator.fields() {
                    values.insert(format!("{}.{field}", entry.key), field_value);
                }
            }
        }
        values
    }

    /// The largest warmup period across all registered indicators.
    pub(crate) fn max_warmup(&self) -> usize {
        self.items
            .iter()
            .map(|e| e.indicator.warmup())
            .max()
            .unwrap_or(0)
    }
}

/// Canonical base key for an indicator expression, without any field suffix:
/// `<name>(<p,p,...>)`. Matches `Expr::key` for a field-less indicator.
fn base_key(name: &str, params: &[f64]) -> String {
    Expr::Indicator {
        name: name.to_string(),
        params: params.to_vec(),
        field: None,
    }
    .key()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feeds::BarFeeds;

    fn candle(close: f64) -> Candle {
        Candle {
            time: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        }
    }

    #[test]
    fn resolves_and_folds_an_sma() {
        let mut set = IndicatorSet::new();
        set.required(&Expr::Indicator {
            name: "Sma".into(),
            params: vec![3.0],
            field: None,
        })
        .unwrap();
        assert!(set.max_warmup() > 0);

        let mut last = BTreeMap::new();
        let mut before = BTreeMap::new();
        for c in [1.0, 2.0, 3.0, 4.0, 5.0] {
            before = std::mem::replace(&mut last, set.update(&candle(c), BarFeeds::default()));
        }
        // 3-bar SMA of the last three closes, and of the window before it.
        assert_eq!(last.get("Sma(3)").copied(), Some(4.0));
        assert_eq!(before.get("Sma(3)").copied(), Some(3.0));
    }

    #[test]
    fn a_compound_expression_registers_the_indicators_nested_in_it() {
        let mut set = IndicatorSet::new();
        set.required(&Expr::Sub {
            left: Box::new(Expr::Indicator {
                name: "Sma".into(),
                params: vec![3.0],
                field: None,
            }),
            right: Box::new(Expr::Prev {
                of: Box::new(Expr::Indicator {
                    name: "Ema".into(),
                    params: vec![4.0],
                    field: None,
                }),
                bars: 2,
            }),
        })
        .unwrap();

        let mut values = BTreeMap::new();
        for c in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0] {
            values = set.update(&candle(c), BarFeeds::default());
        }
        assert!(values.contains_key("Sma(3)"), "{values:?}");
        assert!(values.contains_key("Ema(4)"), "{values:?}");
    }

    #[test]
    fn unknown_indicator_errors() {
        let mut set = IndicatorSet::new();
        assert!(matches!(
            set.required(&Expr::Indicator {
                name: "NotAnIndicator".into(),
                params: vec![],
                field: None,
            }),
            Err(Error::UnknownIndicator(_))
        ));
    }

    #[test]
    fn a_pairwise_indicator_produces_a_value_once_a_reference_arrives() {
        let mut set = IndicatorSet::new();
        set.required(&Expr::Indicator {
            name: "RollingCorrelation".into(),
            params: vec![5.0],
            field: None,
        })
        .unwrap();

        // Without a reference the indicator ticks and yields nothing, every bar.
        let mut values = BTreeMap::new();
        for i in 0..40 {
            values = set.update(&candle(100.0 + f64::from(i)), BarFeeds::default());
        }
        assert_eq!(values.get("RollingCorrelation(5)"), None);

        // With one it produces a value.
        let mut set = IndicatorSet::new();
        set.required(&Expr::Indicator {
            name: "RollingCorrelation".into(),
            params: vec![5.0],
            field: None,
        })
        .unwrap();
        let mut values = BTreeMap::new();
        for i in 0..40 {
            let t = f64::from(i);
            let feeds = BarFeeds {
                reference: Some(50.0 + (t * 0.5).sin() * 5.0),
                ..BarFeeds::default()
            };
            values = set.update(&candle(100.0 + (t * 0.3).sin() * 10.0), feeds);
        }
        assert!(values.contains_key("RollingCorrelation(5)"));
    }

    #[test]
    fn feed_kind_classifies_each_family() {
        assert_eq!(feed_kind("Rsi"), Some(FeedKind::Candle));
        assert_eq!(feed_kind("Beta"), Some(FeedKind::Pair));
        assert_eq!(feed_kind("Cointegration"), Some(FeedKind::Pair));
        assert_eq!(feed_kind("FundingRate"), Some(FeedKind::Derivatives));
        assert_eq!(feed_kind("Microprice"), Some(FeedKind::OrderBook));
        assert_eq!(feed_kind("Vpin"), Some(FeedKind::Trades));
        assert_eq!(feed_kind("EffectiveSpread"), Some(FeedKind::TradeQuote));
        assert_eq!(feed_kind("AdvanceDecline"), Some(FeedKind::CrossSection));
        assert_eq!(feed_kind("NotAnIndicator"), None);
    }

    #[test]
    fn pairwise_list_is_sorted_and_unique() {
        let mut sorted = PAIRWISE;
        sorted.sort_unstable();
        assert_eq!(sorted, PAIRWISE, "keep PAIRWISE sorted for reviewability");
        let mut seen = std::collections::BTreeSet::new();
        assert!(PAIRWISE.iter().all(|n| seen.insert(*n)));
    }

    #[test]
    fn every_pairwise_name_is_a_registry_name_classified_as_kline() {
        for name in PAIRWISE {
            assert_eq!(
                feed_of(name),
                Some(Feed::Kline),
                "{name} is not a Kline-classified registry name"
            );
        }
    }
}
