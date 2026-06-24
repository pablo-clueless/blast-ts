use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use colored::Colorize;
use reqwest::Client;
use tokio::{sync::Mutex, task::JoinHandle};

use crate::config::BlastConfig;
use crate::error::BlastError;
use crate::runner;

/// Inputs for a stress run.
pub struct StressConfig {
    pub config: BlastConfig,
    pub min_rps: u64,
    pub max_rps: u64,
    pub step: u64,
    pub step_duration: u64,
}

/// Result of a single RPS step.
#[derive(Debug, Clone)]
pub struct StressStep {
    pub rps: u32,
    pub requests: u32,
    pub success_rate: f64,
    pub p50: u32,
    pub p95: u32,
    pub p99: u32,
    pub errors: u32,
    pub broke: bool,
}

/// Summary of a full ramp.
#[derive(Debug, Clone)]
pub struct StressResult {
    pub steps: Vec<StressStep>,
    pub breaking_point: Option<u32>,
}

/// Progress event emitted as the ramp runs, so the CLI can render live output
/// without printing leaking into the lib.
#[derive(Debug, Clone)]
pub enum StressProgress {
    StepStarted { rps: u32, step_secs: u32 },
    StepFinished(StressStep),
}

/// Ramp RPS from `min_rps` to `max_rps` in `step` increments, holding each level
/// for `step_duration` seconds, and stop early at the breaking point.
///
/// Pure: no printing, no `process::exit`. For live progress, use
/// [`run_stress_with_progress`].
pub async fn run_stress(cfg: StressConfig) -> Result<StressResult, BlastError> {
    run_stress_with_progress(cfg, |_| {}).await
}

