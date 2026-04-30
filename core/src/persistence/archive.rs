//! Hardware-archive worker.
//!
//! The worker subscribes to [`crate::event_bus::EventBus`] and maintains
//! its own rolling buffers of per-second samples. Every
//! [`HARDWARE_ARCHIVE_INTERVAL_SECONDS`] it computes avg/min/max stats
//! and writes them through [`crate::infrastructure::database`].
//!
//! Persistence intentionally does **not** share the collector's
//! `HistoryStore` `Arc<Mutex<...>>` bag: the only path from collector to
//! persistence is the broadcast channel, which means a slow DB write
//! cannot back-pressure sensor polling.

use std::collections::{HashMap, HashSet, VecDeque};

use tokio::runtime::Handle;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{Duration, MissedTickBehavior, interval};

use crate::event_bus::EventBus;
use crate::log_warn;
use crate::models::{MetricsSnapshot, ProcessSample};
use crate::persistence::archive_data::{GpuData, HardwareData, ProcessStatData};

/// Interval between archive writes. Matches the 60-sample history
/// buffer the collector keeps, so each archive row summarizes one
/// minute of samples.
pub const HARDWARE_ARCHIVE_INTERVAL_SECONDS: u64 = 60;

const PROCESS_RECORD_LIMIT: usize = 5;
const PROCESS_HISTORY_BUFFER: usize = 60;

#[derive(Debug, Clone, Copy)]
enum ProcessRankingMetric {
  Cpu,
  Memory,
  ExecutionTime,
}

impl ProcessRankingMetric {
  const ALL: [Self; 3] = [Self::Cpu, Self::Memory, Self::ExecutionTime];
}

/// Background controller for the archive worker. Returned by
/// [`ArchiveController::setup`]; call [`ArchiveController::terminate`]
/// to drain the task on shutdown.
pub struct ArchiveController {
  handle: JoinHandle<()>,
  stop_tx: watch::Sender<bool>,
}

