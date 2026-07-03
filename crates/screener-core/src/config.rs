//! CLI configuration: a thin wrapper holding a [`ScanSpec`] loaded from a JSON
//! or TOML file.

use crate::error::Result;
use crate::spec::ScanSpec;

/// A loaded scan configuration. The spec file is a bare `ScanSpec`; this wrapper
/// gives the CLI (and future config-level options) a stable type to load into.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// The scan specification.
    pub spec: ScanSpec,
}

impl Config {
    /// Load a config from a JSON spec file (validated).
    pub fn from_json(s: &str) -> Result<Self> {
        Ok(Self {
            spec: ScanSpec::from_json(s)?,
        })
    }

    /// Load a config from a TOML spec file (validated).
    pub fn from_toml(s: &str) -> Result<Self> {
        Ok(Self {
            spec: ScanSpec::from_toml(s)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"universe":["A","B"],"condition":{"type":"cmp","left":{"kind":"price","field":"close"},"op":"gt","right":{"kind":"const","value":0}}}"#;

    #[test]
    fn loads_from_json() {
        let cfg = Config::from_json(JSON).unwrap();
        assert_eq!(cfg.spec.universe, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn loads_from_toml() {
        let toml = r#"
universe = ["A", "B"]

[condition]
type = "cmp"
op = "gt"

[condition.left]
kind = "price"
field = "close"

[condition.right]
kind = "const"
value = 0.0
"#;
        let cfg = Config::from_toml(toml).unwrap();
        assert_eq!(cfg.spec.universe.len(), 2);
    }
}
