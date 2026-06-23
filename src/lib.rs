//! Blast as a library crate.
//!
//! This exposes Blast's internals so they can be shared by the CLI (`src/main.rs`)
//! and the NAPI-RS bindings (`src/napi.rs`). The command modules still print to
//! stdout today; refactoring them to return structured results is tracked
//! separately — this file just establishes the library boundary.

pub mod commands;
pub mod config;
pub mod extractor;
pub mod runner;
pub mod template;

// NAPI binding layer. Lives in src/napi.rs but is exposed as `napi_bindings`
// to avoid clashing with the `napi` crate name.
#[path = "napi.rs"]
pub mod napi_bindings;