impl ArchiveController {
  /// Spawn the archive worker on `runtime`. The worker subscribes to
  /// `bus` immediately so no snapshots are missed between setup and the
  /// first tick.
  pub fn setup(bus: &EventBus, runtime: Handle) -> Self {
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let mut rx = bus.subscribe();

    let handle = runtime.spawn(async move {
      let mut tracker = ArchiveTracker::new();
      let mut ticker =
        interval(Duration::from_secs(HARDWARE_ARCHIVE_INTERVAL_SECONDS));
      ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
      // The first tick fires immediately; consume it so the first archive
      // write is delayed by a full interval and has data to summarize.
      ticker.tick().await;

      loop {
        tokio::select! {
          biased;
          changed = stop_rx.changed() => {
            if changed.is_err() || *stop_rx.borrow() {
              eprintln!("[hardware-archive] shutdown signal received");
              break;
            }
          }
          received = rx.recv() => match received {
            Ok(snapshot) => tracker.ingest(snapshot),
            Err(RecvError::Lagged(skipped)) => {
              log_warn!(
                &format!("archive worker lagged, dropped {skipped} snapshot(s)"),
                "persistence::archive",
                None::<&str>
              );
            }
            Err(RecvError::Closed) => break,
          },
          _ = ticker.tick() => {
            let start = std::time::Instant::now();
            tracker.write_archive().await;
            let elapsed = start.elapsed();
            if elapsed > Duration::from_secs(HARDWARE_ARCHIVE_INTERVAL_SECONDS) {
              log_warn!(
                &format!("overrun {:?} (> {}s)", elapsed, HARDWARE_ARCHIVE_INTERVAL_SECONDS),
                "persistence::archive",
                None::<&str>
              );
            }
          }
        }
      }
    });

    Self { handle, stop_tx }
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}

/// Delete archive rows older than `retention_days`. Errors are logged
/// per-table so a partial failure (e.g. a table missing on an old
/// schema) doesn't suppress cleanup of the remaining tables.
///
/// This is a **one-shot** operation: it deletes everything older than
/// the cutoff at the moment of the call and then returns. App calls it
/// once at startup when `hardware_archive.scheduled_data_deletion` is
/// enabled — that matches the pre-Phase-4 behavior of the previous
/// `batch_delete_old_data` wrapper. The setting's name is historical;
/// the current refresh trigger is "next process boot", not a recurring
/// schedule. Adding a true periodic cleanup is left to the issue that
/// revisits the retention UX rather than this Core/App split.
pub async fn cleanup_old_data(retention_days: u32) {
  use crate::infrastructure::database;
  use crate::log_error;

  if let Err(e) = database::hardware_archive::delete_old_data(retention_days).await {
    log_error!(
      "Failed to delete old hardware archive data",
      "persistence::archive::cleanup_old_data",
      Some(e.to_string())
    );
  }

  if let Err(e) = database::gpu_archive::delete_old_data(retention_days).await {
    log_error!(
      "Failed to delete old GPU hardware archive data",
      "persistence::archive::cleanup_old_data",
      Some(e.to_string())
    );
  }

  if let Err(e) = database::process_stats::delete_old_data(retention_days).await {
    log_error!(
      "Failed to delete old process stats data",
      "persistence::archive::cleanup_old_data",
      Some(e.to_string())
    );
  }
}

// ── Internal: per-snapshot accumulator ────────────────────────────────

/// How long a vanished process is kept around before it's evicted from
/// [`ArchiveTracker::processes`]. Sized to the archive interval so a
/// short-lived process that exits within the active window still
/// appears in the next archive row, then drops on the following tick.
const PROCESS_GRACE_TICKS: u64 = HARDWARE_ARCHIVE_INTERVAL_SECONDS;

/// Cached process info accumulated from EventBus payloads. `name` and
/// `run_time_secs` come from the most recent snapshot; the CPU and
/// memory rings are 60-sample windows used to compute averages on the
/// archive tick. `last_seen_tick` lets the tracker keep a process
/// around for one full archive window after it disappears so a
/// short-lived offender that exits late in the window is still archived.
#[derive(Default)]
struct ProcessAccumulator {
  name: String,
  run_time_secs: u64,
  cpu_history: VecDeque<f32>,
  memory_kb_history: VecDeque<f32>,
  last_seen_tick: u64,
}

#[derive(Default)]
struct ArchiveTracker {
  cpu_history: VecDeque<f32>,
  memory_history: VecDeque<f32>,
  gpu_usage_histories: HashMap<String, VecDeque<f32>>,
  gpu_temperature_histories: HashMap<String, VecDeque<i32>>,
  gpu_dedicated_memory_histories: HashMap<String, VecDeque<i32>>,
  gpu_name_map: HashMap<String, String>,
  processes: HashMap<u32, ProcessAccumulator>,
  /// Monotonic counter incremented on every `ingest` call. Used to age
  /// out vanished processes via [`PROCESS_GRACE_TICKS`].
  tick_counter: u64,
  /// Number of CPU cores observed in the most recent snapshot. Used to
  /// normalize sysinfo's per-core CPU values to a 0-100 percentage.
  cores: f32,
}

impl ArchiveTracker {
  fn new() -> Self {
    Self::default()
  }

  fn ingest(&mut self, snapshot: MetricsSnapshot) {
    self.tick_counter = self.tick_counter.saturating_add(1);
    push_capped(&mut self.cpu_history, snapshot.cpu_usage);
    push_capped(&mut self.memory_history, snapshot.memory_usage);

    if !snapshot.processors_usage.is_empty() {
      self.cores = snapshot.processors_usage.len() as f32;
    }

    for gpu in snapshot.gpus {
      self.gpu_name_map.insert(gpu.gpu_id.clone(), gpu.gpu_name);
      if let Some(usage) = gpu.gpu_usage {
        push_capped(
          self
            .gpu_usage_histories
            .entry(gpu.gpu_id.clone())
            .or_default(),
          usage,
        );
      }
      if let Some(temperature) = gpu.gpu_temperature {
        push_capped_i32(
          self
            .gpu_temperature_histories
            .entry(gpu.gpu_id.clone())
            .or_default(),
          temperature as i32,
        );
      }
      if let Some(memory_kb) = gpu.gpu_dedicated_memory_usage_kb {
        push_capped_i32(
          self
            .gpu_dedicated_memory_histories
            .entry(gpu.gpu_id.clone())
            .or_default(),
          memory_kb as i32,
        );
      }
    }

    self.update_process_accumulators(snapshot.processes);
  }

