//! Per-sensor history rings + the Core read API used by App-side commands.
//!
//! `HistoryStore` replaces `crate::models::hardware::HardwareMonitorState`
//! (which lived in `src-tauri` before Phase 3). It owns:
//!
//! - the shared `sysinfo::System` instance refreshed each tick,
//! - 60-second CPU / memory ring buffers,
//! - per-process CPU and memory history maps,
//! - per-GPU usage / temperature / dedicated-memory ring buffers,
//! - a `gpu_id → gpu_name` map populated during sampling.
//!
//! All fields are kept behind `Arc<Mutex<...>>` so the collector loop and
//! the (still-Tauri-resident) archive worker can share state until Phase 4
//! replaces `MonitorResources` with an EventBus subscription.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use sysinfo::{ProcessesToUpdate, System};

use crate::models::hardware::ProcessInfo;
use crate::utils::rounding;

pub const HARDWARE_HISTORY_BUFFER_SIZE: usize = 60;
pub const MAX_HISTORY_QUERY_DURATION_SECONDS: u32 = 3600;
const PROCESS_AVG_WINDOW: usize = 5;

/// All-the-`Arc`s shared between the collector and (during Phase 3) the
/// archive worker. `lib.rs` uses this to construct the legacy
/// `MonitorResources` struct without re-allocating any state. Phase 4
/// will delete this struct in favor of an EventBus-driven archive.
#[derive(Clone)]
pub struct HistoryArcHandles {
  pub system: Arc<Mutex<System>>,
  pub cpu_history: Arc<Mutex<VecDeque<f32>>>,
  pub memory_history: Arc<Mutex<VecDeque<f32>>>,
  pub process_cpu_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  pub process_memory_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  pub gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  pub gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  pub gpu_dedicated_memory_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  pub gpu_name_map: Arc<Mutex<HashMap<String, String>>>,
}

/// In-process owner of every sensor history series.
///
/// Cheap to clone (`Arc`-wrapped internally). All readers and writers go
/// through the inherent methods so the locking discipline stays in this
/// module.
#[derive(Clone)]
pub struct HistoryStore {
  inner: Arc<HistoryArcHandles>,
}

