// src/commands/validate.rs
use crate::config::BlastConfig;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn run(config_path: &Path) -> Result<()> {
    print!("loading {}... ", config_path.display());

    match BlastConfig::load(config_path) {
        Ok(config) => {
            println!("{}", "valid".green().bold());
            println!();
            println!("  base_url:  {}", config.base_url);
            println!("  endpoints: {}", config.endpoints.len());
            println!();

            for (i, ep) in config.endpoints.iter().enumerate() {
                println!(
                    "  {}  {} {} {}",
                    format!("[{}]", i + 1).dimmed(),
                    ep.method.cyan().bold(),
                    ep.path,
                    ep.name.dimmed(),
                );

                if let Some(extract) = &ep.extract {
                    for (var, path) in extract {
                        println!(
                            "       {} {} → {{{{{}}}}}",
                            "extract:".dimmed(),
                            path.dimmed(),
                            var.yellow()
                        );
                    }
                }
            }

            println!();
            println!("{}", "config is valid — ready to blast".green());
        }
        Err(e) => {
            println!("{}", "invalid".red().bold());
            println!();
            println!("{} {}", "error:".red().bold(), e);
            println!();
            println!(
                "{}",
                "fix the issues above then run blast validate again".yellow()
            );

            // return error so exit code is non-zero
            // useful in CI: blast validate fails the pipeline if config is broken
            anyhow::bail!("config validation failed");
        }
    }

    Ok(())
}