  fn update_process_accumulators(&mut self, samples: Vec<ProcessSample>) {
    let now = self.tick_counter;

    // Age out accumulators that haven't been refreshed within the
    // grace window. Evicting on every snapshot (the previous behavior)
    // dropped processes that exited just before the archive tick — a
    // short-lived offender that burned CPU for most of the minute and
    // then exited would never appear in the archive row. Keeping
    // entries for [`PROCESS_GRACE_TICKS`] solves that without letting
    // the map grow unbounded.
    self
      .processes
      .retain(|_pid, acc| now.saturating_sub(acc.last_seen_tick) <= PROCESS_GRACE_TICKS);

    for sample in samples {
      let entry = self.processes.entry(sample.pid).or_default();
      // PID was reused after a gap → assume a different process and
      // reset the rings so we don't mix samples across distinct
      // processes that happened to inherit the same PID.
      let absent_ticks = now.saturating_sub(entry.last_seen_tick);
      if entry.last_seen_tick != 0 && absent_ticks > 1 {
        entry.cpu_history.clear();
        entry.memory_kb_history.clear();
      }
      entry.name = sample.name;
      entry.run_time_secs = sample.run_time_secs;
      entry.last_seen_tick = now;
      push_capped(&mut entry.cpu_history, sample.cpu_usage);
      push_capped(&mut entry.memory_kb_history, sample.memory_kb);
    }
  }

  async fn write_archive(&self) {
    use crate::infrastructure::database;
    use crate::log_error;

    let cpu = StatsCalculator::compute_stats(self.cpu_history.iter().copied());
    let memory = StatsCalculator::compute_stats(self.memory_history.iter().copied());

    if let Err(e) = database::hardware_archive::insert(cpu, memory).await {
      log_error!(
        "Failed to insert hardware archive data",
        "persistence::archive::write_archive",
        Some(e.to_string())
      );
    }

    for gpu in self.collect_gpu_data() {
      if let Err(e) = database::gpu_archive::insert(gpu).await {
        log_error!(
          "Failed to insert GPU hardware archive data",
          "persistence::archive::write_archive",
          Some(e.to_string())
        );
      }
    }

    let process_stats = self.collect_process_stats();
    if !process_stats.is_empty()
      && let Err(e) = database::process_stats::insert(process_stats).await
    {
      log_error!(
        "Failed to insert process stats data",
        "persistence::archive::write_archive",
        Some(e.to_string())
      );
    }
  }

  fn collect_gpu_data(&self) -> Vec<GpuData> {
    // Walk the union of every GPU history map so adapters that report
    // only temperature or only dedicated memory (e.g. Linux DRM, ADL
    // without VRAM, Apple IOKit without thermals) still produce a row
    // — iterating `gpu_usage_histories` alone would silently drop them.
    let gpu_ids: HashSet<&String> = self
      .gpu_usage_histories
      .keys()
      .chain(self.gpu_temperature_histories.keys())
      .chain(self.gpu_dedicated_memory_histories.keys())
      .collect();

    gpu_ids
      .into_iter()
      .map(|gpu_id| {
        let (usage_avg, usage_max, usage_min) = StatsCalculator::compute_f32_aggregates(
          self
            .gpu_usage_histories
            .get(gpu_id)
            .map(|h| h.iter().copied())
            .into_iter()
            .flatten(),
        );
        let (temp_avg, temp_max, temp_min) = StatsCalculator::compute_i32_aggregates(
          self
            .gpu_temperature_histories
            .get(gpu_id)
            .map(|h| h.iter().copied())
            .into_iter()
            .flatten(),
        );
        let (mem_avg_f32, mem_max, mem_min) = StatsCalculator::compute_i32_aggregates(
          self
            .gpu_dedicated_memory_histories
            .get(gpu_id)
            .map(|h| h.iter().copied())
            .into_iter()
            .flatten(),
        );
        let gpu_name = self
          .gpu_name_map
          .get(gpu_id)
          .cloned()
          .unwrap_or_else(|| gpu_id.clone());
        GpuData {
          gpu_id: Some(gpu_id.clone()),
          gpu_name,
          usage_avg,
          usage_max,
          usage_min,
          temperature_avg: temp_avg,
          temperature_max: temp_max,
          temperature_min: temp_min,
          dedicated_memory_avg: mem_avg_f32.map(|v| v as i32),
          dedicated_memory_max: mem_max,
          dedicated_memory_min: mem_min,
        }
      })
      .collect()
  }

