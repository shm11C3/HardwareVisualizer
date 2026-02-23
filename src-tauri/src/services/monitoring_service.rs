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

  // IOKit usage sampler スレッドを起動（初回のみ）
  let _ = gpu::init_gpu_usage_sampler_thread();

  // GPU 名をキャッシュ（初回のみ IOKit + system_profiler で取得）
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

  // GPU 使用率（IOKit IOReport のキャッシュ値、0.0-1.0 → 0-100%）
  let usage = gpu::read_gpu_usage_cached().map(|v| v * 100.0);

  // 温度: macOS では取得不可 → None
  let temperature: Option<f32> = None;

  // メモリ使用量: IOKit から bytes → KB に変換（Windows NVAPI と同じ単位）
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
