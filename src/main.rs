use std::path::PathBuf;
use anyhow::Result;
use blast::commands;
use clap:: {Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version = "0.1.0", name = "blast", about = "API load tester and traffic generator")]
struct Cli {
    /// Path to the blast.config.json (default: current directory)
    #[arg(short, long, global = true, default_value = ".")]
    config:PathBuf,

    #[command(subcommand)]
    command:Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create the blast.config.json in the given directory
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Hit every enpoint once and verify status codes
    Check,

    /// Validate blast.config.json and report issues
    Validate,

    /// Run all endpoint with tags "seed" with fake fresh data
    Seed {
        #[arg(long, default_value = "10")]
        count: u32,

        #[arg(short = 'j', long, default_value = "1")]
        concurrency: usize
    },

    /// Fire loads at fixed request per second for a set duration and print out the response time
    Run {
        #[arg(long, default_value="10")]
        rps: u32,

        #[arg(long, short='d', default_value="30")]
        duration:u64
    },

    /// Ramps from mins-rps to max-rps in steps, calls the blast run logic for each step
    Stress {
        #[arg(long, default_value="10")]
        min_rps: u64,

        #[arg(long, default_value="100")]
        max_rps: u64,

        #[arg(long, default_value="10")]
        step: u64,

        #[arg(long, default_value="15")]
        step_duration: u64
    }
}

#[tokio::main]
async fn main() -> Result<()>{
    let cli = Cli::parse();

    match cli.command {
        Command::Init { path } => {
            commands::init::run(&path)?;
        },

        Command::Check => {
            commands::check::run(&cli.config).await?;
        },

        Command::Validate => {
            commands::validate::run(&cli.config)?;
        },

        Command::Seed { count, concurrency } => {
            commands::seed::run(&cli.config, count, concurrency).await?;
        },

        Command::Run { rps, duration } => {
            commands::run::run(&cli.config, rps, duration).await?;
        },

        Command::Stress { min_rps, max_rps, step, step_duration } => {
            commands::stress::run(&cli.config, min_rps, max_rps, step, step_duration).await?;
        }
    }

    Ok(())
}