use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::constants::{
  HARDWARE_HISTORY_BUFFER_SIZE, MAX_HISTORY_QUERY_DURATION_SECONDS,
};
use crate::models::hardware::HardwareMonitorState;
use crate::models::hardware_archive::MonitorResources;

/// A single GPU sample: (name, usage%, temperature°C, dedicated_memory_usage%).
/// `None` means the metric is unavailable for this GPU vendor/platform.
type GpuSample = (String, Option<f32>, Option<f32>, Option<f32>);

/// System sampling for one cycle (CPU/memory/process)
pub fn sample_system(resources: &MonitorResources) {
  if let Some((cpu_usage, memory_usage, process_metrics)) =
    resources.system.lock().ok().map(|mut sys| {
      sys.refresh_all();

      let cpu_usage = calculate_average_cpu_usage(sys.cpus());
      let memory_usage =
        calculate_memory_usage_percentage(sys.used_memory(), sys.total_memory());

      let process_metrics: Vec<_> = sys
        .processes()
        .iter()
        .map(|(pid, process)| {
          (*pid, process.cpu_usage(), process.memory() as f32 / 1024.0)
        })
        .collect();

      (cpu_usage, memory_usage, process_metrics)
    })
  {
    push_history(&resources.cpu_history, cpu_usage);
    push_history(&resources.memory_history, memory_usage);
    update_process_histories(resources, &process_metrics);
  }
}

fn calculate_average_cpu_usage(cpus: &[sysinfo::Cpu]) -> f32 {
  match cpus.len() {
    0 => 0.0,
    len => (cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / len as f32).round(),
  }
}

fn calculate_memory_usage_percentage(used: u64, total: u64) -> f32 {
  match total {
    0 => 0.0,
    total => ((used as f64 / total as f64) * 100.0).round() as f32,
  }
}

fn update_process_histories(
  resources: &MonitorResources,
  process_metrics: &[(sysinfo::Pid, f32, f32)],
) {
  let mut cpu_histories = resources.process_cpu_histories.lock().unwrap();
  let mut mem_histories = resources.process_memory_histories.lock().unwrap();

  process_metrics
    .iter()
    .for_each(|(pid, cpu_usage, memory_mb)| {
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
    });
}

#[cfg(target_os = "windows")]
pub async fn sample_gpu(resources: &MonitorResources) {
  use crate::infrastructure::providers::nvapi_provider;
  use nvapi::PhysicalGpu;

  let mut gpu_metrics: Vec<GpuSample> = Vec::new();

  // ── NVIDIA GPUs via NVAPI ──
  if let Some(nvapi_metrics) = PhysicalGpu::enumerate().ok().map(|gpus| {
    gpus
      .iter()
      .map(|gpu| {
        let name = gpu.full_name().unwrap_or_else(|_| "Unknown".to_string());
        let usage = nvapi_provider::get_gpu_usage_from_physical_gpu(gpu);
        let temperature =
          nvapi_provider::get_gpu_temperature_from_physical_gpu(gpu) as f32;
        let memory_usage =
          nvapi_provider::get_gpu_dedicated_memory_usage_from_physical_gpu(gpu) as f32;
        (name, Some(usage), Some(temperature), Some(memory_usage))
      })
      .collect::<Vec<_>>()
  }) {
    gpu_metrics.extend(nvapi_metrics);
  }

  // ── AMD GPUs via ADL ──
  if crate::infrastructure::providers::adl_provider::is_available() {
    sample_amd_gpu(&mut gpu_metrics).await;
  }

  if !gpu_metrics.is_empty() {
    update_gpu_histories(resources, &gpu_metrics);
  }
}

/// Collect AMD GPU usage and temperature via ADL.
/// VRAM usage is not available via ADL.
#[cfg(target_os = "windows")]
async fn sample_amd_gpu(gpu_metrics: &mut Vec<GpuSample>) {
  use crate::infrastructure::providers::adl_provider;

  // Usage per adapter
  let usages: Vec<(String, f32)> = adl_provider::get_amd_gpu_usage_per_adapter()
    .await
    .unwrap_or_default();

  // Temperature per adapter
  let temps: Vec<(String, f32)> = adl_provider::get_amd_gpu_temperatures_per_adapter()
    .await
    .unwrap_or_default();

  // Build a temp lookup by adapter name
  let temp_map: std::collections::HashMap<&str, f32> =
    temps.iter().map(|(n, t)| (n.as_str(), *t)).collect();

  for (name, usage) in &usages {
    let temperature = temp_map.get(name.as_str()).copied();
    // VRAM usage is not available via ADL
    gpu_metrics.push((name.clone(), Some(*usage), temperature, None));
  }
}