  fn collect_process_stats(&self) -> Vec<ProcessStatData> {
    let core_divisor = self.cores.max(1.0);
    let all_stats: Vec<ProcessStatData> = self
      .processes
      .iter()
      .filter_map(|(pid, acc)| {
        if acc.cpu_history.is_empty() || acc.memory_kb_history.is_empty() {
          return None;
        }
        let cpu_avg = acc.cpu_history.iter().sum::<f32>() / acc.cpu_history.len() as f32;
        let mem_avg =
          acc.memory_kb_history.iter().sum::<f32>() / acc.memory_kb_history.len() as f32;
        if cpu_avg == 0.0 && mem_avg == 0.0 {
          return None;
        }
        let exec_time = acc.run_time_secs as i32;
        if !is_valid_execution_time(exec_time) {
          return None;
        }
        Some(ProcessStatData {
          pid: *pid as i32,
          process_name: acc.name.clone(),
          cpu_usage: cpu_avg / core_divisor,
          memory_usage: mem_avg.round() as i32,
          execution_sec: exec_time,
        })
      })
      .collect();

    rank_top_processes(all_stats)
  }
}

fn is_valid_execution_time(exec_time: i32) -> bool {
  (0..=60 * 60 * 24 * 30).contains(&exec_time)
}

fn rank_top_processes(all_stats: Vec<ProcessStatData>) -> Vec<ProcessStatData> {
  let mut result = Vec::new();
  let mut seen = HashSet::new();
  for &metric in &ProcessRankingMetric::ALL {
    let sorted = sort_by_metric(all_stats.clone(), metric);
    for stat in sorted.into_iter().take(PROCESS_RECORD_LIMIT) {
      if seen.insert(stat.pid) {
        result.push(stat);
      }
    }
    if result.len() >= PROCESS_RECORD_LIMIT * ProcessRankingMetric::ALL.len() {
      break;
    }
  }
  result
}

fn sort_by_metric(
  mut stats: Vec<ProcessStatData>,
  metric: ProcessRankingMetric,
) -> Vec<ProcessStatData> {
  match metric {
    ProcessRankingMetric::Cpu => {
      stats.sort_by(|a, b| b.cpu_usage.total_cmp(&a.cpu_usage))
    }
    ProcessRankingMetric::Memory => {
      stats.sort_by_key(|s| std::cmp::Reverse(s.memory_usage))
    }
    ProcessRankingMetric::ExecutionTime => {
      stats.sort_by_key(|s| std::cmp::Reverse(s.execution_sec))
    }
  }
  stats
}

fn push_capped(buf: &mut VecDeque<f32>, value: f32) {
  if buf.len() >= PROCESS_HISTORY_BUFFER {
    buf.pop_front();
  }
  buf.push_back(value);
}

fn push_capped_i32(buf: &mut VecDeque<i32>, value: i32) {
  if buf.len() >= PROCESS_HISTORY_BUFFER {
    buf.pop_front();
  }
  buf.push_back(value);
}

// ── Stats helpers ─────────────────────────────────────────────────────

struct StatsCalculator;

impl StatsCalculator {
  fn compute_stats(values: impl IntoIterator<Item = f32>) -> HardwareData {
    let values: Vec<f32> = values.into_iter().collect();
    if values.is_empty() {
      return HardwareData {
        avg: None,
        max: None,
        min: None,
      };
    }
    let avg = Some(values.iter().sum::<f32>() / values.len() as f32);
    let max = values.iter().copied().max_by(f32::total_cmp);
    let min = values.iter().copied().min_by(f32::total_cmp);
    HardwareData { avg, max, min }
  }

