// src/commands/check.rs
use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;
use reqwest::Client;

use crate::config::BlastConfig;
use crate::error::BlastError;
use crate::extractor;
use crate::runner::{self, RequestResult};

/// Outcome of a `check` run: one [`RequestResult`] per endpoint plus a summary.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub results: Vec<RequestResult>,
    pub passed:  u32,
    pub total:   u32,
}

/// Hit every endpoint once and collect the results.
///
/// Pure: no printing, no `process::exit`. Global headers are merged into each
/// endpoint before sending, and extract rules are applied on success so later
/// endpoints can template in the captured values.
pub async fn run_check(config: BlastConfig) -> Result<CheckResult, BlastError> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut ctx:     HashMap<String, String> = HashMap::new();
    let mut results: Vec<RequestResult> = Vec::new();

    for endpoint in &config.endpoints {
        let mut merged_endpoint = endpoint.clone();
        if let Some(global) = &config.headers {
            let mut merged = global.clone();
            if let Some(ep_headers) = &endpoint.headers {
                merged.extend(ep_headers.clone());
            }
            merged_endpoint.headers = Some(merged);
        }

        // send the request
        let result = runner::execute(&client, &merged_endpoint, &config.base_url, &ctx).await;

        // extract variables on success only
        if result.passed {
            if let (Some(extract_rules), Some(body)) = (&endpoint.extract, &result.body) {
                extractor::extract(body, extract_rules, &mut ctx);
            }
        }

        results.push(result);
    }

    let passed = results.iter().filter(|r| r.passed).count() as u32;
    let total  = results.len() as u32;

    Ok(CheckResult { results, passed, total })
}

/// CLI entry point: run the check, print the report, exit non-zero on failure.
pub async fn run(config_path: &Path) -> anyhow::Result<()> {
    let config = BlastConfig::load(config_path)?;
    let result = run_check(config).await?;

    print_report(&result);

    // non-zero exit if any failed — CI friendly
    let failed = result.total - result.passed;
    if failed > 0 {
        anyhow::bail!("{} endpoint(s) failed", failed);
    }

    Ok(())
}

/// Pretty-print a [`CheckResult`] to stdout. CLI-only — never called from the lib.
fn print_report(result: &CheckResult) {
    // ── per-endpoint lines ──────────────────────────────────────────────────
    println!();
    for r in &result.results {
        if r.passed {
            println!(
                "  {}  {:<30}  {} {}  {}ms",
                "✓".green().bold(),
                r.endpoint_name.as_str(),
                r.method.cyan(),
                r.path.dimmed(),
                r.latency_ms,
            );
        } else {
            println!(
                "  {}  {:<30}  {} {}  {}ms",
                "✗".red().bold(),
                r.endpoint_name.as_str(),
                r.method.cyan(),
                r.path.dimmed(),
                r.latency_ms,
            );
            println!(
                "     expected {} got {}",
                r.expected_status.map(|s| s.to_string()).unwrap_or("any".to_string()).yellow(),
                r.actual_status.to_string().red(),
            );
            if let Some(err) = &r.error {
                // trim long bodies so output stays readable
                let preview = if err.len() > 200 { &err[..200] } else { err };
                println!("     {}", preview.dimmed());
            }
        }
    }

    // ── summary ─────────────────────────────────────────────────────────────
    let passed = result.passed;
    let failed = result.total - result.passed;
    let total  = result.total;

    println!();
    if failed == 0 {
        println!("  {}", format!("{}/{} passed", passed, total).green().bold());
    } else {
        println!(
            "  {}  —  {}",
            format!("{}/{} passed", passed, total).yellow().bold(),
            format!("{} failed", failed).red().bold(),
        );
    }
    println!();
}
