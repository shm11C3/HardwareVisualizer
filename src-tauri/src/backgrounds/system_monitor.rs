use crate::services;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo;

///
/// システム情報の更新頻度（秒）
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1;

///
/// データを保持する期間（秒）
///
const HISTORY_CAPACITY: usize = 60;

///
/// ## システム情報の初期化
///
/// - param system: `Arc<Mutex<System>>` システム情報
///
/// - `SYSTEM_INFO_INIT_INTERVAL` 秒ごとにCPU使用率とメモリ使用率を更新
///
pub async fn setup(
  system: Arc<Mutex<sysinfo::System>>,
  cpu_history: Arc<Mutex<VecDeque<f32>>>,
  memory_history: Arc<Mutex<VecDeque<f32>>>,
  process_cpu_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  process_memory_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  nv_gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  nv_gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  nv_gpu_dedicated_memory_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
) {
  let mut interval = tokio::time::interval(Duration::from_secs(HISTORY_CAPACITY as u64));

  loop {
    interval.tick().await;

    {
      let mut sys = match system.lock() {
        Ok(s) => s,
        Err(_) => continue, // エラーハンドリング：ロックが破損している場合はスキップ
      };

      sys.refresh_all();

      let cpu_usage = {
        let cpus = sys.cpus();
        let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
        (total_usage / cpus.len() as f32).round()
      };

      let memory_usage = {
        let used_memory = sys.used_memory() as f64;
        let total_memory = sys.total_memory() as f64;
        (used_memory / total_memory * 100.0).round() as f32
      };

      {
        let mut cpu_hist = cpu_history.lock().unwrap();
        if cpu_hist.len() >= HISTORY_CAPACITY {
          cpu_hist.pop_front();
        }
        cpu_hist.push_back(cpu_usage);
      }

      {
        let mut memory_hist = memory_history.lock().unwrap();
        if memory_hist.len() >= HISTORY_CAPACITY {
          memory_hist.pop_front();
        }
        memory_hist.push_back(memory_usage);
      }

      // 各プロセスごとのCPUおよびメモリ使用率を保存
      {
        let mut process_cpu_histories = process_cpu_histories.lock().unwrap();
        let mut process_memory_histories = process_memory_histories.lock().unwrap();

        for (pid, process) in sys.processes() {
          // CPU使用率の履歴を更新
          let cpu_usage = process.cpu_usage();
          let cpu_history = process_cpu_histories.entry(*pid).or_default();

          if cpu_history.len() >= HISTORY_CAPACITY {
            cpu_history.pop_front();
          }
          cpu_history.push_back(cpu_usage);

          // メモリ使用率の履歴を更新
          let memory_usage = process.memory() as f32 / 1024.0; // KB単位からMB単位に変換
          let memory_history = process_memory_histories.entry(*pid).or_default();

          if memory_history.len() >= HISTORY_CAPACITY {
            memory_history.pop_front();
          }
          memory_history.push_back(memory_usage);
        }
      }

      // GPU使用率の履歴を保存
      {
        let mut nv_gpu_usage_histories = nv_gpu_usage_histories.lock().unwrap();
        let mut nv_gpu_temperature_histories =
          nv_gpu_temperature_histories.lock().unwrap();

        let gpus = match nvapi::PhysicalGpu::enumerate() {
          Ok(gpus) => gpus,
          Err(_) => continue,
        };

        for (name, gpu) in gpus
          .iter()
          .map(|gpu| (gpu.full_name().unwrap_or("Unknown".to_string()), gpu))
        {
          let usage_history = nv_gpu_usage_histories.entry(name.clone()).or_default();

          if usage_history.len() >= HISTORY_CAPACITY {
            usage_history.pop_front();
          }
          usage_history.push_back(
            services::nvidia_gpu_service::get_gpu_usage_from_physical_gpu(gpu),
          );

          let temperature_history = nv_gpu_temperature_histories
            .entry(name.clone())
            .or_default();

          if temperature_history.len() >= HISTORY_CAPACITY {
            temperature_history.pop_front();
          }
          temperature_history.push_back(
            services::nvidia_gpu_service::get_gpu_temperature_from_physical_gpu(gpu),
          );

          let mut nv_gpu_dedicated_memory_histories =
            nv_gpu_dedicated_memory_histories.lock().unwrap();
          let gpu_memory_history = nv_gpu_dedicated_memory_histories
            .entry(name.clone())
            .or_default();

          if gpu_memory_history.len() >= HISTORY_CAPACITY {
            gpu_memory_history.pop_front();
          }
          gpu_memory_history.push_back(
            services::nvidia_gpu_service::get_gpu_dedicated_memory_usage_from_physical_gpu(gpu) as i32,
          );
        }
      }
    }
    tokio::time::sleep(Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL)).await;
  }
}
