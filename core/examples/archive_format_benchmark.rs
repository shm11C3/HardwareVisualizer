//! Standalone synthetic SQLite benchmark for the lossless archive experiment.
//!
//! The output directory must be new. This example never discovers or opens an
//! application database and never reads local process or device data.

#[path = "archive_format_benchmark/codec.rs"]
mod codec;
#[path = "archive_format_benchmark/database.rs"]
mod database;

use codec::{Compression, Layout};
use database::{Benchmark, FileFootprint};
use serde::Serialize;
use std::{env, fs, path::PathBuf, process::Command, time::Duration};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;

const HELP: &str = r#"Usage:
  cargo run -p hardviz-core --release --example archive_format_benchmark -- \
    --output NEW_DIRECTORY [options]

Options:
  --minutes N                  represented duration (default: 1440)
  --days N                     represented duration in days
  --processes-per-minute N     rows per sampled minute (default: 15)
  --chunk-minutes N            maximum represented minutes/chunk (default: 60)
  --chunk-rows N               maximum rows/chunk, <= 4096 (default: 4096)
  --repetitions N              measured query repetitions (default: 7)
  --layout row|columnar        codec layout (default: columnar)
  --compression none|deflate  compression (default: deflate)
  --seed N                     deterministic seed (default: 2052)
  --duty-cycle N               sample every N represented minutes (default: 1)
  --group-cap N                maximum process groups/query (default: 100000)

Duty cycle > 1 is reported as duty-cycled, not continuous history. Use the
default duty cycle for continuous 24h/30d/1y/10y data."#;

#[derive(Clone, Debug)]
pub(crate) struct Config {
  output: PathBuf,
  minutes: u64,
  processes_per_minute: u32,
  chunk_minutes: u64,
  chunk_rows: usize,
  repetitions: usize,
  layout: Layout,
  compression: Compression,
  seed: u64,
  duty_cycle: u64,
  group_cap: usize,
}

impl Config {
  fn parse() -> Result<Self> {
    let mut config = Self {
      output: PathBuf::new(),
      minutes: 1_440,
      processes_per_minute: 15,
      chunk_minutes: 60,
      chunk_rows: codec::MAX_ROWS,
      repetitions: 7,
      layout: Layout::Columnar,
      compression: Compression::Deflate,
      seed: 2052,
      duty_cycle: 1,
      group_cap: 100_000,
    };
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
      let value = args
        .next()
        .filter(|_| argument != "--help" && argument != "-h");
      match argument.as_str() {
        "--output" => config.output = value.ok_or("missing --output value")?.into(),
        "--minutes" => {
          config.minutes = value.ok_or("missing --minutes value")?.parse()?
        }
        "--days" => {
          config.minutes = value
            .ok_or("missing --days value")?
            .parse::<u64>()?
            .checked_mul(1_440)
            .ok_or("duration overflow")?;
        }
        "--processes-per-minute" => {
          config.processes_per_minute = value
            .ok_or("missing --processes-per-minute value")?
            .parse()?;
        }
        "--chunk-minutes" => {
          config.chunk_minutes = value.ok_or("missing --chunk-minutes value")?.parse()?;
        }
        "--chunk-rows" => {
          config.chunk_rows = value.ok_or("missing --chunk-rows value")?.parse()?;
        }
        "--repetitions" => {
          config.repetitions = value.ok_or("missing --repetitions value")?.parse()?;
        }
        "--seed" => config.seed = value.ok_or("missing --seed value")?.parse()?,
        "--duty-cycle" => {
          config.duty_cycle = value.ok_or("missing --duty-cycle value")?.parse()?;
        }
        "--group-cap" => {
          config.group_cap = value.ok_or("missing --group-cap value")?.parse()?;
        }
        "--layout" => {
          config.layout = match value.ok_or("missing --layout value")?.as_str() {
            "row" => Layout::Row,
            "columnar" => Layout::Columnar,
            other => return Err(format!("unknown layout {other:?}").into()),
          };
        }
        "--compression" => {
          config.compression = match value.ok_or("missing --compression value")?.as_str()
          {
            "none" => Compression::None,
            "deflate" => Compression::Deflate,
            other => return Err(format!("unknown compression {other:?}").into()),
          };
        }
        "--help" | "-h" => {
          println!("{HELP}");
          std::process::exit(0);
        }
        other => return Err(format!("unknown argument {other:?}\n\n{HELP}").into()),
      }
    }
    if config.output.as_os_str().is_empty() {
      return Err("--output is required".into());
    }
    if config.minutes == 0
      || config.processes_per_minute == 0
      || config.chunk_minutes == 0
      || config.chunk_rows == 0
      || config.repetitions == 0
      || config.duty_cycle == 0
      || config.group_cap == 0
    {
      return Err("numeric controls must be positive".into());
    }
    if config.chunk_rows > codec::MAX_ROWS {
      return Err(format!("--chunk-rows exceeds codec limit {}", codec::MAX_ROWS).into());
    }
    Ok(config)
  }
}

