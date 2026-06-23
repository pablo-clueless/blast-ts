//! NAPI-RS binding layer.
//!
//! This is the only module that imports from `napi_derive`. It stays thin: its
//! job is to translate between JS-facing types and Blast's `lib.rs` types. Real
//! bindings (`check`, `run`, `seed`, `stress`) land here once the command
//! modules return structured results instead of printing.

use napi_derive::napi;

/// Smoke-test export: returns the Blast crate version.
///
/// Exists so the NAPI build pipeline can be verified end-to-end before the
/// real bindings exist. Safe to remove once `check`/`run`/`seed`/`stress` land.
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
