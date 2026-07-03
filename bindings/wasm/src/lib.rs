//! WebAssembly bindings for `wickra-screener` (wasm-bindgen).
//!
//! The data-driven scan core, compiled to WebAssembly for the browser: build a
//! `Screener` from a spec JSON, drive it with a command JSON and read back the
//! response JSON. The same command protocol crosses every binding.
//!
//! The `parallel` feature of the core is disabled here: rayon's thread pool is
//! not available in a browser sandbox, so the scan runs sequentially — which is
//! byte-identical to the parallel one, the exact cross-language golden check.

use wasm_bindgen::prelude::*;

use screener_core::Screener as CoreScreener;

/// A screener instance driven by JSON commands.
#[wasm_bindgen]
pub struct Screener {
    inner: CoreScreener,
}

#[wasm_bindgen]
impl Screener {
    /// Build a screener from a spec JSON string.
    #[wasm_bindgen(constructor)]
    pub fn new(spec_json: &str) -> Result<Screener, JsError> {
        CoreScreener::new(spec_json)
            .map(|inner| Self { inner })
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Apply a command JSON and return the resulting response JSON.
    pub fn command(&mut self, cmd_json: &str) -> Result<String, JsError> {
        self.inner
            .command_json(cmd_json)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// The library version.
    #[wasm_bindgen(js_name = version)]
    pub fn instance_version(&self) -> String {
        CoreScreener::version().to_string()
    }
}

/// The library version.
#[wasm_bindgen]
pub fn version() -> String {
    CoreScreener::version().to_string()
}