#[derive(Serialize)]
struct Report {
  format: &'static str,
  scope: &'static str,
  workload: WorkloadReport,
  format_candidate: FormatReport,
  environment: EnvironmentReport,
  sqlite: database::SqliteReport,
  footprints: FootprintReport,
  timings_ms: TimingReport,
  correctness: database::CorrectnessReport,
  limitations: [&'static str; 6],
}

#[derive(Serialize)]
struct WorkloadReport {
  source: &'static str,
  represented_minutes: u64,
  sampled_minutes: u64,
  sampling_mode: &'static str,
  duty_cycle: u64,
  processes_per_sampled_minute: u32,
  numeric_shape: &'static str,
  process_rows: i64,
  ambient_rows: i64,
  seed: u64,
}

#[derive(Serialize)]
struct FormatReport {
  layout: &'static str,
  compression: &'static str,
  chunk_minutes: u64,
  chunk_rows: usize,
  chunks: i64,
  encoded_bytes: i64,
  decoded_value_bytes: u64,
}

#[derive(Serialize)]
struct EnvironmentReport {
  os: &'static str,
  architecture: &'static str,
  logical_cpus: usize,
  cpu_brand: String,
  total_memory_bytes: u64,
  current_rss_bytes: Option<u64>,
  peak_rss_bytes: Option<u64>,
  rustc_version: String,
  git_commit: String,
  filesystem: String,
}

#[derive(Serialize)]
struct FootprintReport {
  relational_baseline: FileFootprint,
  chunked_before_reclamation: FileFootprint,
  chunked_after_vacuum: FileFootprint,
  before_reclamation_reduction_percent: f64,
  after_vacuum_reduction_percent: f64,
  vacuum_wall_ms: f64,
}

#[derive(Serialize)]
struct TimingReport {
  fixture_seed_batch: Timing,
  production_shaped_minute_append: Timing,
  source_scan: Timing,
  encode: Timing,
  decode: Timing,
  chunk_insert_and_tail_delete: Timing,
  finalize_total: Timing,
  process_query_baseline: Timing,
  process_query_chunked: Timing,
  ambient_raw_query_baseline: Timing,
  ambient_raw_query_chunked: Timing,
}

#[derive(Serialize)]
struct Timing {
  samples: usize,
  p50_ms: f64,
  p95_ms: f64,
  p99_ms: f64,
  max_ms: f64,
}

impl Timing {
  fn from(values: &[Duration]) -> Self {
    let mut micros: Vec<_> = values.iter().map(Duration::as_micros).collect();
    micros.sort_unstable();
    let at = |percent: usize| {
      if micros.is_empty() {
        0.0
      } else {
        let index = ((micros.len() - 1) * percent).div_ceil(100);
        micros[index] as f64 / 1_000.0
      }
    };
    Self {
      samples: micros.len(),
      p50_ms: at(50),
      p95_ms: at(95),
      p99_ms: at(99),
      max_ms: micros.last().copied().unwrap_or(0) as f64 / 1_000.0,
    }
  }
}

fn reduction(baseline: &FileFootprint, candidate: &FileFootprint) -> f64 {
  let baseline_bytes = baseline.database_bytes + baseline.wal_bytes;
  let candidate_bytes = candidate.database_bytes + candidate.wal_bytes;
  if baseline_bytes == 0 {
    0.0
  } else {
    (1.0 - candidate_bytes as f64 / baseline_bytes as f64) * 100.0
  }
}

fn command_output(program: &str, arguments: &[&str]) -> String {
  Command::new(program)
    .args(arguments)
    .output()
    .ok()
    .filter(|output| output.status.success())
    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    .unwrap_or_else(|| "unavailable".to_owned())
}

fn environment_report(output: &std::path::Path) -> EnvironmentReport {
  let system = sysinfo::System::new_all();
  let current_rss_bytes = sysinfo::get_current_pid()
    .ok()
    .and_then(|pid| system.process(pid))
    .map(|process| process.memory());
  #[cfg(target_os = "macos")]
  let filesystem = command_output("stat", &["-f", "%T", output.to_str().unwrap_or("")]);
  #[cfg(target_os = "linux")]
  let filesystem =
    command_output("stat", &["-f", "-c", "%T", output.to_str().unwrap_or("")]);
  #[cfg(not(any(target_os = "macos", target_os = "linux")))]
  let filesystem = "unavailable".to_owned();
  EnvironmentReport {
    os: env::consts::OS,
    architecture: env::consts::ARCH,
    logical_cpus: system.cpus().len(),
    cpu_brand: system
      .cpus()
      .first()
      .map(|cpu| cpu.brand().to_owned())
      .unwrap_or_else(|| "unavailable".to_owned()),
    total_memory_bytes: system.total_memory(),
    current_rss_bytes,
    peak_rss_bytes: None,
    rustc_version: command_output("rustc", &["--version"]),
    git_commit: command_output("git", &["rev-parse", "HEAD"]),
    filesystem,
  }
}

#[tokio::main]
async fn main() {
  if let Err(error) = run().await {
    eprintln!("archive benchmark failed: {error}");
    std::process::exit(1);
  }
}

async fn run() -> Result<()> {
  let config = Config::parse()?;
  if config.output.exists() {
    return Err(
      format!(
        "refusing existing output path {}; choose a new directory",
        config.output.display()
      )
      .into(),
    );
  }
  fs::create_dir(&config.output)?;

  let mut benchmark = Benchmark::create(config.clone()).await?;
  benchmark.seed().await?;
  benchmark.finalize().await?;
  benchmark.verify_and_query().await?;
  if !benchmark.correctness.all_pass() {
    return Err("one or more correctness checks failed".into());
  }
  let baseline = benchmark.baseline_footprint().await?;
  let before = benchmark.candidate_footprint().await?;
  let vacuum_started = std::time::Instant::now();
  benchmark.vacuum_candidate().await?;
  let vacuum_wall_ms = vacuum_started.elapsed().as_secs_f64() * 1_000.0;
  let after = benchmark.candidate_footprint().await?;
  let sqlite = benchmark.sqlite_report().await?;

  let report = Report {
    format: "hardviz-archive-g1-v1",
    scope: "synthetic two-family experiment: PROCESS_STATS and AMBIENT_ARCHIVE",
    workload: WorkloadReport {
      source: "deterministic synthetic production-shaped rows",
      represented_minutes: config.minutes,
      sampled_minutes: config.minutes.div_ceil(config.duty_cycle),
      sampling_mode: if config.duty_cycle == 1 {
        "continuous"
      } else {
        "duty-cycled"
      },
      duty_cycle: config.duty_cycle,
      processes_per_sampled_minute: config.processes_per_minute,
      numeric_shape: "producer-range rows plus separately labeled exact i64/binary64 sentinels",
      process_rows: benchmark.process_rows,
      ambient_rows: benchmark.ambient_rows,
      seed: config.seed,
    },
    format_candidate: FormatReport {
      layout: database::layout_name(config.layout),
      compression: database::compression_name(config.compression),
      chunk_minutes: config.chunk_minutes,
      chunk_rows: config.chunk_rows,
      chunks: benchmark.finalized.chunks,
      encoded_bytes: benchmark.finalized.encoded_bytes,
      decoded_value_bytes: benchmark.finalized.decoded_value_bytes,
    },
    environment: environment_report(&config.output),
    sqlite,
    footprints: FootprintReport {
      before_reclamation_reduction_percent: reduction(&baseline, &before),
      after_vacuum_reduction_percent: reduction(&baseline, &after),
      relational_baseline: baseline,
      chunked_before_reclamation: before,
      chunked_after_vacuum: after,
      vacuum_wall_ms,
    },
    timings_ms: TimingReport {
      fixture_seed_batch: Timing::from(&benchmark.times.fixture_seed_batch),
      production_shaped_minute_append: Timing::from(
        &benchmark.times.production_minute_append,
      ),
      source_scan: Timing::from(&benchmark.times.source_scan),
      encode: Timing::from(&benchmark.times.encode),
      decode: Timing::from(&benchmark.times.decode),
      chunk_insert_and_tail_delete: Timing::from(&benchmark.times.chunk_insert),
      finalize_total: Timing::from(&benchmark.times.finalize),
      process_query_baseline: Timing::from(&benchmark.times.process_baseline),
      process_query_chunked: Timing::from(&benchmark.times.process_candidate),
      ambient_raw_query_baseline: Timing::from(&benchmark.times.ambient_baseline),
      ambient_raw_query_chunked: Timing::from(&benchmark.times.ambient_candidate),
    },
    correctness: benchmark.correctness.clone(),
    limitations: [
      "Synthetic data is not real-world workload evidence.",
      "Footprints cover two raw families, not the full application database or its 30 percent gate.",
      "This experiment does not implement migration, pagination, retention, recovery selection, or the 30-minute background CPU gate.",
      "Ambient is a representative raw epoch-millisecond inclusive range; production Cooling also has half-open pairing contracts.",
      "Candidate query times include catalog reads, decoding, filtering, and aggregation.",
      "Peak RSS and temporary SQLite bytes are not instrumented in this initial harness.",
    ],
  };
  let json = serde_json::to_string_pretty(&report)?;
  fs::write(config.output.join("report.json"), format!("{json}\n"))?;
  println!("{json}");
  Ok(())
}
