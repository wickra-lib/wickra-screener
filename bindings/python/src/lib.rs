//! Python bindings for `wickra-screener`, exposed under the `wickra_screener`
//! package.
//!
//! Thin glue over the screener core's data-driven surface: build a [`Screener`]
//! from a spec JSON, drive it with a command JSON and read back the response
//! JSON. The same command protocol crosses every binding, so a Python front-end
//! drives the exact same core as the native CLI.

// PyO3 protocol methods take `self` by value/ref regardless of use.
#![allow(clippy::needless_pass_by_value)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use screener_core::Screener;

/// A screener instance driven by JSON commands.
///
/// `unsendable`: the screener holds a streaming universe of stateful indicator
/// evaluators, so a handle is bound to the thread that created it.
#[pyclass(name = "Screener", unsendable)]
struct PyScreener {
    inner: Screener,
}

#[pymethods]
impl PyScreener {
    /// Build a screener from a spec JSON string.
    #[new]
    fn new(spec_json: &str) -> PyResult<Self> {
        Screener::new(spec_json)
            .map(|inner| Self { inner })
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    /// Apply a command JSON and return the resulting response JSON.
    fn command(&mut self, cmd_json: &str) -> PyResult<String> {
        self.inner
            .command_json(cmd_json)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    /// The library version.
    #[staticmethod]
    fn version() -> &'static str {
        Screener::version()
    }
}

/// The native module (`wickra_screener._wickra_screener`).
#[pymodule]
fn _wickra_screener(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add_class::<PyScreener>()?;
    Ok(())
}