  fn compute_f32_aggregates(
    values: impl IntoIterator<Item = f32>,
  ) -> (Option<f32>, Option<f32>, Option<f32>) {
    let values: Vec<f32> = values.into_iter().collect();
    if values.is_empty() {
      return (None, None, None);
    }
    let avg = Some(values.iter().sum::<f32>() / values.len() as f32);
    let max = values.iter().copied().max_by(f32::total_cmp);
    let min = values.iter().copied().min_by(f32::total_cmp);
    (avg, max, min)
  }

  fn compute_i32_aggregates(
    values: impl IntoIterator<Item = i32>,
  ) -> (Option<f32>, Option<i32>, Option<i32>) {
    let values: Vec<i32> = values.into_iter().collect();
    if values.is_empty() {
      return (None, None, None);
    }
    // Sum in i64: dedicated GPU memory is reported in KB, so 60 samples
    // from a high-VRAM card (e.g. 80 GB ≈ 8.4×10⁷ KB) easily exceed
    // i32::MAX. An i32 accumulator panics in debug and silently wraps
    // in release.
    let sum: i64 = values.iter().map(|&v| v as i64).sum();
    let avg = Some(sum as f32 / values.len() as f32);
    let max = values.iter().copied().max();
    let min = values.iter().copied().min();
    (avg, max, min)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::GpuMetric;

  fn make_snapshot(cpu: f32, memory: f32, processors: usize) -> MetricsSnapshot {
    MetricsSnapshot {
      cpu_usage: cpu,
      memory_usage: memory,
      processors_usage: vec![0.0; processors],
      gpus: vec![],
      processes: vec![],
    }
  }

  fn make_process_stat(pid: i32, cpu: f32, mem: i32, exec: i32) -> ProcessStatData {
    ProcessStatData {
      pid,
      process_name: format!("p{pid}"),
      cpu_usage: cpu,
      memory_usage: mem,
      execution_sec: exec,
    }
  }

  // ── StatsCalculator ──

  #[test]
  fn compute_stats_empty_slice_returns_all_none() {
    let result = StatsCalculator::compute_stats(std::iter::empty::<f32>());
    assert!(result.avg.is_none());
    assert!(result.max.is_none());
    assert!(result.min.is_none());
  }

  #[test]
  fn compute_stats_typical() {
    let result = StatsCalculator::compute_stats([10.0_f32, 20.0, 30.0]);
    assert_eq!(result.avg, Some(20.0));
    assert_eq!(result.max, Some(30.0));
    assert_eq!(result.min, Some(10.0));
  }

  #[test]
  fn compute_f32_aggregates_typical() {
    assert_eq!(
      StatsCalculator::compute_f32_aggregates([1.0_f32, 2.0, 3.0, 4.0, 5.0]),
      (Some(3.0), Some(5.0), Some(1.0))
    );
  }

  #[test]
  fn compute_i32_aggregates_typical() {
    assert_eq!(
      StatsCalculator::compute_i32_aggregates([10, 20, 30]),
      (Some(20.0), Some(30), Some(10))
    );
  }

  // ── push_capped ──

  #[test]
  fn push_capped_evicts_oldest_at_buffer_limit() {
    let mut buf: VecDeque<f32> = (0..PROCESS_HISTORY_BUFFER).map(|i| i as f32).collect();
    push_capped(&mut buf, 999.0);
    assert_eq!(buf.len(), PROCESS_HISTORY_BUFFER);
    assert_eq!(*buf.front().unwrap(), 1.0);
    assert_eq!(*buf.back().unwrap(), 999.0);
  }

  // ── ArchiveTracker::ingest ──

  #[test]
  fn ingest_appends_cpu_and_memory() {
    let mut t = ArchiveTracker::new();
    t.ingest(make_snapshot(10.0, 50.0, 4));
    t.ingest(make_snapshot(20.0, 60.0, 4));
    assert_eq!(
      t.cpu_history.iter().copied().collect::<Vec<_>>(),
      vec![10.0, 20.0]
    );
    assert_eq!(
      t.memory_history.iter().copied().collect::<Vec<_>>(),
      vec![50.0, 60.0]
    );
    assert_eq!(t.cores, 4.0);
  }

  #[test]
  fn ingest_caps_buffers_at_history_limit() {
    let mut t = ArchiveTracker::new();
    for i in 0..PROCESS_HISTORY_BUFFER + 5 {
      t.ingest(make_snapshot(i as f32, 0.0, 1));
    }
    assert_eq!(t.cpu_history.len(), PROCESS_HISTORY_BUFFER);
    assert_eq!(*t.cpu_history.front().unwrap(), 5.0);
  }

  #[test]
  fn ingest_records_gpu_name_and_metrics() {
    let mut t = ArchiveTracker::new();
    t.ingest(MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![0.0],
      gpus: vec![GpuMetric {
        gpu_id: "gpu:0".into(),
        gpu_name: "RTX".into(),
        gpu_usage: Some(50.0),
        gpu_temperature: Some(70.0),
        gpu_source: "Test".into(),
        gpu_dedicated_memory_usage_kb: Some(2048.0),
        gpu_cooler_level: None,
      }],
      processes: vec![],
    });
    assert_eq!(t.gpu_name_map.get("gpu:0").unwrap(), "RTX");
    assert_eq!(t.gpu_usage_histories.get("gpu:0").unwrap().len(), 1);
    assert_eq!(t.gpu_temperature_histories.get("gpu:0").unwrap().len(), 1);
    assert_eq!(
      t.gpu_dedicated_memory_histories.get("gpu:0").unwrap().len(),
      1
    );
  }

