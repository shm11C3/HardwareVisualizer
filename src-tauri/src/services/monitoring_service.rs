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
  let mut sys = match resources.system.lock() {
    Ok(s) => s,
    Err(_) => return,
  };
  sys.refresh_all();

  // CPU 平均使用率
  let cpu_usage = {
    let cpus = sys.cpus();
    if cpus.is_empty() {
      0.0
    } else {
      (cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32).round()
    }
  };
  push_history(&resources.cpu_history, cpu_usage);

  // メモリ使用率
  let memory_usage = {
    let used_memory = sys.used_memory() as f64;
    let total_memory = sys.total_memory() as f64;
    if total_memory == 0.0 {
      0.0
    } else {
      (used_memory / total_memory * 100.0).round() as f32
    }
  };
  push_history(&resources.memory_history, memory_usage);

  // プロセス履歴
  {
    let mut proc_cpu_histories = resources.process_cpu_histories.lock().unwrap();
    let mut proc_mem_histories = resources.process_memory_histories.lock().unwrap();
    for (pid, process) in sys.processes() {
      let cpu = process.cpu_usage();
      let cpu_hist = proc_cpu_histories.entry(*pid).or_default();
      push_history_inner(cpu_hist, cpu);

      let mem = process.memory() as f32 / 1024.0; // KB→MB
      let mem_hist = proc_mem_histories.entry(*pid).or_default();
      push_history_inner(mem_hist, mem);
    }
  }
}

/// Windows 向け GPU サンプリング
#[cfg(target_os = "windows")]
pub fn sample_gpu(resources: &MonitorResources) {
  use crate::infrastructure::nvapi;
  use nvapi::PhysicalGpu;

  let mut usage_histories = resources.nv_gpu_usage_histories.lock().unwrap();
  let mut temp_histories = resources.nv_gpu_temperature_histories.lock().unwrap();
  let mut mem_histories = resources.nv_gpu_dedicated_memory_histories.lock().unwrap();

  let gpus = match PhysicalGpu::enumerate() {
    Ok(g) => g,
    Err(_) => return,
  };

  for (name, gpu) in gpus
    .iter()
    .map(|g| (g.full_name().unwrap_or("Unknown".to_string()), g))
  {
    let usage_hist = usage_histories.entry(name.clone()).or_default();
    push_history_inner(usage_hist, nvapi::get_gpu_usage_from_physical_gpu(gpu));

    let temp_hist = temp_histories.entry(name.clone()).or_default();
    push_history_inner(
      temp_hist,
      nvapi::get_gpu_temperature_from_physical_gpu(gpu) as f32,
    );

    let mem_hist = mem_histories.entry(name.clone()).or_default();
    push_history_inner(
      mem_hist,
      nvapi::get_gpu_dedicated_memory_usage_from_physical_gpu(gpu) as f32,
    );
  }
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
