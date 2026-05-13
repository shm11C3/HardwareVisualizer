use serde::Serialize;

use crate::config::Thresholds;
use crate::monitor::{MonitorResult, ProcessStats};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
  Text,
  Json,
}

struct CheckResult {
  cpu_avg_ok: bool,
  cpu_p95_ok: bool,
  mem_avg_ok: bool,
  mem_p95_ok: bool,
  mem_growth_ok: bool,
  all_passed: bool,
  failure_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonReport {
  duration_seconds: u64,
  warmup_seconds: u64,
  sample_count: usize,
  cpu: CpuReport,
  memory: MemoryReport,
  per_process: Vec<PerProcessReport>,
  failure_reasons: Vec<String>,
  passed: bool,
}

#[derive(Debug, Serialize)]
struct CpuReport {
  avg_percent: f32,
  max_percent: f32,
  p95_percent: f32,
  threshold_avg: f32,
  threshold_p95: f32,
  avg_passed: bool,
  p95_passed: bool,
}

#[derive(Debug, Serialize)]
struct MemoryReport {
  avg_mb: f64,
  max_mb: f64,
  p95_mb: f64,
  growth_mb: f64,
  threshold_avg_mb: f64,
  threshold_p95_mb: f64,
  threshold_growth_mb: f64,
  avg_passed: bool,
  p95_passed: bool,
  growth_passed: bool,
}

#[derive(Debug, Serialize)]
struct PerProcessReport {
  pid: u32,
  name: String,
  is_root: bool,
  sample_count: usize,
  cpu_avg_percent: f32,
  cpu_max_percent: f32,
  cpu_p95_percent: f32,
  memory_avg_mb: f64,
  memory_max_mb: f64,
  memory_p95_mb: f64,
}

/// Formats and prints the report. Returns `true` if all checks passed.
pub fn format_report(
  result: &MonitorResult,
  thresholds: &Thresholds,
  format: OutputFormat,
) -> bool {
  let check = CheckResult::new(result, thresholds);

  match format {
    OutputFormat::Text => print_text_report(result, thresholds, &check),
    OutputFormat::Json => print_json_report(result, thresholds, &check),
  }

  check.all_passed
}

impl CheckResult {
  fn new(result: &MonitorResult, thresholds: &Thresholds) -> Self {
    let cpu_avg_ok = result.avg_cpu <= thresholds.max_avg_cpu_percent;
    let cpu_p95_ok = result.p95_cpu <= thresholds.max_p95_cpu_percent;
    let mem_avg_ok = result.avg_memory_mb <= thresholds.max_avg_memory_mb;
    let mem_p95_ok = result.p95_memory_mb <= thresholds.max_p95_memory_mb;
    let mem_growth_ok = result.memory_growth_mb <= thresholds.max_memory_growth_mb;

    let mut failure_reasons = Vec::new();
    if !cpu_avg_ok {
      failure_reasons.push(format!(
        "CPU average {:.1}% exceeded threshold {:.1}%",
        result.avg_cpu, thresholds.max_avg_cpu_percent
      ));
    }
    if !cpu_p95_ok {
      failure_reasons.push(format!(
        "CPU P95 {:.1}% exceeded threshold {:.1}%",
        result.p95_cpu, thresholds.max_p95_cpu_percent
      ));
    }
    if !mem_avg_ok {
      failure_reasons.push(format!(
        "Memory average {:.1} MB exceeded threshold {:.1} MB",
        result.avg_memory_mb, thresholds.max_avg_memory_mb
      ));
    }
    if !mem_p95_ok {
      failure_reasons.push(format!(
        "Memory P95 {:.1} MB exceeded threshold {:.1} MB",
        result.p95_memory_mb, thresholds.max_p95_memory_mb
      ));
    }
    if !mem_growth_ok {
      failure_reasons.push(format!(
        "Memory growth {:.1} MB exceeded threshold {:.1} MB",
        result.memory_growth_mb, thresholds.max_memory_growth_mb
      ));
    }

    Self {
      cpu_avg_ok,
      cpu_p95_ok,
      mem_avg_ok,
      mem_p95_ok,
      mem_growth_ok,
      all_passed: cpu_avg_ok && cpu_p95_ok && mem_avg_ok && mem_p95_ok && mem_growth_ok,
      failure_reasons,
    }
  }
}

fn status_label(passed: bool) -> &'static str {
  if passed { "PASS" } else { "FAIL" }
}

