use std::cmp::Ordering;
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

/// Aggregated statistics for a single PID observed during the measurement window.
#[derive(Debug, Clone)]
pub struct ProcessStats {
  pub pid: u32,
  pub name: String,
  pub is_root: bool,
  pub sample_count: usize,
  pub avg_cpu: f32,
  pub max_cpu: f32,
  pub p95_cpu: f32,
  pub avg_memory_mb: f64,
  pub max_memory_mb: f64,
  pub p95_memory_mb: f64,
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
  pub per_process: Vec<ProcessStats>,
}

/// One process's contribution to a single sample tick.
#[derive(Debug, Clone)]
struct ProcessSnapshot {
  pid: Pid,
  name: String,
  cpu_usage_raw: f32,
  memory_bytes: u64,
}

/// Accumulator that gathers per-PID samples across the whole measurement window.
#[derive(Debug, Default)]
struct ProcessTrack {
  name: String,
  is_root: bool,
  cpu_samples: Vec<f32>,
  memory_samples_mb: Vec<f64>,
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
  let mut samples: Vec<ProcessMetrics> = Vec::with_capacity(total_samples as usize);
  let mut per_pid_tracks: HashMap<Pid, ProcessTrack> = HashMap::new();

  for _ in 0..total_samples {
    thread::sleep(interval);

    if guard.0.try_wait()?.is_some() {
      return Err(format_exit_error(&mut guard, "measurement"));
    }

    system.refresh_processes(ProcessesToUpdate::All, true);

    let snapshots = match capture_subtree(&system, pid) {
      Some(s) => s,
      None => return Err("Process disappeared during measurement".into()),
    };

    let mut tick_cpu_raw = 0.0_f32;
    let mut tick_memory_bytes: u64 = 0;
    for snap in &snapshots {
      tick_cpu_raw += snap.cpu_usage_raw;
      tick_memory_bytes += snap.memory_bytes;

      let track = per_pid_tracks.entry(snap.pid).or_insert_with(|| ProcessTrack {
        name: snap.name.clone(),
        is_root: snap.pid == pid,
        cpu_samples: Vec::new(),
        memory_samples_mb: Vec::new(),
      });
      track.cpu_samples.push(snap.cpu_usage_raw / num_cpus);
      track
        .memory_samples_mb
        .push(snap.memory_bytes as f64 / (1024.0 * 1024.0));
    }

    samples.push(ProcessMetrics {
      cpu_usage: tick_cpu_raw / num_cpus,
      memory_rss_mb: tick_memory_bytes as f64 / (1024.0 * 1024.0),
    });
  }

  // ProcessGuard::drop will terminate the process
  drop(guard);

  if samples.is_empty() {
    return Err("No samples collected".into());
  }

  Ok(compute_result(
    samples,
    per_pid_tracks,
    timing.measurement_seconds,
    timing.warmup_seconds,
  ))
}

/// Captures per-process metrics for the root PID and all of its descendants.
/// Returns `None` if the root PID is no longer present in `system`.
fn capture_subtree(system: &System, root_pid: Pid) -> Option<Vec<ProcessSnapshot>> {
  system.process(root_pid)?;

  let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();
  let mut snapshot_map: HashMap<Pid, ProcessSnapshot> = HashMap::new();
  for (pid, process) in system.processes() {
    snapshot_map.insert(
      *pid,
      ProcessSnapshot {
        pid: *pid,
        name: process.name().to_string_lossy().into_owned(),
        cpu_usage_raw: process.cpu_usage(),
        memory_bytes: process.memory(),
      },
    );
    if let Some(parent_pid) = process.parent() {
      children_map.entry(parent_pid).or_default().push(*pid);
    }
  }

  Some(walk_subtree(root_pid, &children_map, &snapshot_map))
}

/// Walks the subtree rooted at `root`, returning per-process snapshots.
/// Guards against cycles caused by an inconsistent parent chain.
fn walk_subtree(
  root: Pid,
  children: &HashMap<Pid, Vec<Pid>>,
  snapshots: &HashMap<Pid, ProcessSnapshot>,
) -> Vec<ProcessSnapshot> {
  let mut visited: HashSet<Pid> = HashSet::new();
  let mut stack: Vec<Pid> = vec![root];
  let mut out: Vec<ProcessSnapshot> = Vec::new();

  while let Some(pid) = stack.pop() {
    if !visited.insert(pid) {
      continue;
    }
    if let Some(snap) = snapshots.get(&pid) {
      out.push(snap.clone());
    }
    if let Some(child_pids) = children.get(&pid) {
      stack.extend(child_pids);
    }
  }

  out
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
  per_pid_tracks: HashMap<Pid, ProcessTrack>,
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

  let per_process = compute_per_process_stats(per_pid_tracks);

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
    per_process,
  }
}

