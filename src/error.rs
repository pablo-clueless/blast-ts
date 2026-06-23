//! Error type shared across the library boundary.
//!
//! Library functions return `Result<_, BlastError>` instead of printing and
//! exiting, so both the CLI (`main.rs`) and the NAPI bindings (`napi.rs`) can
//! decide how to surface failures. The CLI converts these into `anyhow` errors
//! via `?`; the NAPI layer maps them to JS exceptions.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlastError {
    /// Configuration could not be loaded, parsed, or validated.
    #[error("config error: {0}")]
    Config(String),

    /// A required setup endpoint failed, so the run cannot continue.
    #[error("setup failed: {0}")]
    Setup(String),

    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
