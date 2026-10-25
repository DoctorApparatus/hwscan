mod client;
mod collectors;
mod error;
mod model;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use schemars::schema_for;
use tracing_subscriber::EnvFilter;

use crate::client::ControllerClient;
use crate::collectors::Fs;
use crate::model::HardwareReport;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Scan this machine's hardware and report it to a controller"
)]
struct Cli {
    /// Read /proc and /sys below this directory (for fixtures and tests).
    #[arg(long, global = true, default_value = "/")]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scan and print the report as JSON. Sends nothing.
    Scan,
    /// Scan and POST the report to the controller.
    Report {
        /// Full URL the report is POSTed to.
        #[arg(long, env = "HWSCAN_CONTROLLER_URL")]
        controller_url: String,
        /// Sent as `Authorization: Bearer <token>`.
        #[arg(long, env = "HWSCAN_TOKEN", hide_env_values = true)]
        token: Option<String>,
        /// Per-request timeout in seconds.
        #[arg(long, env = "HWSCAN_TIMEOUT_SECS", default_value_t = 30)]
        timeout_secs: u64,
    },
    /// Print the JSON Schema of the report (the contract for the controller).
    Schema,
}

#[tokio::main]
async fn main() -> Result<()> {
    // A missing .env is normal.
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Schema => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schema_for!(HardwareReport))?
            );
        }
        Command::Scan => {
            let report = collectors::scan(&Fs::new(&cli.root)).await;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Report {
            controller_url,
            token,
            timeout_secs,
        } => {
            let report = collectors::scan(&Fs::new(&cli.root)).await;
            tracing::info!(
                cpus = report.cpu.logical_cpus,
                disks = report.disks.len(),
                nics = report.nics.len(),
                warnings = report.warnings.len(),
                "scan complete"
            );
            ControllerClient::new(controller_url, token, Duration::from_secs(timeout_secs))?
                .submit(&report)
                .await
                .context("failed to deliver report")?;
        }
    }

    Ok(())
}
