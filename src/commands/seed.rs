use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use colored::Colorize;
use reqwest::Client;
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinHandle,
};

use crate::config::BlastConfig;
use crate::error::BlastError;
use crate::{extractor, runner};

/// Inputs for a seed run.
pub struct SeedConfig {
    pub config: BlastConfig,
    pub count: u32,
    pub concurrency: usize,
}

/// Summary of a seed run.
#[derive(Debug, Clone)]
pub struct SeedResult {
    pub iterations: u32,
    pub passed: u32,
    pub total_requests: u32,
}

/// One full pass over the seed endpoints.
struct IterationResult {
    passed: bool,
    requests: usize,
}

/// Run the `seed`-tagged endpoints `count` times with bounded concurrency.
///
/// Pure: no printing, no `process::exit`. Each iteration carries its own
/// extract context so captured values flow between endpoints within a pass.
pub async fn run_seed(cfg: SeedConfig) -> Result<SeedResult, BlastError> {
    let SeedConfig {
        config,
        count,
        concurrency,
    } = cfg;
    let endpoints = config.endpoint_for("seed");

    if endpoints.is_empty() {
        return Ok(SeedResult {
            iterations: 0,
            passed: 0,
            total_requests: 0,
        });
    }

    let client = Arc::new(Client::builder().timeout(Duration::from_secs(10)).build()?);

    // each task needs to own its endpoints
    let endpoints = Arc::new(endpoints.into_iter().cloned().collect::<Vec<_>>());

    let base_url = Arc::new(config.base_url);
    let results = Arc::new(Mutex::new(Vec::<IterationResult>::new()));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    for _ in 0..count {
        let client = Arc::clone(&client);
        let base_url = Arc::clone(&base_url);
        let endpoints = Arc::clone(&endpoints);
        let semaphore = Arc::clone(&semaphore);
        let results = Arc::clone(&results);

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.unwrap();

            let mut ctx: HashMap<String, String> = HashMap::new();
            let mut iteration_passed = true;

            for endpoint in endpoints.iter() {
                let result = runner::execute(&client, endpoint, &base_url, &ctx).await;

                if result.passed {
                    if let (Some(rules), Some(body)) = (&endpoint.extract, &result.body) {
                        extractor::extract(body, rules, &mut ctx);
                    }
                } else {
                    iteration_passed = false;
                }
            }

            results.lock().await.push(IterationResult {
                passed: iteration_passed,
                requests: endpoints.len(),
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let results = results.lock().await;
    let iterations = results.len() as u32;
    let passed = results.iter().filter(|r| r.passed).count() as u32;
    let total_requests = results.iter().map(|r| r.requests).sum::<usize>() as u32;

    Ok(SeedResult {
        iterations,
        passed,
        total_requests,
    })
}

/// CLI entry point: announce the plan, run the seed, then print a summary.
pub async fn run(config_path: &Path, count: u32, concurrency: usize) -> anyhow::Result<()> {
    let config = BlastConfig::load(config_path)?;

    let endpoint_count = config.endpoint_for("seed").len();
    if endpoint_count == 0 {
        println!(
            "{}",
            "no endpoints tagged \"seed\" found in blast.config.json".yellow()
        );
        println!("add \"tags\": [\"seed\"] to the endpoints you want to seed with");
        return Ok(());
    }

    println!(
        "seeding {count} iterations × {endpoint_count} endpoints (concurrency: {concurrency})\n",
    );

    let result = run_seed(SeedConfig {
        config,
        count,
        concurrency,
    })
    .await?;

    print_summary(&result);

    Ok(())
}

/// Pretty-print a [`SeedResult`]. CLI-only — never called from the lib.
fn print_summary(result: &SeedResult) {
    let failed = result.iterations - result.passed;

    println!();
    println!("  Iterations:      {}", result.iterations);
    println!("  Passed:          {}", result.passed.to_string().green());
    if failed > 0 {
        println!("  Failed:          {}", failed.to_string().red());
    }
    println!("  Total requests:  {}", result.total_requests);
    println!();

    if failed > 0 {
        println!(
            "{}",
            format!("{failed} iteration(s) failed — run blast check to diagnose").yellow()
        );
    } else {
        println!("{}", "all iterations passed".green().bold());
    }
}