  fn snap_with_pid(pid: u32, name: &str) -> MetricsSnapshot {
    MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![0.0],
      gpus: vec![],
      processes: vec![ProcessSample {
        pid,
        name: name.into(),
        cpu_usage: 10.0,
        memory_kb: 1024.0,
        run_time_secs: 60,
      }],
    }
  }

  #[test]
  fn ingest_keeps_recently_vanished_processes_within_grace_window() {
    let mut t = ArchiveTracker::new();
    t.ingest(snap_with_pid(100, "short_lived"));
    // Drive a few snapshots without PID 100 (well under the grace window).
    for _ in 0..5 {
      t.ingest(snap_with_pid(200, "alive"));
    }
    assert!(
      t.processes.contains_key(&100),
      "PID 100 must persist long enough to be archived in the next tick"
    );
    assert!(t.processes.contains_key(&200));
  }

  #[test]
  fn ingest_evicts_processes_absent_past_grace_window() {
    let mut t = ArchiveTracker::new();
    t.ingest(snap_with_pid(100, "short_lived"));
    // Push past the grace window with snapshots that don't include PID 100.
    for _ in 0..PROCESS_GRACE_TICKS + 1 {
      t.ingest(snap_with_pid(200, "alive"));
    }
    assert!(!t.processes.contains_key(&100));
    assert!(t.processes.contains_key(&200));
  }

  #[test]
  fn ingest_resets_rings_on_pid_reuse_after_gap() {
    let mut t = ArchiveTracker::new();
    t.ingest(snap_with_pid(42, "victim"));
    // Drive several ticks without PID 42 — it stays in the map (still
    // within grace) but we don't push to its rings.
    for _ in 0..3 {
      t.ingest(snap_with_pid(99, "filler"));
    }
    // PID 42 reappears, presumably as a different process with the
    // same recycled PID. We should not blend its old rings with the
    // new samples.
    t.ingest(snap_with_pid(42, "fresh"));
    let acc = t.processes.get(&42).unwrap();
    assert_eq!(acc.cpu_history.len(), 1);
    assert_eq!(acc.memory_kb_history.len(), 1);
    assert_eq!(acc.name, "fresh");
  }