impl HistoryStore {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(HistoryArcHandles {
        system: Arc::new(Mutex::new(System::new_all())),
        cpu_history: Arc::new(Mutex::new(VecDeque::with_capacity(
          HARDWARE_HISTORY_BUFFER_SIZE,
        ))),
        memory_history: Arc::new(Mutex::new(VecDeque::with_capacity(
          HARDWARE_HISTORY_BUFFER_SIZE,
        ))),
        process_cpu_histories: Arc::new(Mutex::new(HashMap::new())),
        process_memory_histories: Arc::new(Mutex::new(HashMap::new())),
        gpu_usage_histories: Arc::new(Mutex::new(HashMap::new())),
        gpu_temperature_histories: Arc::new(Mutex::new(HashMap::new())),
        gpu_dedicated_memory_histories: Arc::new(Mutex::new(HashMap::new())),
        gpu_name_map: Arc::new(Mutex::new(HashMap::new())),
      }),
    }
  }

  /// Phase-3 transitional accessor: hand the underlying `Arc<Mutex<...>>`
  /// values to the App-side archive worker so it can keep populating the
  /// existing `MonitorResources` struct. Phase 4 deletes this method.
  pub fn arc_handles(&self) -> HistoryArcHandles {
    HistoryArcHandles {
      system: Arc::clone(&self.inner.system),
      cpu_history: Arc::clone(&self.inner.cpu_history),
      memory_history: Arc::clone(&self.inner.memory_history),
      process_cpu_histories: Arc::clone(&self.inner.process_cpu_histories),
      process_memory_histories: Arc::clone(&self.inner.process_memory_histories),
      gpu_usage_histories: Arc::clone(&self.inner.gpu_usage_histories),
      gpu_temperature_histories: Arc::clone(&self.inner.gpu_temperature_histories),
      gpu_dedicated_memory_histories: Arc::clone(
        &self.inner.gpu_dedicated_memory_histories,
      ),
      gpu_name_map: Arc::clone(&self.inner.gpu_name_map),
    }
  }

  // ── Internal accessors used by `crate::collector::sampling` ──

  /// Borrow the shared `sysinfo::System` Arc. Used by App-side services
  /// that compose hardware info from sysinfo state. The collector loop
  /// also uses this to refresh the system on each tick.
  pub fn system(&self) -> &Arc<Mutex<System>> {
    &self.inner.system
  }

  pub(crate) fn push_cpu_history(&self, value: f32) {
    push_history(&self.inner.cpu_history, value);
  }

  pub(crate) fn push_memory_history(&self, value: f32) {
    push_history(&self.inner.memory_history, value);
  }

  pub(crate) fn update_process_histories(
    &self,
    process_metrics: &[(sysinfo::Pid, f32, f32)],
  ) {
    // Drop history for PIDs that no longer exist so the maps don't grow
    // for the lifetime of the app, and so a recycled PID can't inherit a
    // previous process's rolling samples.
    let active_pids: std::collections::HashSet<sysinfo::Pid> =
      process_metrics.iter().map(|(pid, _, _)| *pid).collect();

    let mut cpu_histories = self.inner.process_cpu_histories.lock().unwrap();
    let mut mem_histories = self.inner.process_memory_histories.lock().unwrap();

    cpu_histories.retain(|pid, _| active_pids.contains(pid));
    mem_histories.retain(|pid, _| active_pids.contains(pid));

    for (pid, cpu_usage, memory_mb) in process_metrics {
      let cpu_history = cpu_histories.entry(*pid).or_default();
      if cpu_history.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
        cpu_history.pop_front();
      }
      cpu_history.push_back(*cpu_usage);

      let mem_history = mem_histories.entry(*pid).or_default();
      if mem_history.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
        mem_history.pop_front();
      }
      mem_history.push_back(*memory_mb);
    }
  }

  pub(crate) fn update_gpu_histories(
    &self,
    samples: &[crate::collector::sampling::GpuSample],
  ) {
    let mut usage_histories = self.inner.gpu_usage_histories.lock().unwrap();
    let mut temp_histories = self.inner.gpu_temperature_histories.lock().unwrap();
    let mut mem_histories = self.inner.gpu_dedicated_memory_histories.lock().unwrap();
    let mut name_map = self.inner.gpu_name_map.lock().unwrap();

    for sample in samples {
      name_map.insert(sample.gpu_id.clone(), sample.name.clone());

      if let Some(usage) = sample.usage {
        let h = usage_histories.entry(sample.gpu_id.clone()).or_default();
        if h.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
          h.pop_front();
        }
        h.push_back(usage);
      }
      if let Some(temperature) = sample.temperature {
        let h = temp_histories.entry(sample.gpu_id.clone()).or_default();
        if h.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
          h.pop_front();
        }
        h.push_back(temperature as i32);
      }
      if let Some(memory_usage) = sample.dedicated_memory_kb {
        let h = mem_histories.entry(sample.gpu_id.clone()).or_default();
        if h.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
          h.pop_front();
        }
        h.push_back(memory_usage as i32);
      }
    }
  }

  // ── Core read API used by App-side Tauri command handlers ──

  /// Last `seconds` of CPU usage (newest first; clamped to
  /// [`MAX_HISTORY_QUERY_DURATION_SECONDS`]).
  pub fn cpu_history(&self, seconds: u32) -> Vec<f32> {
    let history = self.inner.cpu_history.lock().unwrap();
    let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;
    history.iter().rev().take(take_n).cloned().collect()
  }

  /// Last `seconds` of memory usage (newest first).
  pub fn memory_history(&self, seconds: u32) -> Vec<f32> {
    let history = self.inner.memory_history.lock().unwrap();
    let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;
    history.iter().rev().take(take_n).cloned().collect()
  }

  /// Last `seconds` of GPU usage for the GPU identified by `gpu_id`
  /// (newest first). Returns an empty vector if the GPU is unknown.
  pub fn gpu_history(&self, gpu_id: &str, seconds: u32) -> Vec<f32> {
    let histories = self.inner.gpu_usage_histories.lock().unwrap();
    let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;
    histories
      .get(gpu_id)
      .map(|h| h.iter().rev().take(take_n).cloned().collect())
      .unwrap_or_default()
  }

  /// Process list refreshed via sysinfo, with rolling-average CPU and
  /// memory usage (last [`PROCESS_AVG_WINDOW`] samples) per process.
  pub fn process_list(&self) -> Vec<ProcessInfo> {
    let mut system = self.inner.system.lock().unwrap();
    let process_cpu_histories = self.inner.process_cpu_histories.lock().unwrap();
    let process_memory_histories = self.inner.process_memory_histories.lock().unwrap();

    system.refresh_processes(ProcessesToUpdate::All, true);
    let num_cores = system.cpus().len() as f32;

    system
      .processes()
      .values()
      .map(|process| {
        let pid = process.pid();

        // For freshly started processes (no history yet), fall through
        // to the just-refreshed sysinfo sample instead of pretending the
        // CPU usage is 0.0. Mirrors the memory branch below.
        let core_divisor = num_cores.max(1.0);
        let fresh_cpu = || rounding::round1(process.cpu_usage() / core_divisor);
        let cpu_usage = process_cpu_histories
          .get(&pid)
          .map(|hist| {
            let len = hist.len().min(PROCESS_AVG_WINDOW);
            if len == 0 {
              return fresh_cpu();
            }
            let sum: f32 = hist.iter().rev().take(len).sum();
            let avg = sum / len as f32;
            let normalized = avg / num_cores;
            rounding::round1(normalized)
          })
          .unwrap_or_else(fresh_cpu);

        // The frontend's `ProcessTable` displays this value as MB.
        // Stored samples are KB (`process.memory()` is bytes; `sample_system`
        // pushes `bytes / 1024` into the history). Both the averaged-history
        // and the fallback paths must produce MB, so divide by 1024 once
        // more here. Previously the fallback was off by 1024× the first
        // tick of any new process.
        let memory_usage_mb = process_memory_histories
          .get(&pid)
          .map(|hist| {
            let len = hist.len().min(PROCESS_AVG_WINDOW);
            if len == 0 {
              return (process.memory() as f32 / 1024.0) / 1024.0;
            }
            let sum_kb: f32 = hist.iter().rev().take(len).sum();
            let avg_kb = sum_kb / len as f32;
            rounding::round1(avg_kb / 1024.0)
          })
          .unwrap_or_else(|| (process.memory() as f32 / 1024.0) / 1024.0);

        ProcessInfo {
          pid: pid.as_u32() as i32,
          name: process.name().to_string_lossy().into_owned(),
          cpu_usage,
          memory_usage: memory_usage_mb,
        }
      })
      .collect()
  }

  /// Overall CPU usage (%) as a rounded integer based on the most recent
  /// sysinfo sample. Returns 0 when no CPU is reported.
  pub fn current_cpu_usage_overall(&self) -> i32 {
    let system = self.inner.system.lock().unwrap();
    let cpus = system.cpus();
    if cpus.is_empty() {
      return 0;
    }
    let total: f32 = cpus.iter().map(|c| c.cpu_usage()).sum();
    (total / cpus.len() as f32).round() as i32
  }

  /// Per-processor CPU usage (%) from the most recent sysinfo sample,
  /// without rounding.
  pub fn current_cpu_usage_per_processor(&self) -> Vec<f32> {
    let system = self.inner.system.lock().unwrap();
    system.cpus().iter().map(|c| c.cpu_usage()).collect()
  }

  /// Memory usage (%) from the most recent sysinfo sample, rounded.
  pub fn current_memory_usage_percent(&self) -> i32 {
    let system = self.inner.system.lock().unwrap();
    let used = system.used_memory() as f64;
    let total = system.total_memory() as f64;
    if total == 0.0 {
      0
    } else {
      ((used / total) * 100.0).round() as i32
    }
  }
}

