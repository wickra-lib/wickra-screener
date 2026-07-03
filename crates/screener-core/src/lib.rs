//! Data-driven core of the Wickra screener.
//!
//! A serde `ScanSpec` condition tree is folded over a symbol universe with the
//! [Wickra](https://github.com/wickra-lib/wickra) library of 514 O(1) streaming
//! indicators and evaluated across the whole universe, in parallel (rayon) or
//! sequentially (the WASM fallback), producing a byte-identical `ScanReport`.
//!
//! The public surface is assembled module by module through P-SCR-1; the final
//! re-export block lands in `lib.rs` (P-SCR-1.12).

mod error;
mod eval;
mod expr;
mod indicator_set;
mod scan;
mod spec;
mod symbol_state;
mod universe;

pub use error::{Error, Result};
pub use expr::{Expr, PriceField};
pub use scan::{scan_batch, ScanReport, ScanResult};
pub use spec::{Comparator, Condition, CsMetric, RankSpec, ScanSpec};
