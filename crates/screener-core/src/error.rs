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
