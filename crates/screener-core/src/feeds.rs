//! The side feeds an indicator may consume beyond the candle.
//!
//! Most of the registry is driven by the candle alone, but a substantial part of
//! it reads something else: a reference series (the pairwise family), a
//! derivatives tick, an order-book snapshot, the trades that printed within the
//! bar, or the market cross-section. An indicator whose feed is absent resolves,
//! ticks and returns nothing — every bar, without complaint — so a screen naming
//! one would simply never match. Supplying the feeds is what makes those
//! indicators reachable from a scan; [`crate::spec::ScanSpec::validate`] rejects
//! the specs whose feeds a scan cannot supply, so the silent case is gone.
//!
//! The shapes mirror `wickra-backtest`: [`SymbolSeries`] is the batch form —
//! parallel arrays, one entry per candle, exactly like its `RunRequest` — and
//! `wickra_backtest_core::StepFeeds` is the per-bar streaming form, reused
//! verbatim so the JSON field names match across the ecosystem. A caller holding
//! only candles keeps passing a bare array.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use wickra_backtest_core::{
    Candle, CrossSection, DerivativesTick, OrderBook, StepFeeds, TradePrint,
};
use wickra_core::{
    CrossSection as CoreCrossSection, DerivativesTick as CoreDerivativesTick,
    OrderBook as CoreOrderBook, Trade as CoreTrade,
};

/// Which side feed an indicator family consumes.
///
/// This mirrors `wickra_backtest_core::spec::Feed` with one addition: the
/// pairwise family. Upstream classifies pairwise indicators as `Kline` because
/// they are fed the bar close alongside the reference close, which leaves no way
/// to express "needs a reference series" — [`FeedKind::Pair`] is that missing
/// case, and [`crate::indicator_set::feed_kind`] maps a name onto it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedKind {
    /// The candle alone.
    Candle,
    /// The candle plus the reference series' close (pairwise indicators).
    Pair,
    /// A derivatives tick — funding, open interest, mark and index.
    Derivatives,
    /// An order-book snapshot.
    OrderBook,
    /// The trades that printed within the bar.
    Trades,
    /// Trades quoted against the book mid (needs both feeds).
    TradeQuote,
    /// The market cross-section, for the breadth family.
    CrossSection,
}

impl FeedKind {
    /// The name used in error messages and documentation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            FeedKind::Candle => "candle",
            FeedKind::Pair => "reference",
            FeedKind::Derivatives => "derivs",
            FeedKind::OrderBook => "books",
            FeedKind::Trades => "trades",
            FeedKind::TradeQuote => "trades and books",
            FeedKind::CrossSection => "sections",
        }
    }
}

/// One symbol's candle history plus its optional side feeds.
///
/// Every supplied feed is as long as `candles`; a shorter or longer one is an
/// error rather than a feed that quietly runs out part-way through the fold.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct SymbolSeries {
    /// The candle history, oldest first.
    pub candles: Vec<Candle>,
    /// The reference series for pairwise indicators; its close is what they read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<Vec<Candle>>,
    /// One derivatives tick per candle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivs: Option<Vec<DerivativesTick>>,
    /// One order-book snapshot per candle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub books: Option<Vec<OrderBook>>,
    /// The trades that printed within each candle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trades: Option<Vec<Vec<TradePrint>>>,
    /// One market cross-section per candle, for the breadth family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<CrossSection>>,
}

/// A symbol's scan input: a bare candle array, or the full series with feeds.
///
/// The bare form is the shorthand every existing caller and golden spec uses;
/// keeping it means adding feeds costs nothing to a scan that does not need them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum SymbolInput {
    /// Candles only.
    Candles(Vec<Candle>),
    /// Candles plus side feeds.
    Series(Box<SymbolSeries>),
}