#[cfg(target_os = "linux")]
pub async fn sample_gpu(resources: &MonitorResources) {
  let gpu_metrics = collect_linux_gpu_metrics().await;

  if !gpu_metrics.is_empty() {
    update_gpu_histories(resources, &gpu_metrics);
  }
}

#[cfg(target_os = "macos")]
pub async fn sample_gpu(resources: &MonitorResources) {
  use crate::infrastructure::providers::macos::io_kit::iokit_info;
  use crate::infrastructure::providers::macos::{gpu, gpu_info};

  static CACHED_GPU_NAME: tokio::sync::OnceCell<String> =
    tokio::sync::OnceCell::const_new();

  // Start the IOKit usage sampler thread (first call only)
  let _ = gpu::init_gpu_usage_sampler_thread();

  // Cache GPU name (fetched via IOKit + system_profiler on first call)
  let gpu_name = CACHED_GPU_NAME
    .get_or_init(|| async {
      gpu_info::get_gpu_info()
        .await
        .ok()
        .and_then(|list| list.into_iter().next())
        .map(|info| info.name)
        .unwrap_or_else(|| "Apple GPU".to_string())
    })
    .await;

  // GPU usage (cached IOKit IOReport value, 0.0-1.0 → 0-100%)
  let usage = gpu::read_gpu_usage_cached().map(|v| v * 100.0);

  // Temperature: not available on macOS
  let temperature: Option<f32> = None;

  // Memory usage: convert IOKit bytes → KB (same unit as Windows NVAPI)
  let memory_kb: Option<f32> =
    tauri::async_runtime::spawn_blocking(iokit_info::get_gpu_memory_usage_from_iokit)
      .await
      .ok()
      .flatten()
      .and_then(|mem| mem.in_use_bytes)
      .map(|bytes| (bytes / 1024) as f32);

  let gpu_metrics = vec![(gpu_name.clone(), usage, temperature, memory_kb)];
  update_gpu_histories(resources, &gpu_metrics);
}

/// Collect GPU metrics on Linux using the existing platform layer
/// (DRM/sysfs for AMD, DRM for Intel).
#[cfg(target_os = "linux")]
async fn collect_linux_gpu_metrics() -> Vec<GpuSample> {
  use crate::infrastructure::providers::drm_sys;
  use crate::infrastructure::providers::hwmon;

  let mut metrics: Vec<GpuSample> = Vec::new();
  let card_ids = drm_sys::get_all_card_ids();

  for card_id in card_ids {
    let vendor = drm_sys::detect_gpu_vendor(card_id);

    let (name, usage, temperature) = match vendor {
      drm_sys::GpuVendor::Amd => {
        let name =
          crate::infrastructure::providers::lspci::get_gpu_name_from_lspci_by_vendor_id(
            "1002",
          )
          .unwrap_or_else(|| format!("AMD GPU (card{})", card_id));
        let usage = drm_sys::get_amd_gpu_usage(card_id as u32)
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        let temperature = hwmon::read_hwmon_temperatures(card_id)
          .ok()
          .and_then(|temps| temps.first().map(|t| t.value as f32));
        (name, usage, temperature)
      }
      drm_sys::GpuVendor::Intel => {
        let usage = drm_sys::get_intel_gpu_usage()
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        (format!("Intel GPU (card{})", card_id), usage, None)
      }
      _ => continue,
    };

    // VRAM usage is not available on Linux
    metrics.push((name, usage, temperature, None));
  }

  metrics
}