  // ── collect_process_stats / rank_top_processes ──

  #[test]
  fn rank_top_processes_dedupes_across_metrics() {
    let stats = vec![
      make_process_stat(1, 99.0, 999, 9999),
      make_process_stat(2, 1.0, 1, 1),
    ];
    let ranked = rank_top_processes(stats);
    let count_1 = ranked.iter().filter(|s| s.pid == 1).count();
    assert_eq!(count_1, 1);
  }

  #[test]
  fn rank_top_processes_caps_at_limit_times_metrics() {
    let stats: Vec<_> = (0..30)
      .map(|i| make_process_stat(i, i as f32, i * 10, i * 60))
      .collect();
    let ranked = rank_top_processes(stats);
    assert!(ranked.len() <= PROCESS_RECORD_LIMIT * ProcessRankingMetric::ALL.len());
  }

  #[test]
  fn collect_process_stats_skips_zero_only_processes() {
    let mut t = ArchiveTracker::new();
    t.ingest(MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![0.0],
      gpus: vec![],
      processes: vec![ProcessSample {
        pid: 1,
        name: "idle".into(),
        cpu_usage: 0.0,
        memory_kb: 0.0,
        run_time_secs: 60,
      }],
    });
    assert!(t.collect_process_stats().is_empty());
  }

  #[test]
  fn collect_process_stats_normalizes_cpu_by_cores() {
    let mut t = ArchiveTracker::new();
    t.ingest(MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![0.0; 8],
      gpus: vec![],
      processes: vec![ProcessSample {
        pid: 1,
        name: "p".into(),
        cpu_usage: 80.0,
        memory_kb: 1024.0,
        run_time_secs: 60,
      }],
    });
    let stats = t.collect_process_stats();
    assert_eq!(stats.len(), 1);
    assert!((stats[0].cpu_usage - 10.0).abs() < f32::EPSILON);
  }

  #[test]
  fn collect_process_stats_drops_invalid_run_time() {
    let mut t = ArchiveTracker::new();
    t.ingest(MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![0.0],
      gpus: vec![],
      processes: vec![ProcessSample {
        pid: 1,
        name: "p".into(),
        cpu_usage: 50.0,
        memory_kb: 1024.0,
        // exceed 30-day cap
        run_time_secs: (60 * 60 * 24 * 31) as u64,
      }],
    });
    assert!(t.collect_process_stats().is_empty());
  }

  // ── collect_gpu_data ──

  #[test]
  fn collect_gpu_data_uses_name_from_map() {
    let mut t = ArchiveTracker::new();
    t.ingest(MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![0.0],
      gpus: vec![GpuMetric {
        gpu_id: "gpu:0".into(),
        gpu_name: "RTX 4090".into(),
        gpu_usage: Some(50.0),
        gpu_temperature: Some(70.0),
        gpu_source: "Test".into(),
        gpu_dedicated_memory_usage_kb: Some(4096.0),
        gpu_cooler_level: None,
      }],
      processes: vec![],
    });
    let gpus = t.collect_gpu_data();
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].gpu_id.as_deref(), Some("gpu:0"));
    assert_eq!(gpus[0].gpu_name, "RTX 4090");
    assert_eq!(gpus[0].usage_avg, Some(50.0));
    assert_eq!(gpus[0].temperature_max, Some(70));
    assert_eq!(gpus[0].dedicated_memory_min, Some(4096));
  }

  #[test]
  fn collect_gpu_data_falls_back_to_id_when_name_missing() {
    // Direct construction skips the ingest path so we can simulate a
    // GPU id that has metrics but no recorded name.
    let mut t = ArchiveTracker::new();
    t.gpu_usage_histories
      .entry("anon".to_string())
      .or_default()
      .push_back(50.0);
    let gpus = t.collect_gpu_data();
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].gpu_name, "anon");
  }
}