impl SymbolInput {
    /// The candle history behind this input, whichever form it took.
    #[must_use]
    pub fn candles(&self) -> &[Candle] {
        match self {
            SymbolInput::Candles(candles) => candles,
            SymbolInput::Series(series) => &series.candles,
        }
    }

    /// The series behind this input, with the bare form widened.
    #[must_use]
    pub fn into_series(self) -> SymbolSeries {
        match self {
            SymbolInput::Candles(candles) => SymbolSeries {
                candles,
                ..SymbolSeries::default()
            },
            SymbolInput::Series(series) => *series,
        }
    }
}

impl From<Vec<Candle>> for SymbolInput {
    fn from(candles: Vec<Candle>) -> Self {
        SymbolInput::Candles(candles)
    }
}

/// A symbol's series with every feed converted to the `wickra-core` types the
/// indicators actually consume.
///
/// The conversion happens once, on ingest, so a malformed book or tick is an
/// error the caller sees. The backtester drops those with `.ok()`; here that
/// would put back exactly the silent hole this module exists to close.
#[derive(Debug)]
pub(crate) struct CoreSeries {
    pub(crate) candles: Vec<Candle>,
    reference: Option<Vec<f64>>,
    derivs: Option<Vec<CoreDerivativesTick>>,
    books: Option<Vec<CoreOrderBook>>,
    trades: Option<Vec<Vec<CoreTrade>>>,
    sections: Option<Vec<CoreCrossSection>>,
}

impl CoreSeries {
    /// Validate a series' feed lengths and convert every feed to its core type.
    pub(crate) fn build(symbol: &str, series: SymbolSeries) -> Result<Self> {
        let n = series.candles.len();
        check_len(
            symbol,
            "reference",
            series.reference.as_ref().map(Vec::len),
            n,
        )?;
        check_len(symbol, "derivs", series.derivs.as_ref().map(Vec::len), n)?;
        check_len(symbol, "books", series.books.as_ref().map(Vec::len), n)?;
        check_len(symbol, "trades", series.trades.as_ref().map(Vec::len), n)?;
        check_len(
            symbol,
            "sections",
            series.sections.as_ref().map(Vec::len),
            n,
        )?;

        let reference = series
            .reference
            .map(|r| r.into_iter().map(|c| c.close).collect());
        let derivs = convert(symbol, series.derivs, DerivativesTick::to_core)?;
        let books = convert(symbol, series.books, |b| b.to_core())?;
        let sections = convert(symbol, series.sections, |s| s.to_core())?;
        let trades = series
            .trades
            .map(|bars| {
                bars.into_iter()
                    .map(|bar| convert_vec(symbol, bar, TradePrint::to_core))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?;

        Ok(Self {
            candles: series.candles,
            reference,
            derivs,
            books,
            trades,
            sections,
        })
    }

    /// Which feeds this series carries, for the spec's feed check.
    pub(crate) fn available(&self) -> Available {
        Available {
            reference: self.reference.is_some(),
            derivs: self.derivs.is_some(),
            books: self.books.is_some(),
            trades: self.trades.is_some(),
            sections: self.sections.is_some(),
        }
    }

    /// The feeds for bar `index`.
    pub(crate) fn bar(&self, index: usize) -> BarFeeds<'_> {
        BarFeeds {
            reference: self.reference.as_ref().map(|r| r[index]),
            deriv: self.derivs.as_ref().map(|d| d[index]),
            orderbook: self.books.as_ref().map(|b| &b[index]),
            trades: self
                .trades
                .as_ref()
                .map_or(&[][..], |t| t[index].as_slice()),
            cross_section: self.sections.as_ref().map(|s| &s[index]),
        }
    }
}

/// The feeds for one bar, in the types `BarInput` consumes.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct BarFeeds<'a> {
    pub(crate) reference: Option<f64>,
    pub(crate) deriv: Option<CoreDerivativesTick>,
    pub(crate) orderbook: Option<&'a CoreOrderBook>,
    pub(crate) trades: &'a [CoreTrade],
    pub(crate) cross_section: Option<&'a CoreCrossSection>,
}

