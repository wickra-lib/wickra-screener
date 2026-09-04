//! Sourcing a scan universe from a live exchange.
//!
//! Everything else in this crate takes candles it is handed. This is the one
//! place that goes and gets them: given a venue and the symbols a spec names, it
//! pulls each symbol's recent history over the exchange facade and hands back the
//! same [`SymbolInput`] map [`scan_batch`](crate::scan_batch) takes, so a live
//! scan and a scan over committed CSVs run through identical code.
//!
//! **Read-only.** The screener places no orders and holds no secret material —
//! the connection is made with empty credentials and only public market-data
//! endpoints are called. That is the position `THREAT_MODEL.md` states, and this
//! module is where it has to hold.
//!
//! Only candles are available this way. A venue's kline endpoint returns bars,
//! not the order book or the trades that printed inside each of them, so a spec
//! naming an order-book, trade-flow or derivatives indicator is refused against a
//! live universe by the same feed check that refuses it against a file. Repeating
//! one book snapshot across every bar would be an invention, not a feed.

use std::collections::BTreeMap;

use wickra_backtest_core::Candle;
use wickra_exchange::{
    connect, Candle as ExchangeCandle, Credentials, Exchange, ExchangeOptions, Symbol,
};

use crate::error::{Error, Result};
use crate::feeds::SymbolInput;

/// A connection to one venue, used to pull scan universes from it.
pub struct LiveUniverse {
    exchange: Box<dyn Exchange>,
    venue: String,
    interval: String,
    bars: u32,
}

impl LiveUniverse {
    /// Connect to `venue` for public market data.
    ///
    /// `interval` is the venue's kline interval (`"1m"`, `"1h"`, `"1d"`) and
    /// `bars` how many of them to pull per symbol — enough to cover the longest
    /// warmup the spec needs, or every symbol will still be warming up when the
    /// scan evaluates.
    ///
    /// # Errors
    ///
    /// Returns an error if the venue is not one the facade supports or the
    /// transport cannot be built.
    pub fn connect(
        venue: &str,
        interval: impl Into<String>,
        bars: u32,
        options: &ExchangeOptions,
    ) -> Result<Self> {
        if bars == 0 {
            return Err(Error::BadSpec("live bars must be greater than 0".into()));
        }
        // Public endpoints only: no key is sent, and none is held.
        let exchange = connect(venue, Credentials::new("", ""), options)
            .map_err(|e| Error::Live(format!("{venue}: {e}")))?;
        Ok(Self {
            exchange,
            venue: venue.to_string(),
            interval: interval.into(),
            bars,
        })
    }

    /// The venue this universe is pulled from.
    #[must_use]
    pub fn venue(&self) -> &str {
        &self.venue
    }

    /// Pull each symbol's recent candles into a dataset a scan can take.
    ///
    /// A symbol the venue does not know, or one it returns nothing for, is left
    /// out rather than filled in: the scan then names it in the report's
    /// `missing`, which is the same answer a file that was not there would give.
    ///
    /// # Errors
    ///
    /// Returns an error if a symbol cannot be parsed as `BASE/QUOTE`. A venue
    /// that rejects one symbol does not fail the whole pull.
    pub fn fetch(&mut self, symbols: &[String]) -> Result<BTreeMap<String, SymbolInput>> {
        let mut out = BTreeMap::new();
        for name in symbols {
            let symbol: Symbol = name
                .parse()
                .map_err(|e| Error::Live(format!("{name}: {e}")))?;
            let Ok(candles) = self.exchange.klines(&symbol, &self.interval, self.bars) else {
                continue;
            };
            if candles.is_empty() {
                continue;
            }
            out.insert(
                name.clone(),
                SymbolInput::Candles(candles.iter().map(to_scan_candle).collect()),
            );
        }
        Ok(out)
    }
}

/// Convert an exchange candle into the one the scan folds.
fn to_scan_candle(candle: &ExchangeCandle) -> Candle {
    Candle {
        time: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unsupported_venue_is_named_in_the_error() {
        let Err(err) = LiveUniverse::connect("not-a-venue", "1h", 100, &ExchangeOptions::default())
        else {
            panic!("the facade knows ten venues, not this one");
        };
        assert!(err.to_string().contains("not-a-venue"), "{err}");
    }

    #[test]
    fn a_zero_bar_pull_is_refused() {
        let Err(err) = LiveUniverse::connect("binance", "1h", 0, &ExchangeOptions::default())
        else {
            panic!("a universe of zero bars is not a universe");
        };
        assert!(err.to_string().contains("greater than 0"), "{err}");
    }

    #[test]
    fn an_exchange_candle_becomes_a_scan_candle() {
        // The candle type is `#[non_exhaustive]` upstream, so it is built
        // through the validating constructor rather than a field literal.
        let candle = ExchangeCandle::new(1.0, 3.0, 0.5, 2.0, 100.0, 1_700_000_000).unwrap();
        let converted = to_scan_candle(&candle);
        assert_eq!(converted.time, 1_700_000_000);
        // The conversion moves the fields across unchanged, so the values are
        // bit-identical and comparing them exactly is the point of the test.
        assert!((converted.open - 1.0).abs() < f64::EPSILON);
        assert!((converted.high - 3.0).abs() < f64::EPSILON);
        assert!((converted.low - 0.5).abs() < f64::EPSILON);
        assert!((converted.close - 2.0).abs() < f64::EPSILON);
        assert!((converted.volume - 100.0).abs() < f64::EPSILON);
    }
}
