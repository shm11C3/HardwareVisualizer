use std::collections::{HashMap, HashSet};
use std::io::Read as _;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use sysinfo::{Pid, Process, ProcessesToUpdate, System};

use crate::config::Timing;

const BYTES_PER_MIB: f64 = 1024.0 * 1024.0;
const WEBVIEW_PROCESS_MARKERS: &[&str] = &[
  "msedgewebview2",
  "webkitwebprocess",
  "webkitwebproces",
  "webkitnetworkprocess",
  "webkitnetwork",
  "webkitnetworkp",
  "webkitgpu",
  "com.apple.webkit.webcontent",
  "com.apple.webkit.networking",
  "com.apple.webkit.gpu",
];

#[derive(Debug)]
pub struct ProcessMetrics {
  pub cpu_usage: f32,
  pub memory_rss_mb: f64,
  pub parent_memory_rss_mb: f64,
  pub webview_memory_rss_mb: f64,
  pub other_process_memory_rss_mb: f64,
  pub process_count: usize,
  pub webview_process_count: usize,
}

#[derive(Debug)]
pub struct MemoryStats {
  pub avg_mb: f64,
  pub max_mb: f64,
  pub p95_mb: f64,
  pub growth_mb: f64,
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
  pub parent_memory: MemoryStats,
  pub webview_memory: MemoryStats,
  pub other_process_memory: MemoryStats,
  pub max_process_count: usize,
  pub max_webview_process_count: usize,
  pub duration_seconds: u64,
  pub warmup_seconds: u64,
}

#[derive(Debug)]
struct ProcessTracker {
  root_pid: Pid,
  #[cfg(any(target_os = "windows", target_os = "macos"))]
  preexisting_webview_pids: HashSet<Pid>,
  #[cfg(target_os = "macos")]
  app_identity: AppIdentity,
}

#[cfg(any(test, target_os = "macos"))]
#[derive(Debug)]
struct AppIdentity {
  tokens: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProcessSnapshot {
  parent: Option<Pid>,
  memory_bytes: u64,
  cpu_usage: f32,
  search_text: String,
}

#[derive(Debug, Default, PartialEq)]
struct ProcessMemoryBreakdown {
  parent_memory_bytes: u64,
  webview_memory_bytes: u64,
  other_process_memory_bytes: u64,
  process_count: usize,
  webview_process_count: usize,
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
  let preexisting_webview_pids = collect_preexisting_webview_pids();

  let mut guard = ProcessGuard(launch_process(binary_path)?);
  let pid = Pid::from_u32(guard.0.id());
  let tracker = ProcessTracker::new(pid, binary_path, preexisting_webview_pids);
  let num_cpus = thread::available_parallelism()
    .map(|n| n.get() as f32)
    .unwrap_or(1.0);

  let mut system = System::new();

  // Initial refresh to establish CPU baselines and discover helper processes.
  system.refresh_processes(ProcessesToUpdate::All, true);

  eprintln!("Warming up for {} seconds...", timing.warmup_seconds);