impl Default for HistoryStore {
  fn default() -> Self {
    Self::new()
  }
}

fn push_history(history: &Arc<Mutex<VecDeque<f32>>>, value: f32) {
  let mut h = history.lock().unwrap();
  if h.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
    h.pop_front();
  }
  h.push_back(value);
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::collector::sampling::GpuSample;

  fn store_with(cpu: Vec<f32>, mem: Vec<f32>) -> HistoryStore {
    let s = HistoryStore::new();
    {
      let mut h = s.inner.cpu_history.lock().unwrap();
      *h = VecDeque::from(cpu);
    }
    {
      let mut h = s.inner.memory_history.lock().unwrap();
      *h = VecDeque::from(mem);
    }
    s
  }

  fn store_with_gpu(gpu_id: &str, gpu_data: Vec<f32>) -> HistoryStore {
    let s = HistoryStore::new();
    s.inner
      .gpu_usage_histories
      .lock()
      .unwrap()
      .insert(gpu_id.to_string(), VecDeque::from(gpu_data));
    s
  }

  fn make_gpu_sample(
    gpu_id: &str,
    name: &str,
    usage: Option<f32>,
    temperature: Option<f32>,
    dedicated_memory_kb: Option<f32>,
  ) -> GpuSample {
    GpuSample {
      gpu_id: gpu_id.to_string(),
      name: name.to_string(),
      usage,
      temperature,
      dedicated_memory_kb,
      cooler_level: None,
      source: "Test".to_string(),
    }
  }

  // ── push_history helper ──

  #[test]
  fn push_history_to_empty() {
    let h = Arc::new(Mutex::new(VecDeque::new()));
    push_history(&h, 1.0);
    let g = h.lock().unwrap();
    assert_eq!(g.len(), 1);
    assert_eq!(g[0], 1.0);
  }

  #[test]
  fn push_history_at_capacity_evicts_oldest() {
    let initial: VecDeque<f32> = (0..HARDWARE_HISTORY_BUFFER_SIZE)
      .map(|i| i as f32)
      .collect();
    let h = Arc::new(Mutex::new(initial));
    push_history(&h, 999.0);

    let g = h.lock().unwrap();
    assert_eq!(g.len(), HARDWARE_HISTORY_BUFFER_SIZE);
    assert_eq!(*g.front().unwrap(), 1.0);
    assert_eq!(*g.back().unwrap(), 999.0);
  }

  #[test]
  fn push_history_preserves_order() {
    let h = Arc::new(Mutex::new(VecDeque::new()));
    push_history(&h, 1.0);
    push_history(&h, 2.0);
    push_history(&h, 3.0);
    let g = h.lock().unwrap();
    assert_eq!(g.iter().cloned().collect::<Vec<_>>(), vec![1.0, 2.0, 3.0]);
  }

  // ── cpu_history / memory_history ──

  #[test]
  fn cpu_history_returns_last_n_reversed() {
    let s = store_with(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![]);
    assert_eq!(s.cpu_history(3), vec![5.0, 4.0, 3.0]);
  }

  #[test]
  fn cpu_history_clamped_to_available() {
    let s = store_with(vec![1.0, 2.0], vec![]);
    assert_eq!(s.cpu_history(10), vec![2.0, 1.0]);
  }

  #[test]
  fn cpu_history_clamped_to_max_duration() {
    let data: Vec<f32> = (0..5000).map(|i| i as f32).collect();
    let s = store_with(data, vec![]);
    let result = s.cpu_history(u32::MAX);
    assert_eq!(result.len(), MAX_HISTORY_QUERY_DURATION_SECONDS as usize);
  }

  #[test]
  fn cpu_history_empty() {
    let s = store_with(vec![], vec![]);
    assert!(s.cpu_history(5).is_empty());
  }

  #[test]
  fn memory_history_returns_last_n_reversed() {
    let s = store_with(vec![], vec![10.0, 20.0, 30.0]);
    assert_eq!(s.memory_history(2), vec![30.0, 20.0]);
  }

  // ── gpu_history ──

  #[test]
  fn gpu_history_returns_last_n_reversed() {
    let s = store_with_gpu("gpu:0", vec![40.0, 50.0, 60.0, 70.0]);
    assert_eq!(s.gpu_history("gpu:0", 3), vec![70.0, 60.0, 50.0]);
  }

  #[test]
  fn gpu_history_unknown_id_returns_empty() {
    let s = store_with_gpu("gpu:0", vec![10.0, 20.0]);
    assert!(s.gpu_history("gpu:nonexistent", 5).is_empty());
  }

  #[test]
  fn gpu_history_per_id() {
    let s = HistoryStore::new();
    s.inner
      .gpu_usage_histories
      .lock()
      .unwrap()
      .insert("gpu:0".to_string(), VecDeque::from(vec![10.0, 20.0]));
    s.inner
      .gpu_usage_histories
      .lock()
      .unwrap()
      .insert("gpu:1".to_string(), VecDeque::from(vec![90.0, 80.0]));
    assert_eq!(s.gpu_history("gpu:0", 5), vec![20.0, 10.0]);
    assert_eq!(s.gpu_history("gpu:1", 5), vec![80.0, 90.0]);
  }

  // ── update_gpu_histories ──

  #[test]
  fn update_gpu_histories_adds_new_gpu() {
    let s = HistoryStore::new();
    let samples = vec![make_gpu_sample(
      "gpu:0",
      "TestGPU",
      Some(50.0),
      Some(70.0),
      Some(1024.0),
    )];
    s.update_gpu_histories(&samples);

    let usage = s.inner.gpu_usage_histories.lock().unwrap();
    assert_eq!(*usage.get("gpu:0").unwrap().back().unwrap(), 50.0);
    let temp = s.inner.gpu_temperature_histories.lock().unwrap();
    assert_eq!(*temp.get("gpu:0").unwrap().back().unwrap(), 70);
    let mem = s.inner.gpu_dedicated_memory_histories.lock().unwrap();
    assert_eq!(*mem.get("gpu:0").unwrap().back().unwrap(), 1024);
  }

  #[test]
  fn update_gpu_histories_skips_none() {
    let s = HistoryStore::new();
    s.update_gpu_histories(&[make_gpu_sample("gpu:0", "X", None, None, None)]);
    assert!(s.inner.gpu_usage_histories.lock().unwrap().is_empty());
    assert!(s.inner.gpu_temperature_histories.lock().unwrap().is_empty());
    assert!(
      s.inner
        .gpu_dedicated_memory_histories
        .lock()
        .unwrap()
        .is_empty()
    );
  }

  #[test]
  fn update_gpu_histories_populates_name_map() {
    let s = HistoryStore::new();
    s.update_gpu_histories(&[
      make_gpu_sample("pci:0:2.0", "RTX 4090", Some(50.0), None, None),
      make_gpu_sample("pci:0:3.0", "RX 7900 XTX", Some(80.0), None, None),
    ]);

    let name_map = s.inner.gpu_name_map.lock().unwrap();
    assert_eq!(name_map.get("pci:0:2.0").unwrap(), "RTX 4090");
    assert_eq!(name_map.get("pci:0:3.0").unwrap(), "RX 7900 XTX");
  }

  #[test]
  fn update_gpu_histories_respects_buffer_cap() {
    let s = HistoryStore::new();
    for i in 0..HARDWARE_HISTORY_BUFFER_SIZE {
      s.update_gpu_histories(&[make_gpu_sample(
        "gpu:0",
        "GPU",
        Some(i as f32),
        None,
        None,
      )]);
    }
    s.update_gpu_histories(&[make_gpu_sample("gpu:0", "GPU", Some(999.0), None, None)]);

    let usage = s.inner.gpu_usage_histories.lock().unwrap();
    let h = usage.get("gpu:0").unwrap();
    assert_eq!(h.len(), HARDWARE_HISTORY_BUFFER_SIZE);
    assert_eq!(*h.back().unwrap(), 999.0);
    assert_eq!(*h.front().unwrap(), 1.0);
  }

  // ── update_process_histories ──

  #[test]
  fn update_process_histories_new_process() {
    let s = HistoryStore::new();
    let pid = sysinfo::Pid::from_u32(100);
    s.update_process_histories(&[(pid, 50.0, 256.0)]);

    let cpu = s.inner.process_cpu_histories.lock().unwrap();
    assert_eq!(*cpu.get(&pid).unwrap().back().unwrap(), 50.0);
    let mem = s.inner.process_memory_histories.lock().unwrap();
    assert_eq!(*mem.get(&pid).unwrap().back().unwrap(), 256.0);
  }

  #[test]
  fn update_process_histories_respects_buffer_cap() {
    let s = HistoryStore::new();
    let pid = sysinfo::Pid::from_u32(300);
    for i in 0..HARDWARE_HISTORY_BUFFER_SIZE + 5 {
      s.update_process_histories(&[(pid, i as f32, i as f32)]);
    }
    let cpu = s.inner.process_cpu_histories.lock().unwrap();
    assert_eq!(cpu.get(&pid).unwrap().len(), HARDWARE_HISTORY_BUFFER_SIZE);
  }
}
