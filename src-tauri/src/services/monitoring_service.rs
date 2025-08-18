use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::constants::{
  HARDWARE_HISTORY_BUFFER_SIZE,
  MAX_HISTORY_QUERY_DURATION_SECONDS,
};
use crate::structs::hardware::HardwareMonitorState;
use crate::structs::hardware_archive::MonitorResources;


/// 1サイクル分のシステムサンプリング (CPU/メモリ/プロセス)
pub fn sample_system(resources: &MonitorResources) {
  resources
    .system
    .lock()
    .ok()
    .map(|mut sys| {
      sys.refresh_all();
      
      let cpu_usage = calculate_average_cpu_usage(sys.cpus());
      let memory_usage = calculate_memory_usage_percentage(sys.used_memory(), sys.total_memory());
      
      let process_metrics: Vec<_> = sys
        .processes()
        .iter()
        .map(|(pid, process)| (*pid, process.cpu_usage(), process.memory() as f32 / 1024.0))
        .collect();
      
      (cpu_usage, memory_usage, process_metrics)
    })
    .map(|(cpu_usage, memory_usage, process_metrics)| {
      push_history(&resources.cpu_history, cpu_usage);
      push_history(&resources.memory_history, memory_usage);
      update_process_histories(resources, &process_metrics);
    });
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
  
  process_metrics.iter().for_each(|(pid, cpu_usage, memory_mb)| {
    push_history_inner(cpu_histories.entry(*pid).or_default(), *cpu_usage);
    push_history_inner(mem_histories.entry(*pid).or_default(), *memory_mb);
  });
}

#[cfg(target_os = "windows")]
pub fn sample_gpu(resources: &MonitorResources) {
  use crate::infrastructure::nvapi;
  use nvapi::PhysicalGpu;

  PhysicalGpu::enumerate()
    .ok()
    .map(|gpus| {
      gpus
        .iter()
        .map(|gpu| {
          let name = gpu.full_name().unwrap_or_else(|| "Unknown".to_string());
          let usage = nvapi::get_gpu_usage_from_physical_gpu(gpu);
          let temperature = nvapi::get_gpu_temperature_from_physical_gpu(gpu) as f32;
          let memory_usage = nvapi::get_gpu_dedicated_memory_usage_from_physical_gpu(gpu) as f32;
          (name, usage, temperature, memory_usage)
        })
        .collect::<Vec<_>>()
    })
    .map(|gpu_metrics| update_gpu_histories(resources, &gpu_metrics));
}

#[cfg(target_os = "windows")]
fn update_gpu_histories(
  resources: &MonitorResources,
  gpu_metrics: &[(String, f32, f32, f32)],
) {
  let mut usage_histories = resources.nv_gpu_usage_histories.lock().unwrap();
  let mut temp_histories = resources.nv_gpu_temperature_histories.lock().unwrap();
  let mut mem_histories = resources.nv_gpu_dedicated_memory_histories.lock().unwrap();

  gpu_metrics.iter().for_each(|(name, usage, temperature, memory_usage)| {
    push_history_inner(usage_histories.entry(name.clone()).or_default(), *usage);
    push_history_inner(temp_histories.entry(name.clone()).or_default(), *temperature);
    push_history_inner(mem_histories.entry(name.clone()).or_default(), *memory_usage);
  });
}

///
/// ## CPU 使用率履歴
///
/// (最新から `seconds` 秒, 上限 MAX_HISTORY_QUERY_DURATION_SECONDS) を逆順スライス収集
///
pub fn cpu_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.cpu_history.lock().unwrap();
  let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}

///
/// ## メモリ使用率履歴
///
/// (最新から `seconds` 秒, 上限 MAX_HISTORY_QUERY_DURATION_SECONDS) を逆順スライス収集
///
pub fn memory_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.memory_history.lock().unwrap();
  let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}

///
/// ## GPU 使用率履歴
///
/// (最新から `seconds` 秒, 上限 MAX_HISTORY_QUERY_DURATION_SECONDS) を逆順スライス収集
///
pub fn gpu_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.gpu_history.lock().unwrap();
  let take_n = seconds.min(MAX_HISTORY_QUERY_DURATION_SECONDS) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}

fn push_history(history: &Arc<Mutex<VecDeque<f32>>>, value: f32) {
  let mut h = history.lock().unwrap();
  push_history_inner(&mut h, value);
}

fn push_history_inner<T: Into<f32> + Copy>(deque: &mut VecDeque<T>, value: T) {
  if deque.len() >= HARDWARE_HISTORY_BUFFER_SIZE {
    deque.pop_front();
  }
  deque.push_back(value);
}