  // Warmup phase: wait for app to stabilize
  for _ in 0..timing.warmup_seconds {
    thread::sleep(Duration::from_secs(1));
    if guard.0.try_wait()?.is_some() {
      return Err(format_exit_error(&mut guard, "warmup", binary_path));
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
      return Err(format_exit_error(&mut guard, "measurement", binary_path));
    }

    system.refresh_processes(ProcessesToUpdate::All, true);

    if let Some(sample) = tracker.sample(&system, num_cpus) {
      samples.push(sample);
    } else {
      if guard.0.try_wait()?.is_some() {
        return Err(format_exit_error(&mut guard, "measurement", binary_path));
      }
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

impl ProcessTracker {
  fn new(
    root_pid: Pid,
    binary_path: &Path,
    preexisting_webview_pids: HashSet<Pid>,
  ) -> Self {
    #[cfg(target_os = "macos")]
    {
      Self {
        root_pid,
        preexisting_webview_pids,
        app_identity: AppIdentity::from_binary_path(binary_path),
      }
    }

    #[cfg(target_os = "windows")]
    {
      let _ = binary_path;
      Self {
        root_pid,
        preexisting_webview_pids,
      }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
      let _ = binary_path;
      let _ = preexisting_webview_pids;
      Self { root_pid }
    }
  }

  fn sample(&self, system: &System, num_cpus: f32) -> Option<ProcessMetrics> {
    let processes = snapshot_processes(system);
    self.sample_from_snapshots(&processes, num_cpus)
  }

  fn sample_from_snapshots(
    &self,
    processes: &HashMap<Pid, ProcessSnapshot>,
    num_cpus: f32,
  ) -> Option<ProcessMetrics> {
    let root_process = processes.get(&self.root_pid)?;
    let tracked_pids = self.tracked_pids_from_snapshots(processes);
    let breakdown = process_memory_breakdown(&tracked_pids, self.root_pid, processes);
    let total_memory_bytes = breakdown.parent_memory_bytes
      + breakdown.webview_memory_bytes
      + breakdown.other_process_memory_bytes;

    Some(ProcessMetrics {
      // Keep CPU semantics scoped to the launched process; issue #1579 changes
      // the RSS accounting that feeds memory thresholds.
      cpu_usage: root_process.cpu_usage / num_cpus,
      memory_rss_mb: bytes_to_mib(total_memory_bytes),
      parent_memory_rss_mb: bytes_to_mib(breakdown.parent_memory_bytes),
      webview_memory_rss_mb: bytes_to_mib(breakdown.webview_memory_bytes),
      other_process_memory_rss_mb: bytes_to_mib(breakdown.other_process_memory_bytes),
      process_count: breakdown.process_count,
      webview_process_count: breakdown.webview_process_count,
    })
  }

  fn tracked_pids_from_snapshots(
    &self,
    processes: &HashMap<Pid, ProcessSnapshot>,
  ) -> HashSet<Pid> {
    let mut pids = HashSet::from([self.root_pid]);

    for pid in processes.keys().copied() {
      if pid != self.root_pid && is_descendant_snapshot(processes, pid, self.root_pid) {
        pids.insert(pid);
      }
    }

    #[cfg(target_os = "macos")]
    for (pid, process) in processes {
      if !pids.contains(pid) && self.is_associated_macos_webview(*pid, process) {
        pids.insert(*pid);
      }
    }

    #[cfg(target_os = "windows")]
    for (pid, process) in processes {
      if !pids.contains(pid) && self.is_associated_windows_webview(*pid, process) {
        pids.insert(*pid);
      }
    }

    pids
  }

  #[cfg(target_os = "macos")]
  fn is_associated_macos_webview(&self, pid: Pid, process: &ProcessSnapshot) -> bool {
    is_associated_macos_webview_snapshot(
      &self.preexisting_webview_pids,
      Some(&self.app_identity),
      pid,
      process,
    )
  }

  #[cfg(target_os = "windows")]
  fn is_associated_windows_webview(&self, pid: Pid, process: &ProcessSnapshot) -> bool {
    is_associated_webview_snapshot(&self.preexisting_webview_pids, pid, process)
  }
}

#[cfg(any(test, target_os = "macos"))]
impl AppIdentity {
  fn from_binary_path(binary_path: &Path) -> Self {
    let mut tokens = Vec::new();

    if let Some(stem) = binary_path.file_stem().and_then(|s| s.to_str()) {
      push_identity_token(&mut tokens, stem);
    }

    for component in binary_path.components() {
      let Some(component) = component.as_os_str().to_str() else {
        continue;
      };

      if let Some(bundle_name) = component.strip_suffix(".app") {
        push_identity_token(&mut tokens, bundle_name);
      }
    }

    Self { tokens }
  }

  fn matches_text(&self, text: &str) -> bool {
    let compact_text = compact_token(text);

    self
      .tokens
      .iter()
      .any(|token| text.contains(token) || compact_text.contains(token))
  }
}

#[cfg(any(test, target_os = "macos"))]
fn push_identity_token(tokens: &mut Vec<String>, token: &str) {
  let normalized = token.to_ascii_lowercase();
  if !normalized.is_empty() && !tokens.contains(&normalized) {
    tokens.push(normalized);
  }

  let compact = compact_token(token);
  if !compact.is_empty() && !tokens.contains(&compact) {
    tokens.push(compact);
  }
}

#[cfg(any(test, target_os = "macos"))]
fn compact_token(token: &str) -> String {
  token
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .flat_map(char::to_lowercase)
    .collect()
}

fn launch_process(binary_path: &Path) -> Result<Child, Box<dyn std::error::Error>> {
  let child = Command::new(binary_path).stderr(Stdio::piped()).spawn()?;
  eprintln!("Launched process with PID: {}", child.id());
  Ok(child)
}

fn format_exit_error(
  guard: &mut ProcessGuard,
  phase: &str,
  binary_path: &Path,
) -> Box<dyn std::error::Error> {
  let status = guard.0.try_wait().ok().flatten();
  let stderr = guard.0.stderr.take().and_then(|mut s| {
    let mut buf = String::new();
    s.read_to_string(&mut buf).ok()?;
    if buf.is_empty() { None } else { Some(buf) }
  });

  format_exit_message(
    phase,
    status.as_ref().map(ToString::to_string).as_deref(),
    status.as_ref().is_some_and(|st| st.success()),
    stderr.as_deref(),
    binary_path,
  )
  .into()
}

fn format_exit_message(
  phase: &str,
  status: Option<&str>,
  exited_successfully: bool,
  stderr: Option<&str>,
  binary_path: &Path,
) -> String {
  let mut msg = format!("Process exited during {phase}");
  if let Some(st) = status {
    msg.push_str(&format!(" (exit status: {st})"));
  }
  if exited_successfully {
    msg.push_str(
      "\n\nHint: the target exited successfully before monitoring completed. \
       If HardwareVisualizer is already running, Tauri's single-instance \
       plugin can hand this launch off to the existing instance and exit the \
       measured process. Close existing `hardware-visualizer` processes and \
       rerun the perf test.",
    );
    msg.push_str(&format!("\nTarget: {}", binary_path.display()));
  }
  if let Some(err) = stderr {
    msg.push_str(&format!("\n--- process stderr ---\n{err}"));
  }
  msg
}

fn collect_preexisting_webview_pids() -> HashSet<Pid> {
  #[cfg(any(target_os = "windows", target_os = "macos"))]
  {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    collect_webview_pids(&system)
  }

  #[cfg(not(any(target_os = "windows", target_os = "macos")))]
  {
    HashSet::new()
  }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn collect_webview_pids(system: &System) -> HashSet<Pid> {
  system
    .processes()
    .iter()
    .filter_map(|(pid, process)| {
      is_webview_process_text(&process_search_text(process)).then_some(*pid)
    })
    .collect()
}

fn snapshot_processes(system: &System) -> HashMap<Pid, ProcessSnapshot> {
  system
    .processes()
    .iter()
    .map(|(pid, process)| {
      (
        *pid,
        ProcessSnapshot {
          parent: process.parent(),
          memory_bytes: process.memory(),
          cpu_usage: process.cpu_usage(),
          search_text: process_search_text(process),
        },
      )
    })
    .collect()
}

fn is_descendant_snapshot(
  processes: &HashMap<Pid, ProcessSnapshot>,
  pid: Pid,
  root_pid: Pid,
) -> bool {
  let mut current_pid = pid;
  let mut visited = HashSet::new();

  while let Some(process) = processes.get(&current_pid) {
    let Some(parent_pid) = process.parent else {
      return false;
    };

    if parent_pid == root_pid {
      return true;
    }

    if !visited.insert(parent_pid) {
      return false;
    }

    current_pid = parent_pid;
  }

  false
}

fn process_memory_breakdown(
  pids: &HashSet<Pid>,
  root_pid: Pid,
  processes: &HashMap<Pid, ProcessSnapshot>,
) -> ProcessMemoryBreakdown {
  let mut breakdown = ProcessMemoryBreakdown::default();

  for pid in pids {
    let Some(process) = processes.get(pid) else {
      continue;
    };

    breakdown.process_count += 1;

    if *pid == root_pid {
      breakdown.parent_memory_bytes += process.memory_bytes;
    } else if is_webview_process_text(&process.search_text) {
      breakdown.webview_memory_bytes += process.memory_bytes;
      breakdown.webview_process_count += 1;
    } else {
      breakdown.other_process_memory_bytes += process.memory_bytes;
    }
  }

  breakdown
}

fn is_webview_process_text(text: &str) -> bool {
  let text = text.to_ascii_lowercase();
  WEBVIEW_PROCESS_MARKERS
    .iter()
    .any(|marker| text.contains(marker))
}

#[cfg(any(test, target_os = "macos"))]
fn is_associated_macos_webview_snapshot(
  preexisting_webview_pids: &HashSet<Pid>,
  app_identity: Option<&AppIdentity>,
  pid: Pid,
  process: &ProcessSnapshot,
) -> bool {
  if preexisting_webview_pids.contains(&pid)
    || !is_webview_process_text(&process.search_text)
  {
    return false;
  }

  if app_identity.is_some_and(|identity| identity.matches_text(&process.search_text)) {
    return true;
  }

  // sysinfo does not expose macOS responsibility metadata. The monitor runs in
  // an isolated CI session, so newly-created WebKit helpers are treated as
  // associated after excluding helpers that existed before launch.
  true
}

#[cfg(any(test, target_os = "windows"))]
fn is_associated_webview_snapshot(
  preexisting_webview_pids: &HashSet<Pid>,
  pid: Pid,
  process: &ProcessSnapshot,
) -> bool {
  if preexisting_webview_pids.contains(&pid) {
    return false;
  }

  // WebView2 helper processes are not always reported as descendants of the
  // launched app process. In the isolated perf-monitor run, newly-created
  // WebView helpers are part of the app footprint.
  if !is_webview_process_text(&process.search_text) {
    return false;
  }

  true
}

fn process_search_text(process: &Process) -> String {
  let mut parts = Vec::new();

  parts.push(process.name().to_string_lossy().into_owned());

  if let Some(exe) = process.exe() {
    parts.push(exe.to_string_lossy().into_owned());
  }

  parts.extend(
    process
      .cmd()
      .iter()
      .map(|arg| arg.to_string_lossy().into_owned()),
  );

  parts.join(" ").to_ascii_lowercase()
}

fn bytes_to_mib(bytes: u64) -> f64 {
  bytes as f64 / BYTES_PER_MIB
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

  let total_memory = memory_stats(&samples, |s| s.memory_rss_mb);
  let parent_memory = memory_stats(&samples, |s| s.parent_memory_rss_mb);
  let webview_memory = memory_stats(&samples, |s| s.webview_memory_rss_mb);
  let other_process_memory = memory_stats(&samples, |s| s.other_process_memory_rss_mb);
  let max_process_count = samples.iter().map(|s| s.process_count).max().unwrap_or(0);
  let max_webview_process_count = samples
    .iter()
    .map(|s| s.webview_process_count)
    .max()
    .unwrap_or(0);

  MonitorResult {
    samples,
    avg_cpu,
    max_cpu,
    p95_cpu,
    avg_memory_mb: total_memory.avg_mb,
    max_memory_mb: total_memory.max_mb,
    p95_memory_mb: total_memory.p95_mb,
    memory_growth_mb: total_memory.growth_mb,
    parent_memory,
    webview_memory,
    other_process_memory,
    max_process_count,
    max_webview_process_count,
    duration_seconds,
    warmup_seconds,
  }
}

fn memory_stats<F>(samples: &[ProcessMetrics], value: F) -> MemoryStats
where
  F: Fn(&ProcessMetrics) -> f64,
{
  if samples.is_empty() {
    return MemoryStats {
      avg_mb: 0.0,
      max_mb: 0.0,
      p95_mb: 0.0,
      growth_mb: 0.0,
    };
  }

  let values = samples.iter().map(value).collect::<Vec<_>>();
  let avg_mb = values.iter().sum::<f64>() / values.len() as f64;
  let max_mb = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
  let p95_mb = percentile_f64(&values, 95.0);
  let first_mb = values.first().copied().unwrap_or(0.0);
  let last_mb = values.last().copied().unwrap_or(0.0);

  MemoryStats {
    avg_mb,
    max_mb,
    p95_mb,
    growth_mb: last_mb - first_mb,
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
      sample(10.0, 100.0, 80.0, 15.0, 5.0),
      sample(20.0, 200.0, 90.0, 100.0, 10.0),
      sample(30.0, 150.0, 85.0, 60.0, 5.0),
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

  #[test]
  fn compute_result_tracks_memory_breakdown() {
    let samples = vec![
      sample(10.0, 100.0, 80.0, 15.0, 5.0),
      sample(20.0, 200.0, 90.0, 100.0, 10.0),
      sample(30.0, 150.0, 85.0, 60.0, 5.0),
    ];

    let result = compute_result(samples, 3, 5);

    assert!((result.parent_memory.avg_mb - 85.0).abs() < 0.01);
    assert!((result.webview_memory.avg_mb - 58.33).abs() < 0.01);
    assert!((result.other_process_memory.max_mb - 10.0).abs() < 0.01);
    assert!((result.webview_memory.growth_mb - 45.0).abs() < 0.01);
    assert_eq!(result.max_process_count, 3);
    assert_eq!(result.max_webview_process_count, 1);
  }

  #[test]
  fn tracked_pids_include_root_children_and_grandchildren_only() {
    let processes = HashMap::from([
      (pid(10), snapshot(None, 50, 20.0, "hardware-visualizer.exe")),
      (pid(11), snapshot(Some(pid(10)), 10, 0.0, "child.exe")),
      (pid(12), snapshot(Some(pid(11)), 10, 0.0, "grandchild.exe")),
      (pid(20), snapshot(None, 10, 0.0, "unrelated.exe")),
      (
        pid(21),
        snapshot(Some(pid(20)), 10, 0.0, "unrelated-child.exe"),
      ),
    ]);

    let tracked = tracker().tracked_pids_from_snapshots(&processes);

    assert_eq!(tracked, HashSet::from([pid(10), pid(11), pid(12)]));
  }

  #[test]
  fn descendant_detection_stops_on_missing_parent_and_cycles() {
    let processes = HashMap::from([
      (pid(10), snapshot(None, 50, 20.0, "hardware-visualizer.exe")),
      (
        pid(30),
        snapshot(Some(pid(99)), 10, 0.0, "missing-parent.exe"),
      ),
      (pid(40), snapshot(Some(pid(41)), 10, 0.0, "cycle-a.exe")),
      (pid(41), snapshot(Some(pid(40)), 10, 0.0, "cycle-b.exe")),
    ]);

    assert!(!is_descendant_snapshot(&processes, pid(30), pid(10)));
    assert!(!is_descendant_snapshot(&processes, pid(40), pid(10)));
  }

  #[test]
  fn sample_from_snapshots_sums_parent_webview_and_other_processes() {
    let processes = HashMap::from([
      (pid(10), snapshot(None, 50, 20.0, "hardware-visualizer.exe")),
      (
        pid(11),
        snapshot(Some(pid(10)), 200, 0.0, "msedgewebview2.exe"),
      ),
      (
        pid(12),
        snapshot(Some(pid(10)), 25, 0.0, "hardware-helper.exe"),
      ),
      (
        pid(20),
        snapshot(None, 100, 0.0, "unrelated-msedgewebview2.exe"),
      ),
    ]);

    let sample = tracker_with_preexisting_webviews(HashSet::from([pid(20)]))
      .sample_from_snapshots(&processes, 2.0)
      .expect("root process should be present");

    assert!((sample.cpu_usage - 10.0).abs() < 0.01);
    assert!((sample.memory_rss_mb - 275.0).abs() < 0.01);
    assert!((sample.parent_memory_rss_mb - 50.0).abs() < 0.01);
    assert!((sample.webview_memory_rss_mb - 200.0).abs() < 0.01);
    assert!((sample.other_process_memory_rss_mb - 25.0).abs() < 0.01);
    assert_eq!(sample.process_count, 3);
    assert_eq!(sample.webview_process_count, 1);
  }

  #[test]
  fn webview_process_text_matches_known_helper_names() {
    assert!(is_webview_process_text(
      r"C:\Program Files\WebView2\msedgewebview2.exe"
    ));
    assert!(is_webview_process_text("WebKitWebProcess"));
    assert!(is_webview_process_text("WebKitNetworkProcess"));
    assert!(is_webview_process_text("com.apple.WebKit.WebContent"));
    assert!(is_webview_process_text("com.apple.WebKit.Networking"));
    assert!(!is_webview_process_text("hardware-visualizer.exe"));
  }

  #[test]
  fn webview_association_excludes_preexisting_helpers() {
    let preexisting = HashSet::from([pid(42)]);
    let existing_webview = snapshot(None, 100, 0.0, "msedgewebview2.exe");
    let new_webview = snapshot(None, 100, 0.0, "msedgewebview2.exe");
    let non_webview = snapshot(None, 100, 0.0, "hardware-visualizer.exe");

    assert!(!is_associated_webview_snapshot(
      &preexisting,
      pid(42),
      &existing_webview,
    ));
    assert!(!is_associated_webview_snapshot(
      &preexisting,
      pid(43),
      &non_webview,
    ));
    assert!(is_associated_webview_snapshot(
      &preexisting,
      pid(44),
      &new_webview,
    ));
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn tracked_pids_include_new_windows_webview_helpers_without_parent_link() {
    let processes = HashMap::from([
      (pid(10), snapshot(None, 50, 20.0, "hardware-visualizer.exe")),
      (pid(11), snapshot(None, 200, 0.0, "msedgewebview2.exe")),
      (pid(12), snapshot(None, 100, 0.0, "msedgewebview2.exe")),
    ]);
    let tracker = ProcessTracker::new(
      pid(10),
      Path::new("hardware-visualizer.exe"),
      HashSet::from([pid(12)]),
    );

    let tracked = tracker.tracked_pids_from_snapshots(&processes);

    assert_eq!(tracked, HashSet::from([pid(10), pid(11)]));
  }

  #[test]
  fn macos_webview_association_excludes_preexisting_helpers() {
    let identity = AppIdentity::from_binary_path(Path::new(
      "/Applications/HardwareVisualizer.app/Contents/MacOS/hardware-visualizer",
    ));
    let preexisting = HashSet::from([pid(42)]);
    let existing_webkit = snapshot(None, 100, 0.0, "com.apple.WebKit.WebContent");
    let new_webkit = snapshot(None, 100, 0.0, "com.apple.WebKit.WebContent");
    let non_webkit = snapshot(None, 100, 0.0, "hardware-visualizer.exe");

    assert!(!is_associated_macos_webview_snapshot(
      &preexisting,
      Some(&identity),
      pid(42),
      &existing_webkit,
    ));
    assert!(!is_associated_macos_webview_snapshot(
      &preexisting,
      Some(&identity),
      pid(43),
      &non_webkit,
    ));
    assert!(is_associated_macos_webview_snapshot(
      &preexisting,
      Some(&identity),
      pid(44),
      &new_webkit,
    ));
  }

  #[test]
  fn successful_early_exit_message_mentions_single_instance_hint() {
    let message = format_exit_message(
      "warmup",
      Some("exit code: 0"),
      true,
      None,
      Path::new("target/release/hardware-visualizer.exe"),
    );

    assert!(message.contains("Process exited during warmup"));
    assert!(message.contains("exit code: 0"));
    assert!(message.contains("HardwareVisualizer is already running"));
    assert!(message.contains("single-instance"));
    assert!(message.contains("target/release/hardware-visualizer.exe"));
  }

  #[test]
  fn failing_early_exit_message_keeps_stderr_without_single_instance_hint() {
    let message = format_exit_message(
      "measurement",
      Some("exit code: 1"),
      false,
      Some("startup failed"),
      Path::new("target/release/hardware-visualizer.exe"),
    );

    assert!(message.contains("Process exited during measurement"));
    assert!(message.contains("exit code: 1"));
    assert!(message.contains("--- process stderr ---"));
    assert!(message.contains("startup failed"));
    assert!(!message.contains("HardwareVisualizer is already running"));
  }

  #[test]
  fn compact_token_removes_separators() {
    assert_eq!(
      compact_token("Hardware-Visualizer.app"),
      "hardwarevisualizerapp"
    );
  }

  #[test]
  fn identity_tokens_include_binary_and_bundle_variants() {
    let identity = AppIdentity::from_binary_path(Path::new(
      "/Applications/HardwareVisualizer.app/Contents/MacOS/hardware-visualizer",
    ));

    assert!(identity.tokens.contains(&"hardware-visualizer".to_string()));
    assert!(identity.tokens.contains(&"hardwarevisualizer".to_string()));
  }

  fn pid(value: u32) -> Pid {
    Pid::from_u32(value)
  }

  fn tracker() -> ProcessTracker {
    tracker_with_preexisting_webviews(HashSet::new())
  }

  fn tracker_with_preexisting_webviews(
    preexisting_webview_pids: HashSet<Pid>,
  ) -> ProcessTracker {
    ProcessTracker::new(
      pid(10),
      Path::new("hardware-visualizer.exe"),
      preexisting_webview_pids,
    )
  }

  fn snapshot(
    parent: Option<Pid>,
    memory_mb: u64,
    cpu_usage: f32,
    search_text: &str,
  ) -> ProcessSnapshot {
    ProcessSnapshot {
      parent,
      memory_bytes: memory_mb * 1024 * 1024,
      cpu_usage,
      search_text: search_text.to_ascii_lowercase(),
    }
  }

  fn sample(
    cpu_usage: f32,
    memory_rss_mb: f64,
    parent_memory_rss_mb: f64,
    webview_memory_rss_mb: f64,
    other_process_memory_rss_mb: f64,
  ) -> ProcessMetrics {
    ProcessMetrics {
      cpu_usage,
      memory_rss_mb,
      parent_memory_rss_mb,
      webview_memory_rss_mb,
      other_process_memory_rss_mb,
      process_count: 3,
      webview_process_count: 1,
    }
  }
}
