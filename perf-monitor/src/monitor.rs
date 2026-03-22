use std::io::Read as _;
use std::path::Path;
use std::process::{Child, Command, Stdio};
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

/// RAII guard that ensures the child process is terminated on drop.
struct ProcessGuard(Child);

impl Drop for ProcessGuard {
  fn drop(&mut self) {
    eprintln!("Terminating process...");
    let _ = self.0.kill();
    let _ = self.0.wait();
  }
}

pub fn run_monitor(
  binary_path: &Path,
  timing: &Timing,
) -> Result<MonitorResult, Box<dyn std::error::Error>> {
  let mut guard = ProcessGuard(launch_process(binary_path)?);
  let pid = Pid::from_u32(guard.0.id());
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
    if guard.0.try_wait()?.is_some() {
      return Err(format_exit_error(&mut guard, "warmup"));
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

    if guard.0.try_wait()?.is_some() {
      return Err(format_exit_error(&mut guard, "measurement"));
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

  // ProcessGuard::drop will terminate the process
  drop(guard);

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
  let child = Command::new(binary_path)
    .stderr(Stdio::piped())
    .spawn()?;
  eprintln!("Launched process with PID: {}", child.id());
  Ok(child)
}

fn format_exit_error(
  guard: &mut ProcessGuard,
  phase: &str,
) -> Box<dyn std::error::Error> {
  let status = guard.0.try_wait().ok().flatten();
  let stderr = guard
    .0
    .stderr
    .take()
    .and_then(|mut s| {
      let mut buf = String::new();
      s.read_to_string(&mut buf).ok()?;
      if buf.is_empty() { None } else { Some(buf) }
    });

  let mut msg = format!("Process exited during {phase}");
  if let Some(st) = status {
    msg.push_str(&format!(" (exit status: {st})"));
  }
  if let Some(err) = stderr {
    msg.push_str(&format!("\n--- process stderr ---\n{err}"));
  }
  msg.into()
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn percentile_f32_empty_returns_zero() {
    assert_eq!(percentile_f32(&[], 95.0), 0.0);
  }

  #[test]
  fn percentile_f64_empty_returns_zero() {
    assert_eq!(percentile_f64(&[], 95.0), 0.0);
  }

  #[test]
  fn percentile_f32_single_value() {
    assert_eq!(percentile_f32(&[42.0], 95.0), 42.0);
  }

  #[test]
  fn percentile_f32_known_values_odd_count() {
    // Sorted: [1, 2, 3, 4, 5]
    // P95 index: 0.95 * 4 = 3.8 → round to 4 → value = 5
    let values = vec![3.0, 1.0, 5.0, 2.0, 4.0];
    assert_eq!(percentile_f32(&values, 95.0), 5.0);
  }

  #[test]
  fn percentile_f32_known_values_even_count() {
    // Sorted: [10, 20, 30, 40]
    // P95 index: 0.95 * 3 = 2.85 → round to 3 → value = 40
    let values = vec![30.0, 10.0, 40.0, 20.0];
    assert_eq!(percentile_f32(&values, 95.0), 40.0);
  }

  #[test]
  fn percentile_f32_p50_median() {
    // Sorted: [1, 2, 3, 4, 5]
    // P50 index: 0.50 * 4 = 2.0 → value = 3
    let values = vec![5.0, 3.0, 1.0, 4.0, 2.0];
    assert_eq!(percentile_f32(&values, 50.0), 3.0);
  }

  #[test]
  fn percentile_f64_matches_f32_logic() {
    let values = vec![3.0, 1.0, 5.0, 2.0, 4.0];
    assert_eq!(percentile_f64(&values, 95.0), 5.0);
  }

  #[test]
  fn compute_result_basic_statistics() {
    let samples = vec![
      ProcessMetrics { cpu_usage: 10.0, memory_rss_mb: 100.0 },
      ProcessMetrics { cpu_usage: 20.0, memory_rss_mb: 200.0 },
      ProcessMetrics { cpu_usage: 30.0, memory_rss_mb: 150.0 },
    ];

    let result = compute_result(samples, 3, 5);

    assert_eq!(result.duration_seconds, 3);
    assert_eq!(result.warmup_seconds, 5);

    // avg = (10 + 20 + 30) / 3 = 20
    assert!((result.avg_cpu - 20.0).abs() < 0.01);
    // max = 30
    assert!((result.max_cpu - 30.0).abs() < 0.01);
    // avg memory = (100 + 200 + 150) / 3 = 150
    assert!((result.avg_memory_mb - 150.0).abs() < 0.01);
    // growth = last - first = 150 - 100 = 50
    assert!((result.memory_growth_mb - 50.0).abs() < 0.01);
  }
}
