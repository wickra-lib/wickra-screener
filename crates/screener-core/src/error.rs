//! Error type for the screener core.

/// Errors returned by the screener core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A spec, command or dataset failed to parse.
    #[error("parse: {0}")]
    Parse(String),
    /// A spec referenced an indicator the `wickra-core` registry does not know.
    #[error("unknown indicator: {0}")]
    UnknownIndicator(String),
    /// A spec was structurally invalid (empty, out of range or contradictory).
    #[error("bad spec: {0}")]
    BadSpec(String),
    /// The universe data was missing or malformed.
    #[error("data: {0}")]
    Data(String),
    /// A side feed did not have one entry per candle.
    #[error("{symbol}: {feed} feed length {len} does not match {candles} candles")]
    FeedLength {
        /// The symbol whose series is mismatched.
        symbol: String,
        /// The feed array that is the wrong length.
        feed: String,
        /// The length supplied.
        len: usize,
        /// The candle count it has to match.
        candles: usize,
    },
    /// A symbol was fed that the spec's universe does not name. The universe is
    /// what the spec asks to be screened, so folding a symbol outside it would
    /// put values into a scan that the screen never asked for.
    #[error("symbol {0} is not in the spec's universe")]
    NotInUniverse(String),
    /// A side feed entry could not be converted to the type indicators consume.
    #[error("feed: {0}")]
    Feed(String),
    /// The spec names an indicator whose feed the scan does not supply. Without
    /// the feed the indicator would tick and return nothing on every bar, so the
    /// screen would silently never match; saying so is the point of this error.
    #[error("indicator {indicator} needs the {feed} feed, which this scan does not supply")]
    MissingFeed {
        /// The indicator the spec names.
        indicator: String,
        /// The feed family it consumes.
        feed: String,
    },
}

/// Convenience result alias for the screener core.
pub type Result<T> = core::result::Result<T, Error>;

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Parse(e.to_string())
    }
}