fn print_text_report(
  result: &MonitorResult,
  thresholds: &Thresholds,
  check: &CheckResult,
) {
  println!();
  println!("=== Performance Test Results ===");
  println!(
    "Duration: {}s (after {}s warmup)",
    result.duration_seconds, result.warmup_seconds
  );
  println!("Samples: {}", result.samples.len());
  println!("Processes observed: {}", result.per_process.len());
  println!();
  println!("Aggregated (process tree):");
  println!("  CPU Usage:");
  println!(
    "    Average: {:>6.1}%  (threshold: {:>5.1}%)  {}",
    result.avg_cpu,
    thresholds.max_avg_cpu_percent,
    status_label(check.cpu_avg_ok)
  );
  println!(
    "    P95:     {:>6.1}%  (threshold: {:>5.1}%)  {}",
    result.p95_cpu,
    thresholds.max_p95_cpu_percent,
    status_label(check.cpu_p95_ok)
  );
  println!("  Memory (RSS):");
  println!(
    "    Average: {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.avg_memory_mb,
    thresholds.max_avg_memory_mb,
    status_label(check.mem_avg_ok)
  );
  println!(
    "    P95:     {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.p95_memory_mb,
    thresholds.max_p95_memory_mb,
    status_label(check.mem_p95_ok)
  );
  println!(
    "    Growth:  {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.memory_growth_mb,
    thresholds.max_memory_growth_mb,
    status_label(check.mem_growth_ok)
  );

  print_per_process_text(&result.per_process, result.samples.len());

  if !check.failure_reasons.is_empty() {
    println!();
    println!("Failure Reasons:");
    for reason in &check.failure_reasons {
      println!("  - {reason}");
    }
  }

  println!();
  println!("Result: {}", if check.all_passed { "PASS" } else { "FAIL" });
}

fn print_per_process_text(per_process: &[ProcessStats], total_samples: usize) {
  if per_process.is_empty() {
    return;
  }
  println!();
  println!("Per-Process Breakdown:");
  for stats in per_process {
    let role = if stats.is_root { " (root)" } else { "" };
    println!(
      "  PID {:>6} {}{}  [observed {}/{} samples]",
      stats.pid, stats.name, role, stats.sample_count, total_samples
    );
    println!(
      "    CPU  avg/max/P95: {:>6.1}% / {:>6.1}% / {:>6.1}%",
      stats.avg_cpu, stats.max_cpu, stats.p95_cpu
    );
    println!(
      "    Mem  avg/max/P95: {:>7.1} MB / {:>7.1} MB / {:>7.1} MB",
      stats.avg_memory_mb, stats.max_memory_mb, stats.p95_memory_mb
    );
  }
}

fn print_json_report(
  result: &MonitorResult,
  thresholds: &Thresholds,
  check: &CheckResult,
) {
  let report = JsonReport {
    duration_seconds: result.duration_seconds,
    warmup_seconds: result.warmup_seconds,
    sample_count: result.samples.len(),
    cpu: CpuReport {
      avg_percent: result.avg_cpu,
      max_percent: result.max_cpu,
      p95_percent: result.p95_cpu,
      threshold_avg: thresholds.max_avg_cpu_percent,
      threshold_p95: thresholds.max_p95_cpu_percent,
      avg_passed: check.cpu_avg_ok,
      p95_passed: check.cpu_p95_ok,
    },
    memory: MemoryReport {
      avg_mb: result.avg_memory_mb,
      max_mb: result.max_memory_mb,
      p95_mb: result.p95_memory_mb,
      growth_mb: result.memory_growth_mb,
      threshold_avg_mb: thresholds.max_avg_memory_mb,
      threshold_p95_mb: thresholds.max_p95_memory_mb,
      threshold_growth_mb: thresholds.max_memory_growth_mb,
      avg_passed: check.mem_avg_ok,
      p95_passed: check.mem_p95_ok,
      growth_passed: check.mem_growth_ok,
    },
    per_process: result
      .per_process
      .iter()
      .map(|s| PerProcessReport {
        pid: s.pid,
        name: s.name.clone(),
        is_root: s.is_root,
        sample_count: s.sample_count,
        cpu_avg_percent: s.avg_cpu,
        cpu_max_percent: s.max_cpu,
        cpu_p95_percent: s.p95_cpu,
        memory_avg_mb: s.avg_memory_mb,
        memory_max_mb: s.max_memory_mb,
        memory_p95_mb: s.p95_memory_mb,
      })
      .collect(),
    failure_reasons: check.failure_reasons.clone(),
    passed: check.all_passed,
  };

  match serde_json::to_string_pretty(&report) {
    Ok(json) => println!("{json}"),
    Err(e) => {
      eprintln!("Error: failed to serialize report: {e}");
      std::process::exit(1);
    }
  }
}