fn compute_per_process_stats(
  tracks: HashMap<Pid, ProcessTrack>,
) -> Vec<ProcessStats> {
  let mut stats: Vec<ProcessStats> = tracks
    .into_iter()
    .filter_map(|(pid, track)| {
      if track.cpu_samples.is_empty() {
        return None;
      }
      let n = track.cpu_samples.len() as f32;
      let avg_cpu = track.cpu_samples.iter().sum::<f32>() / n;
      let max_cpu = track
        .cpu_samples
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
      let p95_cpu = percentile_f32(&track.cpu_samples, 95.0);

      let n_f64 = track.memory_samples_mb.len() as f64;
      let avg_memory_mb = track.memory_samples_mb.iter().sum::<f64>() / n_f64;
      let max_memory_mb = track
        .memory_samples_mb
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
      let p95_memory_mb = percentile_f64(&track.memory_samples_mb, 95.0);

      Some(ProcessStats {
        pid: pid.as_u32(),
        name: track.name,
        is_root: track.is_root,
        sample_count: track.cpu_samples.len(),
        avg_cpu,
        max_cpu,
        p95_cpu,
        avg_memory_mb,
        max_memory_mb,
        p95_memory_mb,
      })
    })
    .collect();

  // Root first, then descendants by avg CPU descending (most interesting first).
  stats.sort_by(|a, b| match (a.is_root, b.is_root) {
    (true, false) => Ordering::Less,
    (false, true) => Ordering::Greater,
    _ => b
      .avg_cpu
      .partial_cmp(&a.avg_cpu)
      .unwrap_or(Ordering::Equal),
  });

  stats
}

fn percentile_f32(values: &[f32], pct: f32) -> f32 {
  if values.is_empty() {
    return 0.0;
  }
  let mut sorted: Vec<f32> = values.to_vec();
  sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
  let idx = ((pct / 100.0) * (sorted.len() - 1) as f32).round() as usize;
  sorted[idx.min(sorted.len() - 1)]
}