impl BarFeeds<'_> {
    /// Which feeds this bar carries.
    pub(crate) fn available(&self) -> Available {
        Available {
            reference: self.reference.is_some(),
            derivs: self.deriv.is_some(),
            books: self.orderbook.is_some(),
            trades: !self.trades.is_empty(),
            sections: self.cross_section.is_some(),
        }
    }
}

/// One bar's feeds owned rather than borrowed, so a streaming caller can hand
/// them in a command payload and have them outlive the deserialized document.
pub(crate) struct OwnedBarFeeds {
    reference: Option<f64>,
    deriv: Option<CoreDerivativesTick>,
    orderbook: Option<CoreOrderBook>,
    trades: Vec<CoreTrade>,
    cross_section: Option<CoreCrossSection>,
}

impl OwnedBarFeeds {
    /// Convert a streaming `StepFeeds` document, surfacing malformed feeds.
    pub(crate) fn build(symbol: &str, feeds: StepFeeds) -> Result<Self> {
        Ok(Self {
            reference: feeds.reference,
            deriv: feeds
                .deriv
                .map(|d| d.to_core().map_err(|e| feed_error(symbol, "derivs", &e)))
                .transpose()?,
            orderbook: feeds
                .orderbook
                .map(|b| b.to_core().map_err(|e| feed_error(symbol, "books", &e)))
                .transpose()?,
            trades: convert_vec(
                symbol,
                feeds.trades.unwrap_or_default(),
                TradePrint::to_core,
            )?,
            cross_section: feeds
                .cross_section
                .map(|s| s.to_core().map_err(|e| feed_error(symbol, "sections", &e)))
                .transpose()?,
        })
    }

    /// Borrow as the per-bar view.
    pub(crate) fn as_bar(&self) -> BarFeeds<'_> {
        BarFeeds {
            reference: self.reference,
            deriv: self.deriv,
            orderbook: self.orderbook.as_ref(),
            trades: &self.trades,
            cross_section: self.cross_section.as_ref(),
        }
    }
}

/// Which feed families are present, for the spec's feed check.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one independent flag per feed family; a bitset would read worse at every use site"
)]
pub(crate) struct Available {
    pub(crate) reference: bool,
    pub(crate) derivs: bool,
    pub(crate) books: bool,
    pub(crate) trades: bool,
    pub(crate) sections: bool,
}

impl Available {
    /// Whether a feed family is present.
    pub(crate) fn has(self, kind: FeedKind) -> bool {
        match kind {
            FeedKind::Candle => true,
            FeedKind::Pair => self.reference,
            FeedKind::Derivatives => self.derivs,
            FeedKind::OrderBook => self.books,
            FeedKind::Trades => self.trades,
            FeedKind::TradeQuote => self.trades && self.books,
            FeedKind::CrossSection => self.sections,
        }
    }
}

/// Reject a feed whose length does not match the candle count.
fn check_len(symbol: &str, name: &str, len: Option<usize>, candles: usize) -> Result<()> {
    match len {
        Some(l) if l != candles => Err(Error::FeedLength {
            symbol: symbol.to_string(),
            feed: name.to_string(),
            len: l,
            candles,
        }),
        _ => Ok(()),
    }
}

/// Convert an optional feed array, surfacing the first malformed entry.
fn convert<T, U, E, F>(symbol: &str, feed: Option<Vec<T>>, to_core: F) -> Result<Option<Vec<U>>>
where
    F: Fn(T) -> core::result::Result<U, E>,
    E: core::fmt::Display,
{
    feed.map(|items| convert_vec(symbol, items, &to_core))
        .transpose()
}

