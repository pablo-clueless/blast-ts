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

/// Inputs for a fixed-rate load test.
pub struct RunConfig {
    pub config: BlastConfig,
    pub rps: u32,
    pub duration: u64,
}

/// Final summary of a load test.
#[derive(Debug, Clone)]
pub struct RunResult {
    pub total_requests: u32,
    pub success_rate: f64,
    pub p50: u32,
    pub p95: u32,
    pub p99: u32,
    pub p999: u32,
    pub duration_secs: u32,
}

/// Live snapshot emitted once per elapsed second while a run is in flight.
#[derive(Debug, Clone)]
pub struct RunProgress {
    pub elapsed_secs: u64,
    pub sent: u32,
    pub success: u32,
    pub p99: u32,
}

/// Fire requests at a fixed rate for a fixed duration and report latency stats.
///
/// Pure: no printing, no `process::exit`. For live progress, use
/// [`run_load_test_with_progress`].
pub async fn run_load_test(cfg: RunConfig) -> Result<RunResult, BlastError> {
    run_load_test_with_progress(cfg, |_| {}).await
}

/// Like [`run_load_test`], but invokes `on_progress` once per elapsed second so
/// callers (the CLI) can render live output without printing leaking into the lib.
pub async fn run_load_test_with_progress(
    cfg: RunConfig,
    on_progress: impl Fn(RunProgress),
) -> Result<RunResult, BlastError> {
    let RunConfig {
        config,
        rps,
        duration,
    } = cfg;
    let endpoints = config.endpoint_for("run");

    if endpoints.is_empty() {
        return Ok(RunResult {
            total_requests: 0,
            success_rate: 0.0,
            p50: 0,
            p95: 0,
            p99: 0,
            p999: 0,
            duration_secs: duration as u32,
        });
    }

    let client = Arc::new(Client::builder().timeout(Duration::from_secs(30)).build()?);

    let ctx = config.load_setup(&client).await?;
    let base_url = Arc::new(config.base_url.clone());
    let endpoints = Arc::new(endpoints.into_iter().cloned().collect::<Vec<_>>());

    // timing
    let duration = Duration::from_secs(duration);
    let interval_ms = 1000 / rps;
    let start_time = Instant::now();
    let mut current_idx = 0;
    let mut last_print = 0u64;

    let results = Arc::new(Mutex::new(Vec::<runner::RequestResult>::new()));
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms.into()));

    loop {
        ticker.tick().await;

        let elapsed = start_time.elapsed();
        if elapsed >= duration {
            break;
        }

        let elapsed_secs = elapsed.as_secs();
        if elapsed_secs > last_print {
            last_print = elapsed_secs;
            let r = results.lock().await;
            let total = r.len();
            let success = r.iter().filter(|r| r.passed).count();
            let mut latencies: Vec<u128> = r.iter().map(|r| r.latency_ms).collect();
            latencies.sort_unstable();
            let p99 = percentile(&latencies, 99);
            drop(r);
            on_progress(RunProgress {
                elapsed_secs,
                sent: total as u32,
                success: success as u32,
                p99: p99 as u32,
            });
        }

        let endpoint = endpoints[current_idx % endpoints.len()].clone();
        current_idx += 1;

        let client = Arc::clone(&client);
        let base_url = Arc::clone(&base_url);
        let ctx = ctx.clone();
        let results = Arc::clone(&results);

        let handle = tokio::spawn(async move {
            let result = runner::execute(&client, &endpoint, &base_url, &ctx).await;
            results.lock().await.push(result);
        });

        handles.push(handle);
    }

    // drain in-flight requests; a panicked task just drops its result
    for handle in handles {
        let _ = handle.await;
    }

    let results = results.lock().await;
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let mut latencies: Vec<u128> = results.iter().map(|r| r.latency_ms).collect();
    latencies.sort_unstable();

    let success_rate = if total == 0 {
        0.0
    } else {
        (passed as f64 / total as f64) * 100.0
    };

    Ok(RunResult {
        total_requests: total as u32,
        success_rate,
        p50: percentile(&latencies, 50) as u32,
        p95: percentile(&latencies, 95) as u32,
        p99: percentile(&latencies, 99) as u32,
        p999: percentile(&latencies, 999) as u32,
        duration_secs: duration.as_secs() as u32,
    })
}

/// CLI entry point: run the load test with live progress, then print a summary.
pub async fn run(config_path: &Path, rps: u32, duration: u64) -> anyhow::Result<()> {
    let config = BlastConfig::load(config_path)?;

    if config.endpoint_for("run").is_empty() {
        println!("No endpoint to run");
        return Ok(());
    }

    let result = run_load_test_with_progress(
        RunConfig {
            config,
            rps,
            duration,
        },
        |p| {
            println!(
                "  elapsed: {}s   sent: {}   success: {}   p99: {}ms",
                p.elapsed_secs, p.sent, p.success, p.p99
            );
        },
    )
    .await?;

    print_summary(&result);

    Ok(())
}

/// Pretty-print a [`RunResult`]. CLI-only — never called from the lib.
fn print_summary(result: &RunResult) {
    let passed = (result.success_rate / 100.0 * result.total_requests as f64).round() as u32;
    let failed = result.total_requests - passed;

    println!();
    println!("  Total requests:  {}", result.total_requests);
    println!("  Duration:        {}s", result.duration_secs);
    println!(
        "  Success rate:    {}",
        format!("{:.1}%", result.success_rate).green()
    );
    println!();
    println!("  Latency");
    println!("    p50:   {}ms", result.p50);
    println!("    p95:   {}ms", result.p95);
    println!("    p99:   {}ms", result.p99);
    println!("    p999:  {}ms", result.p999);
    println!();
    if failed > 0 {
        println!("  Errors: {}", failed.to_string().red());
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
