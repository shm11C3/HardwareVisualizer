use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::config::Timing;

#[derive(Debug)]
pub struct ProcessMetrics {
  pub cpu_usage: f32,
  pub memory_rss_mb: f64,
}

#[derive(Debug)]
pub struct MonitorResult {
  pub samples: Vec<ProcessMetrics>,
  pub avg_cpu: f32,
  pub max_cpu: f32,
  pub p95_cpu: f32,
  pub avg_memory_mb: f64,
  pub max_memory_mb: f64,
  pub p95_memory_mb: f64,
  pub memory_growth_mb: f64,
  pub duration_seconds: u64,
  pub warmup_seconds: u64,
}

pub fn run_monitor(
  binary_path: &Path,
  timing: &Timing,
) -> Result<MonitorResult, Box<dyn std::error::Error>> {
  let mut child = launch_process(binary_path)?;
  let pid = Pid::from_u32(child.id());
  let num_cpus = thread::available_parallelism()
    .map(|n| n.get() as f32)
    .unwrap_or(1.0);

  let mut system = System::new();

  // Initial refresh to establish CPU baseline
  system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

  eprintln!("Warming up for {} seconds...", timing.warmup_seconds);

  // Warmup phase: wait for app to stabilize
  for _ in 0..timing.warmup_seconds {
    thread::sleep(Duration::from_secs(1));
    if child.try_wait()?.is_some() {
      return Err("Process exited during warmup".into());
    }
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
  }

  eprintln!(
    "Measuring for {} seconds (interval: {}ms)...",
    timing.measurement_seconds, timing.sample_interval_ms
  );

  // Measurement phase
  let interval = Duration::from_millis(timing.sample_interval_ms);
  let total_samples = (timing.measurement_seconds * 1000) / timing.sample_interval_ms;
  let mut samples = Vec::with_capacity(total_samples as usize);

  for _ in 0..total_samples {
    thread::sleep(interval);

    if child.try_wait()?.is_some() {
      return Err("Process exited during measurement".into());
    }

    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

    if let Some(process) = system.process(pid) {
      let cpu_normalized = process.cpu_usage() / num_cpus;
      let memory_mb = process.memory() as f64 / (1024.0 * 1024.0);

      samples.push(ProcessMetrics {
        cpu_usage: cpu_normalized,
        memory_rss_mb: memory_mb,
      });
    } else {
      return Err("Process disappeared during measurement".into());
    }
  }

  // Terminate the process
  terminate_process(&mut child);

  if samples.is_empty() {
    return Err("No samples collected".into());
  }

  Ok(compute_result(
    samples,
    timing.measurement_seconds,
    timing.warmup_seconds,
  ))
}

fn launch_process(binary_path: &Path) -> Result<Child, Box<dyn std::error::Error>> {
  let child = Command::new(binary_path).spawn()?;
  eprintln!("Launched process with PID: {}", child.id());
  Ok(child)
}

fn terminate_process(child: &mut Child) {
  eprintln!("Terminating process...");
  let _ = child.kill();
  let _ = child.wait();
}

fn compute_result(
  samples: Vec<ProcessMetrics>,
  duration_seconds: u64,
  warmup_seconds: u64,
) -> MonitorResult {
  let n = samples.len() as f32;

  let avg_cpu = samples.iter().map(|s| s.cpu_usage).sum::<f32>() / n;
  let max_cpu = samples
    .iter()
    .map(|s| s.cpu_usage)
    .fold(f32::NEG_INFINITY, f32::max);
  let p95_cpu = percentile_f32(
    &samples.iter().map(|s| s.cpu_usage).collect::<Vec<_>>(),
    95.0,
  );

  let n_f64 = samples.len() as f64;
  let avg_memory_mb = samples.iter().map(|s| s.memory_rss_mb).sum::<f64>() / n_f64;
  let max_memory_mb = samples
    .iter()
    .map(|s| s.memory_rss_mb)
    .fold(f64::NEG_INFINITY, f64::max);
  let p95_memory_mb = percentile_f64(
    &samples.iter().map(|s| s.memory_rss_mb).collect::<Vec<_>>(),
    95.0,
  );

  let first_memory = samples.first().map(|s| s.memory_rss_mb).unwrap_or(0.0);
  let last_memory = samples.last().map(|s| s.memory_rss_mb).unwrap_or(0.0);
  let memory_growth_mb = last_memory - first_memory;

  MonitorResult {
    samples,
    avg_cpu,
    max_cpu,
    p95_cpu,
    avg_memory_mb,
    max_memory_mb,
    p95_memory_mb,
    memory_growth_mb,
    duration_seconds,
    warmup_seconds,
  }
}

fn percentile_f32(values: &[f32], pct: f32) -> f32 {
  if values.is_empty() {
    return 0.0;
  }
  let mut sorted: Vec<f32> = values.to_vec();
  sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
  let idx = ((pct / 100.0) * (sorted.len() - 1) as f32).round() as usize;
  sorted[idx.min(sorted.len() - 1)]
}

fn percentile_f64(values: &[f64], pct: f64) -> f64 {
  if values.is_empty() {
    return 0.0;
  }
  let mut sorted: Vec<f64> = values.to_vec();
  sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
  let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
  sorted[idx.min(sorted.len() - 1)]
}