fn update_gpu_histories(resources: &MonitorResources, gpu_metrics: &[GpuSample]) {
  let mut usage_histories = resources.gpu_usage_histories.lock().unwrap();
  let mut temp_histories = resources.gpu_temperature_histories.lock().unwrap();
  let mut mem_histories = resources.gpu_dedicated_memory_histories.lock().unwrap();

  gpu_metrics
    .iter()
    .for_each(|(name, usage, temperature, memory_usage)| {
      if let Some(usage) = usage {
        let usage_history = usage_histories.entry(name.clone()).or_default();
        if usage_history.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
          usage_history.pop_front();
        }
        usage_history.push_back(*usage);
      }
      if let Some(temperature) = temperature {
        let temp_history = temp_histories.entry(name.clone()).or_default();
        if temp_history.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
          temp_history.pop_front();
        }
        temp_history.push_back(*temperature as i32);
      }
      if let Some(memory_usage) = memory_usage {
        let mem_history = mem_histories.entry(name.clone()).or_default();
        if mem_history.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
          mem_history.pop_front();
        }
        mem_history.push_back(*memory_usage as i32);
      }
    });
}

///
/// ## CPU usage history
///
/// (Last `seconds` from newest, max MAX_HISTORY_QUERY_DURATION_SECONDS) collected in reverse order
///
pub fn cpu_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.cpu_history.lock().unwrap();
  let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}

///
/// ## Memory usage history
///
/// (Last `seconds` from newest, max MAX_HISTORY_QUERY_DURATION_SECONDS) collected in reverse order
///
pub fn memory_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.memory_history.lock().unwrap();
  let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}