fn percentile_f64(values: &[f64], pct: f64) -> f64 {
  if values.is_empty() {
    return 0.0;
  }
  let mut sorted: Vec<f64> = values.to_vec();
  sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
  let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
  sorted[idx.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
  use super::*;

  fn pid(n: usize) -> Pid {
    Pid::from(n)
  }

  fn snap(pid_n: usize, name: &str, cpu: f32, mem: u64) -> ProcessSnapshot {
    ProcessSnapshot {
      pid: pid(pid_n),
      name: name.to_string(),
      cpu_usage_raw: cpu,
      memory_bytes: mem,
    }
  }

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
    let values = vec![3.0, 1.0, 5.0, 2.0, 4.0];
    assert_eq!(percentile_f32(&values, 95.0), 5.0);
  }

  #[test]
  fn percentile_f32_known_values_even_count() {
    let values = vec![30.0, 10.0, 40.0, 20.0];
    assert_eq!(percentile_f32(&values, 95.0), 40.0);
  }

  #[test]
  fn percentile_f32_p50_median() {
    let values = vec![5.0, 3.0, 1.0, 4.0, 2.0];
    assert_eq!(percentile_f32(&values, 50.0), 3.0);
  }

  #[test]
  fn percentile_f64_matches_f32_logic() {
    let values = vec![3.0, 1.0, 5.0, 2.0, 4.0];
    assert_eq!(percentile_f64(&values, 95.0), 5.0);
  }

  #[test]
  fn walk_subtree_root_only() {
    let mut snapshots = HashMap::new();
    snapshots.insert(pid(1), snap(1, "root", 10.0, 1024));
    let children = HashMap::new();

    let result = walk_subtree(pid(1), &children, &snapshots);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].pid, pid(1));
  }

  #[test]
  fn walk_subtree_visits_all_descendants() {
    // Tree:  1
    //       / \
    //      2   3
    //      |
    //      4
    let mut snapshots = HashMap::new();
    snapshots.insert(pid(1), snap(1, "root", 10.0, 100));
    snapshots.insert(pid(2), snap(2, "child-a", 20.0, 200));
    snapshots.insert(pid(3), snap(3, "child-b", 30.0, 300));
    snapshots.insert(pid(4), snap(4, "grandchild", 40.0, 400));

    let mut children = HashMap::new();
    children.insert(pid(1), vec![pid(2), pid(3)]);
    children.insert(pid(2), vec![pid(4)]);

    let result = walk_subtree(pid(1), &children, &snapshots);
    let total_cpu: f32 = result.iter().map(|s| s.cpu_usage_raw).sum();
    let total_mem: u64 = result.iter().map(|s| s.memory_bytes).sum();
    assert_eq!(result.len(), 4);
    assert!((total_cpu - 100.0).abs() < 0.01);
    assert_eq!(total_mem, 1000);
  }

  #[test]
  fn walk_subtree_ignores_siblings_outside_subtree() {
    let mut snapshots = HashMap::new();
    snapshots.insert(pid(1), snap(1, "root", 10.0, 100));
    snapshots.insert(pid(2), snap(2, "child", 20.0, 200));
    snapshots.insert(pid(99), snap(99, "unrelated", 999.0, 9999));

    let mut children = HashMap::new();
    children.insert(pid(1), vec![pid(2)]);

    let result = walk_subtree(pid(1), &children, &snapshots);
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|s| s.pid != pid(99)));
  }

  #[test]
  fn walk_subtree_handles_cycle_without_double_counting() {
    // Pathological: 1 -> 2 -> 1 (cycle).
    let mut snapshots = HashMap::new();
    snapshots.insert(pid(1), snap(1, "root", 10.0, 100));
    snapshots.insert(pid(2), snap(2, "child", 20.0, 200));

    let mut children = HashMap::new();
    children.insert(pid(1), vec![pid(2)]);
    children.insert(pid(2), vec![pid(1)]);

    let result = walk_subtree(pid(1), &children, &snapshots);
    assert_eq!(result.len(), 2);
  }

  #[test]
  fn walk_subtree_missing_root_returns_empty() {
    let snapshots = HashMap::new();
    let children = HashMap::new();
    let result = walk_subtree(pid(1), &children, &snapshots);
    assert!(result.is_empty());
  }

  #[test]
  fn compute_per_process_stats_sorts_root_first_then_by_avg_cpu() {
    let mut tracks: HashMap<Pid, ProcessTrack> = HashMap::new();
    tracks.insert(pid(1), ProcessTrack {
      name: "root".into(),
      is_root: true,
      cpu_samples: vec![1.0, 1.0],
      memory_samples_mb: vec![10.0, 10.0],
    });
    tracks.insert(pid(2), ProcessTrack {
      name: "low-cpu".into(),
      is_root: false,
      cpu_samples: vec![5.0],
      memory_samples_mb: vec![20.0],
    });
    tracks.insert(pid(3), ProcessTrack {
      name: "hi-cpu".into(),
      is_root: false,
      cpu_samples: vec![90.0],
      memory_samples_mb: vec![30.0],
    });

    let stats = compute_per_process_stats(tracks);
    assert_eq!(stats.len(), 3);
    assert_eq!(stats[0].name, "root");
    assert_eq!(stats[1].name, "hi-cpu");
    assert_eq!(stats[2].name, "low-cpu");
  }

  #[test]
  fn compute_per_process_stats_basic_metrics() {
    let mut tracks: HashMap<Pid, ProcessTrack> = HashMap::new();
    tracks.insert(pid(7), ProcessTrack {
      name: "worker".into(),
      is_root: false,
      cpu_samples: vec![10.0, 20.0, 30.0],
      memory_samples_mb: vec![100.0, 150.0, 200.0],
    });

    let stats = compute_per_process_stats(tracks);
    assert_eq!(stats.len(), 1);
    let s = &stats[0];
    assert_eq!(s.pid, 7);
    assert_eq!(s.sample_count, 3);
    assert!((s.avg_cpu - 20.0).abs() < 0.01);
    assert!((s.max_cpu - 30.0).abs() < 0.01);
    assert!((s.avg_memory_mb - 150.0).abs() < 0.01);
    assert!((s.max_memory_mb - 200.0).abs() < 0.01);
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

    let result = compute_result(samples, HashMap::new(), 3, 5);

    assert_eq!(result.duration_seconds, 3);
    assert_eq!(result.warmup_seconds, 5);
    assert!((result.avg_cpu - 20.0).abs() < 0.01);
    assert!((result.max_cpu - 30.0).abs() < 0.01);
    assert!((result.avg_memory_mb - 150.0).abs() < 0.01);
    assert!((result.memory_growth_mb - 50.0).abs() < 0.01);
    assert!(result.per_process.is_empty());
  }
}
