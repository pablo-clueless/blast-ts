//! NAPI-RS binding layer.
//!
//! This is the only module that imports from `napi_derive`. It stays thin: its
//! job is to translate between JS-facing types and Blast's `lib.rs` types. Each
//! exported function loads the config from a path, runs the matching pure
//! command, and maps the structured result into `#[napi(object)]` shapes that
//! NAPI-RS turns into TypeScript interfaces.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;

use crate::config::BlastConfig;
use crate::{RunConfig, SeedConfig, StressConfig, run_check, run_load_test, run_seed, run_stress};

/// Outcome of hitting a single endpoint once.
#[napi(object)]
pub struct JsEndpointResult {
    pub name: String,
    pub method: String,
    pub path: String,
    /// The configured `expect_status`, if the endpoint declared one.
    pub expected_status: Option<u32>,
    /// `0` when the request never reached the server (network error).
    pub actual_status: u32,
    pub latency_ms: u32,
    pub passed: bool,
    /// Response body (or network error) on failure; absent on success.
    pub error: Option<String>,
}

/// Result of a `check` run: one entry per endpoint plus a summary count.
#[napi(object)]
pub struct JsCheckResult {
    pub results: Vec<JsEndpointResult>,
    pub passed: u32,
    pub total: u32,
}

/// Hit every endpoint in the config once and report which passed.
#[napi]
pub async fn check(config_path: String) -> Result<JsCheckResult> {
    let config = load_config(&config_path)?;

    let result = run_check(config)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(JsCheckResult {
        results: result
            .results
            .into_iter()
            .map(|r| JsEndpointResult {
                name: r.endpoint_name,
                method: r.method,
                path: r.path,
                expected_status: r.expected_status.map(u32::from),
                actual_status: u32::from(r.actual_status),
                latency_ms: r.latency_ms as u32,
                passed: r.passed,
                error: r.error,
            })
            .collect(),
        passed: result.passed,
        total: result.total,
    })
}

/// Options for a fixed-rate load test.
#[napi(object)]
pub struct JsRunOptions {
    pub config_path: String,
    pub rps: u32,
    pub duration: u32,
}

/// Summary of a load test.
#[napi(object)]
pub struct JsRunResult {
    pub total_requests: u32,
    pub success_rate: f64,
    pub p50: u32,
    pub p95: u32,
    pub p99: u32,
    pub p999: u32,
    pub duration_secs: u32,
}

/// Fire requests at a fixed rate for a fixed duration and report latency stats.
#[napi]
pub async fn run(options: JsRunOptions) -> Result<JsRunResult> {
    let config = load_config(&options.config_path)?;

    let result = run_load_test(RunConfig {
        config,
        rps: options.rps,
        duration: options.duration as u64,
    })
    .await
    .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(JsRunResult {
        total_requests: result.total_requests,
        success_rate: result.success_rate,
        p50: result.p50,
        p95: result.p95,
        p99: result.p99,
        p999: result.p999,
        duration_secs: result.duration_secs,
    })
}

/// Options for a seed run.
#[napi(object)]
pub struct JsSeedOptions {
    pub config_path: String,
    pub count: u32,
    pub concurrency: u32,
}

/// Summary of a seed run.
#[napi(object)]
pub struct JsSeedResult {
    pub iterations: u32,
    pub passed: u32,
    pub total_requests: u32,
}

/// Run the `seed`-tagged endpoints `count` times with bounded concurrency.
#[napi]
pub async fn seed(options: JsSeedOptions) -> Result<JsSeedResult> {
    let config = load_config(&options.config_path)?;

    let result = run_seed(SeedConfig {
        config,
        count: options.count,
        concurrency: options.concurrency as usize,
    })
    .await
    .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(JsSeedResult {
        iterations: result.iterations,
        passed: result.passed,
        total_requests: result.total_requests,
    })
}

/// Options for a stress ramp.
#[napi(object)]
pub struct JsStressOptions {
    pub config_path: String,
    pub min_rps: u32,
    pub max_rps: u32,
    pub step: u32,
    pub step_duration: u32,
}

/// Result of a single RPS step in a stress ramp.
#[napi(object)]
pub struct JsStressStep {
    pub rps: u32,
    pub requests: u32,
    pub success_rate: f64,
    pub p50: u32,
    pub p95: u32,
    pub p99: u32,
    pub errors: u32,
    pub broke: bool,
}

/// Summary of a full stress ramp.
#[napi(object)]
pub struct JsStressResult {
    pub steps: Vec<JsStressStep>,
    /// The RPS at which the API started failing, if a breaking point was hit.
    pub breaking_point: Option<u32>,
}

/// Ramp RPS from `min_rps` to `max_rps`, stopping early at the breaking point.
#[napi]
pub async fn stress(options: JsStressOptions) -> Result<JsStressResult> {
    let config = load_config(&options.config_path)?;

    let result = run_stress(StressConfig {
        config,
        min_rps: options.min_rps as u64,
        max_rps: options.max_rps as u64,
        step: options.step as u64,
        step_duration: options.step_duration as u64,
    })
    .await
    .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(JsStressResult {
        steps: result
            .steps
            .into_iter()
            .map(|s| JsStressStep {
                rps: s.rps,
                requests: s.requests,
                success_rate: s.success_rate,
                p50: s.p50,
                p95: s.p95,
                p99: s.p99,
                errors: s.errors,
                broke: s.broke,
            })
            .collect(),
        breaking_point: result.breaking_point,
    })
}

/// Load and validate a config from a file or directory path, mapping any
/// `BlastError` into a JS-facing error.
fn load_config(config_path: &str) -> Result<BlastConfig> {
    BlastConfig::load(Path::new(config_path)).map_err(|e| Error::from_reason(e.to_string()))
}