/// Convert one feed array, surfacing the first malformed entry.
fn convert_vec<T, U, E, F>(symbol: &str, items: Vec<T>, to_core: F) -> Result<Vec<U>>
where
    F: Fn(T) -> core::result::Result<U, E>,
    E: core::fmt::Display,
{
    items
        .into_iter()
        .map(|item| to_core(item).map_err(|e| feed_error(symbol, "feed", &e)))
        .collect()
}

/// A malformed feed entry.
fn feed_error(symbol: &str, feed: &str, cause: &dyn core::fmt::Display) -> Error {
    Error::Feed(format!("{symbol}: {feed}: {cause}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(close: f64) -> Candle {
        Candle {
            time: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn bare_candles_deserialize_as_the_shorthand() {
        let input: SymbolInput =
            serde_json::from_str(r#"[{"time":0,"open":1,"high":1,"low":1,"close":1,"volume":1}]"#)
                .unwrap();
        assert!(matches!(input, SymbolInput::Candles(_)));
        assert_eq!(input.into_series().candles.len(), 1);
    }

    #[test]
    fn series_with_a_reference_deserializes() {
        let json = r#"{"candles":[{"time":0,"open":1,"high":1,"low":1,"close":1,"volume":1}],
                       "reference":[{"time":0,"open":2,"high":2,"low":2,"close":2,"volume":1}]}"#;
        let series = serde_json::from_str::<SymbolInput>(json)
            .unwrap()
            .into_series();
        assert_eq!(series.reference.as_ref().map(Vec::len), Some(1));
        let core = CoreSeries::build("A", series).unwrap();
        assert_eq!(core.bar(0).reference, Some(2.0));
        assert!(core.available().reference);
    }

    #[test]
    fn a_short_feed_is_rejected() {
        let series = SymbolSeries {
            candles: vec![candle(1.0), candle(2.0)],
            reference: Some(vec![candle(1.0)]),
            ..SymbolSeries::default()
        };
        let err = CoreSeries::build("A", series).unwrap_err().to_string();
        assert!(err.contains("reference"), "{err}");
        assert!(err.contains('2'), "{err}");
    }

    #[test]
    fn a_candle_only_series_carries_no_feeds() {
        let core = CoreSeries::build(
            "A",
            SymbolSeries {
                candles: vec![candle(1.0)],
                ..SymbolSeries::default()
            },
        )
        .unwrap();
        let bar = core.bar(0);
        assert_eq!(bar.reference, None);
        assert!(bar.trades.is_empty());
        assert_eq!(core.available(), Available::default());
    }

    #[test]
    fn available_answers_each_family() {
        let all = Available {
            reference: true,
            derivs: true,
            books: true,
            trades: true,
            sections: true,
        };
        assert!(all.has(FeedKind::Pair));
        assert!(all.has(FeedKind::Derivatives));
        assert!(all.has(FeedKind::OrderBook));
        assert!(all.has(FeedKind::Trades));
        assert!(all.has(FeedKind::TradeQuote));
        assert!(all.has(FeedKind::CrossSection));

        // TradeQuote needs both trades and a book; trades alone is not enough.
        let trades_only = Available {
            trades: true,
            ..Available::default()
        };
        assert!(trades_only.has(FeedKind::Trades));
        assert!(!trades_only.has(FeedKind::TradeQuote));
        // The candle is always there.
        assert!(Available::default().has(FeedKind::Candle));
    }

    #[test]
    fn feed_kind_names_are_the_payload_field_names() {
        assert_eq!(FeedKind::Pair.as_str(), "reference");
        assert_eq!(FeedKind::Derivatives.as_str(), "derivs");
        assert_eq!(FeedKind::OrderBook.as_str(), "books");
        assert_eq!(FeedKind::Trades.as_str(), "trades");
        assert_eq!(FeedKind::CrossSection.as_str(), "sections");
        assert_eq!(FeedKind::Candle.as_str(), "candle");
        assert_eq!(FeedKind::TradeQuote.as_str(), "trades and books");
    }
}
