//! Data-driven core of the Wickra screener.
//!
//! A serde `ScanSpec` condition tree is folded over a symbol universe with the
//! [Wickra](https://github.com/wickra-lib/wickra) indicator library — the 497
//! the shared registry resolves by name — and evaluated across the whole
//! universe, in parallel (rayon) or
//! sequentially (the WASM fallback), producing a byte-identical `ScanReport`.
//!
//! Two modes share one core and one result type: [`scan_batch`] folds a whole
//! universe and evaluates at the last bar, while [`Screener`] streams candles in
//! (`feed`) and evaluates on demand — both return the same [`ScanReport`]. Every
//! language binding drives the core through [`Screener::command_json`], a single
//! JSON-in / JSON-out boundary.
//!
//! Indicators are resolved by name from the `wickra-core` registry (via the
//! backtester's factory), and candles use [`Candle`] re-exported below.

mod breadth;
mod config;
mod error;
mod eval;
mod expr;
mod feeds;
mod indicator_set;
#[cfg(feature = "live")]
mod live;
mod scan;
mod screener;
mod spec;
mod symbol_state;
mod universe;

pub use breadth::BreadthSpec;
pub use config::Config;
pub use error::{Error, Result};
pub use expr::{Expr, PriceField};
pub use feeds::{FeedKind, SymbolInput, SymbolSeries};
// The exchange options a live pull is configured with, re-exported so a caller
// does not have to depend on the facade to name them.
#[cfg(feature = "live")]
pub use live::LiveUniverse;
pub use scan::{scan_batch, ScanReport, ScanResult};
pub use screener::Screener;
pub use spec::{Comparator, Condition, CsMetric, RankSpec, ScanSpec};
#[cfg(feature = "live")]
pub use wickra_exchange::ExchangeOptions;

// The candle type consumers feed and scan with (the backtester's OHLCV bar,
// which the registry indicators are driven by).
pub use wickra_backtest_core::Candle;

// The side-feed documents a scan supplies alongside the candles, re-exported so
// a caller does not have to depend on the backtester to name them.
pub use wickra_backtest_core::{
    CrossSection, CrossSectionMember, DerivativesTick, Level, OrderBook, StepFeeds, TradePrint,
    TradeSide,
};

/// Which feed an indicator consumes, or `None` if the registry does not know the
/// name. Lets a caller check a spec's needs before assembling a dataset.
pub use indicator_set::feed_kind;

/// The screener-core version string.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
