mod config;
mod monitor;
mod report;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use config::Config;
use monitor::run_monitor;
use report::{OutputFormat, format_report};

#[derive(Parser)]
#[command(name = "perf-monitor")]
#[command(about = "Performance monitor for HardwareVisualizer")]
struct Cli {
  /// Path to the target binary
  #[arg(long)]
  binary: PathBuf,

  /// Path to the threshold config file
  #[arg(long, default_value = "perf-thresholds.toml")]
  config: PathBuf,

  /// Warmup duration in seconds (skip initial spike)
  #[arg(long)]
  warmup: Option<u64>,

  /// Measurement duration in seconds
  #[arg(long)]
  duration: Option<u64>,

  /// Output format
  #[arg(long, default_value = "text")]
  output: OutputFormat,
}

fn main() -> ExitCode {
  let cli = Cli::parse();

  if !cli.binary.exists() {
    eprintln!("Error: binary not found: {}", cli.binary.display());
    return ExitCode::FAILURE;
  }

  let config = match Config::load(&cli.config) {
    Ok(mut c) => {
      if let Some(w) = cli.warmup {
        c.timing.warmup_seconds = w;
      }
      if let Some(d) = cli.duration {
        c.timing.measurement_seconds = d;
      }
      c
    }
    Err(e) => {
      eprintln!("Error: failed to load config: {e}");
      return ExitCode::FAILURE;
    }
  };

  let thresholds = config.effective_thresholds();

  match run_monitor(&cli.binary, &config.timing) {
    Ok(result) => {
      let passed = format_report(&result, &thresholds, cli.output);
      if passed {
        ExitCode::SUCCESS
      } else {
        ExitCode::FAILURE
      }
    }
    Err(e) => {
      eprintln!();
      eprintln!("=== Performance Test Failed ===");
      eprintln!("Failure reason: monitoring aborted before metrics could be collected");
      eprintln!("Details: {e}");
      ExitCode::FAILURE
    }
  }
}