///
/// ## GPU usage history
///
/// (Last `seconds` from newest, max MAX_HISTORY_QUERY_DURATION_SECONDS) collected in reverse order
///
pub fn gpu_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.gpu_history.lock().unwrap();
  let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;

  history.iter().rev().take(take_n).cloned().collect()
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
  use std::collections::HashMap;

  // ── Helpers ──

  fn create_test_state(history_data: Vec<f32>) -> HardwareMonitorState {
    let history: VecDeque<f32> = history_data.into();
    HardwareMonitorState {
      system: Arc::new(Mutex::new(sysinfo::System::new())),
      cpu_history: Arc::new(Mutex::new(history.clone())),
      memory_history: Arc::new(Mutex::new(history.clone())),
      gpu_history: Arc::new(Mutex::new(history)),
      process_cpu_histories: Arc::new(Mutex::new(HashMap::new())),
      process_memory_histories: Arc::new(Mutex::new(HashMap::new())),
      gpu_usage_histories: Arc::new(Mutex::new(HashMap::new())),
      gpu_temperature_histories: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  fn create_test_resources() -> MonitorResources {
    MonitorResources {
      system: Arc::new(Mutex::new(sysinfo::System::new())),
      cpu_history: Arc::new(Mutex::new(VecDeque::new())),
      memory_history: Arc::new(Mutex::new(VecDeque::new())),
      process_cpu_histories: Arc::new(Mutex::new(HashMap::new())),
      process_memory_histories: Arc::new(Mutex::new(HashMap::new())),
      gpu_usage_histories: Arc::new(Mutex::new(HashMap::new())),
      gpu_temperature_histories: Arc::new(Mutex::new(HashMap::new())),
      gpu_dedicated_memory_histories: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  // ── calculate_memory_usage_percentage ──

  #[test]
  fn memory_usage_percentage_total_zero() {
    assert_eq!(calculate_memory_usage_percentage(1000, 0), 0.0);
  }

  #[test]
  fn memory_usage_percentage_half_used() {
    assert_eq!(calculate_memory_usage_percentage(500, 1000), 50.0);
  }

  #[test]
  fn memory_usage_percentage_fully_used() {
    assert_eq!(calculate_memory_usage_percentage(1000, 1000), 100.0);
  }

  #[test]
  fn memory_usage_percentage_zero_used() {
    assert_eq!(calculate_memory_usage_percentage(0, 1000), 0.0);
  }

  #[test]
  fn memory_usage_percentage_rounding() {
    assert_eq!(calculate_memory_usage_percentage(333, 1000), 33.0);
  }

  #[test]
  fn memory_usage_percentage_large_values() {
    assert_eq!(
      calculate_memory_usage_percentage(16_000_000_000, 32_000_000_000),
      50.0
    );
  }

  // ── push_history ──

  #[test]
  fn push_history_to_empty() {
    let history = Arc::new(Mutex::new(VecDeque::new()));
    push_history(&history, 1.0);
    let h = history.lock().unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0], 1.0);
  }

  #[test]
  fn push_history_under_capacity() {
    let history = Arc::new(Mutex::new(VecDeque::from(vec![0.0; 10])));
    push_history(&history, 1.0);
    assert_eq!(history.lock().unwrap().len(), 11);
  }

  #[test]
  fn push_history_at_capacity_evicts_oldest() {
    let initial: VecDeque<f32> = (0..HARDWARE_HISTORY_BUFFER_SIZE)
      .map(|i| i as f32)
      .collect();
    let history = Arc::new(Mutex::new(initial));
    push_history(&history, 999.0);

    let h = history.lock().unwrap();
    assert_eq!(h.len(), HARDWARE_HISTORY_BUFFER_SIZE);
    assert_eq!(*h.front().unwrap(), 1.0); // 0.0 was evicted
    assert_eq!(*h.back().unwrap(), 999.0);
  }

  #[test]
  fn push_history_preserves_order() {
    let history = Arc::new(Mutex::new(VecDeque::new()));
    push_history(&history, 1.0);
    push_history(&history, 2.0);
    push_history(&history, 3.0);
    let h = history.lock().unwrap();
    assert_eq!(h.iter().cloned().collect::<Vec<_>>(), vec![1.0, 2.0, 3.0]);
  }

  // ── cpu_usage_history ──

  #[test]
  fn cpu_usage_history_returns_last_n_reversed() {
    let state = create_test_state(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let result = cpu_usage_history(&state, 3);
    assert_eq!(result, vec![5.0, 4.0, 3.0]);
  }

  #[test]
  fn cpu_usage_history_clamped_to_available() {
    let state = create_test_state(vec![1.0, 2.0]);
    let result = cpu_usage_history(&state, 10);
    assert_eq!(result, vec![2.0, 1.0]);
  }

  #[test]
  fn cpu_usage_history_clamped_to_max_duration() {
    let data: Vec<f32> = (0..5000).map(|i| i as f32).collect();
    let state = create_test_state(data);
    let result = cpu_usage_history(&state, u32::MAX);
    assert_eq!(result.len(), MAX_HISTORY_QUERY_DURATION_SECONDS as usize);
  }

  #[test]
  fn cpu_usage_history_empty() {
    let state = create_test_state(vec![]);
    let result = cpu_usage_history(&state, 5);
    assert!(result.is_empty());
  }

  // ── memory_usage_history ──

  #[test]
  fn memory_usage_history_returns_last_n_reversed() {
    let state = create_test_state(vec![10.0, 20.0, 30.0]);
    let result = memory_usage_history(&state, 2);
    assert_eq!(result, vec![30.0, 20.0]);
  }

  #[test]
  fn memory_usage_history_empty() {
    let state = create_test_state(vec![]);
    let result = memory_usage_history(&state, 5);
    assert!(result.is_empty());
  }

  // ── gpu_usage_history ──

  #[test]
  fn gpu_usage_history_returns_last_n_reversed() {
    let state = create_test_state(vec![40.0, 50.0, 60.0, 70.0]);
    let result = gpu_usage_history(&state, 3);
    assert_eq!(result, vec![70.0, 60.0, 50.0]);
  }

  #[test]
  fn gpu_usage_history_empty() {
    let state = create_test_state(vec![]);
    let result = gpu_usage_history(&state, 5);
    assert!(result.is_empty());
  }

  // ── update_gpu_histories ──

  #[test]
  fn update_gpu_histories_adds_new_gpu() {
    let resources = create_test_resources();
    let samples: Vec<GpuSample> =
      vec![("TestGPU".to_string(), Some(50.0), Some(70.0), Some(1024.0))];
    update_gpu_histories(&resources, &samples);

    let usage = resources.gpu_usage_histories.lock().unwrap();
    assert_eq!(usage.get("TestGPU").unwrap().len(), 1);
    assert_eq!(*usage.get("TestGPU").unwrap().back().unwrap(), 50.0);

    let temp = resources.gpu_temperature_histories.lock().unwrap();
    assert_eq!(*temp.get("TestGPU").unwrap().back().unwrap(), 70);

    let mem = resources.gpu_dedicated_memory_histories.lock().unwrap();
    assert_eq!(*mem.get("TestGPU").unwrap().back().unwrap(), 1024);
  }

  #[test]
  fn update_gpu_histories_none_metrics_skipped() {
    let resources = create_test_resources();
    let samples: Vec<GpuSample> =
      vec![("NoData".to_string(), None, None, None)];
    update_gpu_histories(&resources, &samples);

    assert!(resources.gpu_usage_histories.lock().unwrap().is_empty());
    assert!(resources.gpu_temperature_histories.lock().unwrap().is_empty());
    assert!(
      resources
        .gpu_dedicated_memory_histories
        .lock()
        .unwrap()
        .is_empty()
    );
  }

  #[test]
  fn update_gpu_histories_partial_metrics() {
    let resources = create_test_resources();
    let samples: Vec<GpuSample> =
      vec![("Partial".to_string(), Some(80.0), None, None)];
    update_gpu_histories(&resources, &samples);

    assert_eq!(
      resources.gpu_usage_histories.lock().unwrap().len(),
      1
    );
    assert!(resources.gpu_temperature_histories.lock().unwrap().is_empty());
    assert!(
      resources
        .gpu_dedicated_memory_histories
        .lock()
        .unwrap()
        .is_empty()
    );
  }

  #[test]
  fn update_gpu_histories_multiple_gpus() {
    let resources = create_test_resources();
    let samples: Vec<GpuSample> = vec![
      ("GPU_A".to_string(), Some(10.0), None, None),
      ("GPU_B".to_string(), Some(90.0), None, None),
    ];
    update_gpu_histories(&resources, &samples);

    let usage = resources.gpu_usage_histories.lock().unwrap();
    assert_eq!(usage.len(), 2);
    assert!(usage.contains_key("GPU_A"));
    assert!(usage.contains_key("GPU_B"));
  }

  #[test]
  fn update_gpu_histories_respects_buffer_cap() {
    let resources = create_test_resources();
    // Fill to capacity
    for i in 0..HARDWARE_HISTORY_BUFFER_SIZE {
      let samples: Vec<GpuSample> =
        vec![("GPU".to_string(), Some(i as f32), None, None)];
      update_gpu_histories(&resources, &samples);
    }
    // Add one more
    let samples: Vec<GpuSample> =
      vec![("GPU".to_string(), Some(999.0), None, None)];
    update_gpu_histories(&resources, &samples);

    let usage = resources.gpu_usage_histories.lock().unwrap();
    let history = usage.get("GPU").unwrap();
    assert_eq!(history.len(), HARDWARE_HISTORY_BUFFER_SIZE);
    assert_eq!(*history.back().unwrap(), 999.0);
    assert_eq!(*history.front().unwrap(), 1.0); // 0.0 was evicted
  }

  // ── update_process_histories ──

  #[test]
  fn update_process_histories_new_process() {
    let resources = create_test_resources();
    let pid = sysinfo::Pid::from_u32(100);
    let metrics = vec![(pid, 50.0_f32, 256.0_f32)];
    update_process_histories(&resources, &metrics);

    let cpu_h = resources.process_cpu_histories.lock().unwrap();
    assert_eq!(cpu_h.get(&pid).unwrap().len(), 1);
    assert_eq!(*cpu_h.get(&pid).unwrap().back().unwrap(), 50.0);

    let mem_h = resources.process_memory_histories.lock().unwrap();
    assert_eq!(*mem_h.get(&pid).unwrap().back().unwrap(), 256.0);
  }

  #[test]
  fn update_process_histories_existing_process() {
    let resources = create_test_resources();
    let pid = sysinfo::Pid::from_u32(200);
    let metrics = vec![(pid, 10.0_f32, 128.0_f32)];
    update_process_histories(&resources, &metrics);
    let metrics2 = vec![(pid, 20.0_f32, 256.0_f32)];
    update_process_histories(&resources, &metrics2);

    let cpu_h = resources.process_cpu_histories.lock().unwrap();
    assert_eq!(cpu_h.get(&pid).unwrap().len(), 2);
  }

  #[test]
  fn update_process_histories_respects_buffer_cap() {
    let resources = create_test_resources();
    let pid = sysinfo::Pid::from_u32(300);
    for i in 0..HARDWARE_HISTORY_BUFFER_SIZE + 5 {
      let metrics = vec![(pid, i as f32, i as f32)];
      update_process_histories(&resources, &metrics);
    }

    let cpu_h = resources.process_cpu_histories.lock().unwrap();
    assert_eq!(cpu_h.get(&pid).unwrap().len(), HARDWARE_HISTORY_BUFFER_SIZE);
  }
}