/// Like [`run_stress`], but invokes `on_progress` as each step starts and
/// finishes.
pub async fn run_stress_with_progress(
    cfg: StressConfig,
    on_progress: impl Fn(StressProgress),
) -> Result<StressResult, BlastError> {
    let StressConfig {
        config,
        min_rps,
        max_rps,
        step,
        step_duration,
    } = cfg;
    let endpoints = config.endpoint_for("stress");

    if endpoints.is_empty() {
        return Ok(StressResult {
            steps: Vec::new(),
            breaking_point: None,
        });
    }

    let client = Arc::new(Client::builder().timeout(Duration::from_secs(30)).build()?);

    let endpoints = Arc::new(endpoints.into_iter().cloned().collect::<Vec<_>>());

    let ctx = config.load_setup(&client).await?;
    let base_url = Arc::new(config.base_url.clone());

    let mut steps = Vec::<StressStep>::new();
    let mut breaking_point = None;
    let mut current_rps = min_rps;
    let step_dur = Duration::from_secs(step_duration);

    while current_rps <= max_rps {
        on_progress(StressProgress::StepStarted {
            rps: current_rps as u32,
            step_secs: step_dur.as_secs() as u32,
        });

        let interval_ms = 1000 / current_rps;
        let start_time = Instant::now();
        let mut current_idx = 0;

        let http_result = Arc::new(Mutex::new(Vec::<runner::RequestResult>::new()));
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));

        loop {
            ticker.tick().await;

            if start_time.elapsed() >= step_dur {
                break;
            }

            let endpoint = endpoints[current_idx % endpoints.len()].clone();
            current_idx += 1;

            let client = Arc::clone(&client);
            let base_url = Arc::clone(&base_url);
            let ctx = ctx.clone();
            let http_result = Arc::clone(&http_result);

            let handle = tokio::spawn(async move {
                let result = runner::execute(&client, &endpoint, &base_url, &ctx).await;
                http_result.lock().await.push(result);
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let result = http_result.lock().await;
        let total = result.len();
        let passed = result.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        let error_rate = if total == 0 {
            0.0
        } else {
            (failed as f64 / total as f64) * 100.0
        };
        let success_rate = if total == 0 {
            0.0
        } else {
            (passed as f64 / total as f64) * 100.0
        };

        let mut latencies: Vec<u128> = result.iter().map(|r| r.latency_ms).collect();
        drop(result);
        latencies.sort_unstable();

        let p50 = percentile(&latencies, 50);
        let p95 = percentile(&latencies, 95);
        let p99 = percentile(&latencies, 99);

        let broke = p99 > 500 || error_rate > 1.0;

        let step_data = StressStep {
            rps: current_rps as u32,
            requests: total as u32,
            success_rate,
            p50: p50 as u32,
            p95: p95 as u32,
            p99: p99 as u32,
            errors: failed as u32,
            broke,
        };

        steps.push(step_data.clone());
        on_progress(StressProgress::StepFinished(step_data));

        if broke {
            breaking_point = Some(current_rps as u32);
            break;
        }

        current_rps += step;
    }

    Ok(StressResult {
        steps,
        breaking_point,
    })
}

/// CLI entry point: run the ramp with live output, then print the summary table.
pub async fn run(
    config_path: &Path,
    min_rps: u64,
    max_rps: u64,
    step: u64,
    step_duration: u64,
) -> anyhow::Result<()> {
    let config = BlastConfig::load(config_path)?;

    if config.endpoint_for("stress").is_empty() {
        println!("{}", "no endpoints tagged \"stress\" found".yellow());
        return Ok(());
    }

    let result = run_stress_with_progress(
        StressConfig {
            config,
            min_rps,
            max_rps,
            step,
            step_duration,
        },
        print_progress,
    )
    .await?;

    print_summary(&result, max_rps);

    Ok(())
}

/// Render a live step event. CLI-only — never called from the lib.
fn print_progress(event: StressProgress) {
    match event {
        StressProgress::StepStarted { rps, step_secs } => {
            println!("\n -> step {rps} req/s for {step_secs}s");
        }
        StressProgress::StepFinished(step) => {
            let row = format!(
                "  {:>5} req/s   {:>6} req   {:>6.1}%   p50: {:>5}ms   p99: {:>5}ms   errors: {}",
                step.rps, step.requests, step.success_rate, step.p50, step.p99, step.errors
            );
            if step.broke {
                println!("{} {}", row.red(), "⚠".red().bold());
                println!(
                    "\n{}",
                    format!("⚠ breaking point at {} req/s", step.rps)
                        .red()
                        .bold()
                );
                println!("  p99:        {}ms", step.p99);
                println!("  error rate: {:.1}%", 100.0 - step.success_rate);
            } else {
                println!("{}", row.green());
            }
        }
    }
}

/// Print the summary table and recommendation. CLI-only.
fn print_summary(result: &StressResult, max_rps: u64) {
    println!("\n{}", "─".repeat(70));
    println!(
        "  {:<8} {:<10} {:<10} {:<8} {:<8} {:<8} {:<8}",
        "RPS", "Requests", "Success", "p50", "p95", "p99", "Errors"
    );
    println!("{}", "─".repeat(70));

    for sr in &result.steps {
        let row = format!(
            "  {:<8} {:<10} {:<10} {:<8} {:<8} {:<8} {:<8}",
            sr.rps,
            sr.requests,
            format!("{:.1}%", sr.success_rate),
            format!("{}ms", sr.p50),
            format!("{}ms", sr.p95),
            format!("{}ms", sr.p99),
            sr.errors,
        );
        if sr.broke {
            println!("{} ⚠", row.red());
        } else {
            println!("{row}");
        }
    }
    println!("{}", "─".repeat(70));

    println!();
    if result.breaking_point.is_some() {
        println!("{}", "recommendation:".bold());
        println!("check GET /metrics on your API");
        println!(" run EXPLAIN ANALYZE on your slowest query");
    } else {
        println!(
            "{}",
            format!("API held at {max_rps} req/s — try a higher --max-rps").green()
        );
    }
}

fn percentile(sorted: &[u128], p: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((p as f64 / 100.0) * sorted.len() as f64) as usize;
    let index = index.min(sorted.len() - 1);
    sorted[index]
}
