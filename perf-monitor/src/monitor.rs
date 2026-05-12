use std::collections::{HashMap, HashSet};
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

  // Initial refresh to establish CPU baseline. We refresh ALL processes
  // because the target may spawn descendants at any time, and sysinfo
  // computes CPU usage as a delta from the previous refresh of that PID.
  system.refresh_processes(ProcessesToUpdate::All, true);

  eprintln!("Warming up for {} seconds...", timing.warmup_seconds);

  // Warmup phase: wait for app to stabilize
  for _ in 0..timing.warmup_seconds {
    thread::sleep(Duration::from_secs(1));
    if guard.0.try_wait()?.is_some() {
      return Err(format_exit_error(&mut guard, "warmup"));
    }
    system.refresh_processes(ProcessesToUpdate::All, true);
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

    system.refresh_processes(ProcessesToUpdate::All, true);

    match aggregate_descendants(&system, pid, num_cpus) {
      Some(metrics) => samples.push(metrics),
      None => return Err("Process disappeared during measurement".into()),
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

/// Sums CPU and RSS across the root process and all of its descendants.
/// Returns `None` if the root PID is no longer present in `system`.
fn aggregate_descendants(
  system: &System,
  root_pid: Pid,
  num_cpus: f32,
) -> Option<ProcessMetrics> {
  system.process(root_pid)?;

  let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();
  let mut metrics_map: HashMap<Pid, (f32, u64)> = HashMap::new();
  for (pid, process) in system.processes() {
    metrics_map.insert(*pid, (process.cpu_usage(), process.memory()));
    if let Some(parent_pid) = process.parent() {
      children_map.entry(parent_pid).or_default().push(*pid);
    }
  }

  let (total_cpu, total_memory_bytes) =
    sum_subtree(root_pid, &children_map, &metrics_map);

  Some(ProcessMetrics {
    cpu_usage: total_cpu / num_cpus,
    memory_rss_mb: total_memory_bytes as f64 / (1024.0 * 1024.0),
  })
}

/// Walks the subtree rooted at `root`, summing per-process CPU and memory.
/// Guards against cycles in case the snapshot contains an inconsistent
/// parent chain (e.g. PID reuse during sampling).
fn sum_subtree(
  root: Pid,
  children: &HashMap<Pid, Vec<Pid>>,
  metrics: &HashMap<Pid, (f32, u64)>,
) -> (f32, u64) {
  let mut visited: HashSet<Pid> = HashSet::new();
  let mut stack: Vec<Pid> = vec![root];
  let mut total_cpu = 0.0_f32;
  let mut total_memory_bytes: u64 = 0;

  while let Some(pid) = stack.pop() {
    if !visited.insert(pid) {
      continue;
    }
    if let Some(&(cpu, memory)) = metrics.get(&pid) {
      total_cpu += cpu;
      total_memory_bytes += memory;
    }
    if let Some(child_pids) = children.get(&pid) {
      stack.extend(child_pids);
    }
  }

  (total_cpu, total_memory_bytes)
}

fn launch_process(binary_path: &Path) -> Result<Child, Box<dyn std::error::Error>> {
  let child = Command::new(binary_path).stderr(Stdio::piped()).spawn()?;
  eprintln!("Launched process with PID: {}", child.id());
  Ok(child)
}

fn format_exit_error(
  guard: &mut ProcessGuard,
  phase: &str,
) -> Box<dyn std::error::Error> {
  let status = guard.0.try_wait().ok().flatten();
  let stderr = guard.0.stderr.take().and_then(|mut s| {
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

  fn pid(n: usize) -> Pid {
    Pid::from(n)
  }

  #[test]
  fn sum_subtree_root_only() {
    let mut metrics = HashMap::new();
    metrics.insert(pid(1), (10.0, 1024));
    let children = HashMap::new();

    let (cpu, mem) = sum_subtree(pid(1), &children, &metrics);
    assert!((cpu - 10.0).abs() < 0.01);
    assert_eq!(mem, 1024);
  }

  #[test]
  fn sum_subtree_aggregates_descendants() {
    // Tree:  1
    //       / \
    //      2   3
    //      |
    //      4
    let mut metrics = HashMap::new();
    metrics.insert(pid(1), (10.0, 100));
    metrics.insert(pid(2), (20.0, 200));
    metrics.insert(pid(3), (30.0, 300));
    metrics.insert(pid(4), (40.0, 400));

    let mut children = HashMap::new();
    children.insert(pid(1), vec![pid(2), pid(3)]);
    children.insert(pid(2), vec![pid(4)]);

    let (cpu, mem) = sum_subtree(pid(1), &children, &metrics);
    assert!((cpu - 100.0).abs() < 0.01);
    assert_eq!(mem, 1000);
  }

  #[test]
  fn sum_subtree_ignores_siblings_outside_subtree() {
    // 1 -> 2; 99 is a sibling unrelated to 1.
    let mut metrics = HashMap::new();
    metrics.insert(pid(1), (10.0, 100));
    metrics.insert(pid(2), (20.0, 200));
    metrics.insert(pid(99), (999.0, 9999));

    let mut children = HashMap::new();
    children.insert(pid(1), vec![pid(2)]);

    let (cpu, mem) = sum_subtree(pid(1), &children, &metrics);
    assert!((cpu - 30.0).abs() < 0.01);
    assert_eq!(mem, 300);
  }

  #[test]
  fn sum_subtree_handles_cycle_without_double_counting() {
    // Pathological: 1 -> 2 -> 1 (cycle).
    let mut metrics = HashMap::new();
    metrics.insert(pid(1), (10.0, 100));
    metrics.insert(pid(2), (20.0, 200));

    let mut children = HashMap::new();
    children.insert(pid(1), vec![pid(2)]);
    children.insert(pid(2), vec![pid(1)]);

    let (cpu, mem) = sum_subtree(pid(1), &children, &metrics);
    assert!((cpu - 30.0).abs() < 0.01);
    assert_eq!(mem, 300);
  }

  #[test]
  fn sum_subtree_missing_root_returns_zero() {
    let metrics = HashMap::new();
    let children = HashMap::new();
    let (cpu, mem) = sum_subtree(pid(1), &children, &metrics);
    assert_eq!(cpu, 0.0);
    assert_eq!(mem, 0);
  }

  #[test]
  fn compute_result_basic_statistics() {
    let samples = vec![
      ProcessMetrics {
        cpu_usage: 10.0,
        memory_rss_mb: 100.0,
      },
      ProcessMetrics {
        cpu_usage: 20.0,
        memory_rss_mb: 200.0,
      },
      ProcessMetrics {
        cpu_usage: 30.0,
        memory_rss_mb: 150.0,
      },
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
