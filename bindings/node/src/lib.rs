//! Node.js bindings for `wickra-screener` (napi-rs).
//!
//! Thin glue over the screener core's data-driven surface: build a `Screener`
//! from a spec JSON, drive it with a command JSON and read back the response
//! JSON. The same command protocol crosses every binding, so a Node front-end
//! drives the exact same core as the native CLI.

#![allow(missing_debug_implementations)]
// napi exposes owned `String` arguments; the bodies only need to borrow them.
#![allow(clippy::needless_pass_by_value)]

use napi::Result;
use napi_derive::napi;

use wickra_screener_core::Screener as CoreScreener;

/// Build a napi error from a message.
fn err(message: impl Into<String>) -> napi::Error {
    napi::Error::from_reason(message.into())
}

/// The library version.
#[napi]
pub fn version() -> String {
    CoreScreener::version().to_string()
}

/// A screener instance driven by JSON commands.
// CodeQL reports `rust/access-invalid-pointer` against the struct identifier
// below. This file writes no `unsafe`, no raw pointer and no
// `from_raw`/`into_raw` -- the only occurrences of those words are in this
// comment. What CodeQL analyses is the napi-derive expansion, which the
// identifier is merely the anchor for: `cargo expand -p wickra-screener-node`
// turns the file into 849 lines carrying 29 generated `unsafe` blocks (at
// napi-derive 3.6.3), and at the reported site:
//
//     validate_type_tag(env, napi_val, &<Screener as TypeTag>::type_tag(), "Screener")?;
//     register_native_borrow_with_value(env, napi_val, wrapped_val.cast::<Screener>(), false)?;
//     Ok(&*(wrapped_val as *const Screener))
//
// The dereference sits two lines below the runtime asserting that the pointer
// the JS engine handed back is in fact a `Screener`. CodeQL cannot follow that
// invariant across the FFI boundary, so it sees a bare deref of a pointer of
// unknown provenance.
//
// Dismissed as a false positive rather than excluded by a CodeQL config: this
// binding exports one class and produces exactly one alert, so dismissing that
// one alert keeps the rule live for anything genuinely new in this file.
#[napi]
pub struct Screener {
    inner: CoreScreener,
}

#[napi]
impl Screener {
    /// Build a screener from a spec JSON string.
    #[napi(constructor)]
    pub fn new(spec_json: String) -> Result<Self> {
        CoreScreener::new(&spec_json)
            .map(|inner| Self { inner })
            .map_err(|e| err(e.to_string()))
    }

    /// Apply a command JSON and return the resulting response JSON.
    #[napi]
    pub fn command(&mut self, cmd_json: String) -> Result<String> {
        self.inner
            .command_json(&cmd_json)
            .map_err(|e| err(e.to_string()))
    }

    /// The library version.
    #[napi]
    pub fn version(&self) -> String {
        CoreScreener::version().to_string()
    }
}
