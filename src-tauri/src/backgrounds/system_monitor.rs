use crate::services::monitoring_service;
use crate::structs;
use std::time::Duration;

///
/// システム情報の更新頻度（秒）
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1;

///
/// ## システム情報の初期化
///
/// - param system: `Arc<Mutex<System>>` システム情報
///
/// - `SYSTEM_INFO_INIT_INTERVAL` 秒ごとにCPU使用率とメモリ使用率を更新
///
pub async fn setup(resources: structs::hardware_archive::MonitorResources) {
  #[cfg(target_os = "windows")]
  let structs::hardware_archive::MonitorResources {
    system,
    cpu_history,
    memory_history,
    process_cpu_histories,
    process_memory_histories,
    nv_gpu_usage_histories,
    nv_gpu_temperature_histories,
    nv_gpu_dedicated_memory_histories,
  } = resources;

  #[cfg(target_os = "linux")]
  let structs::hardware_archive::MonitorResources {
    system,
    cpu_history,
    memory_history,
    process_cpu_histories,
    process_memory_histories,
    nv_gpu_usage_histories: _nv_gpu_usage_histories,
    nv_gpu_temperature_histories: _nv_gpu_temperature_histories,
    nv_gpu_dedicated_memory_histories: _nv_gpu_dedicated_memory_histories,
  } = resources;

  let mut interval =
    tokio::time::interval(Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL));

  loop {
    interval.tick().await;

    {
      // サンプリングをサービスへ委譲 (内部で system をロック)
      #[cfg(target_os = "windows")]
      {
        monitoring_service::sample_system(&structs::hardware_archive::MonitorResources {
          system: system.clone(),
          cpu_history: cpu_history.clone(),
          memory_history: memory_history.clone(),
          process_cpu_histories: process_cpu_histories.clone(),
          process_memory_histories: process_memory_histories.clone(),
          nv_gpu_usage_histories: nv_gpu_usage_histories.clone(),
          nv_gpu_temperature_histories: nv_gpu_temperature_histories.clone(),
          nv_gpu_dedicated_memory_histories: nv_gpu_dedicated_memory_histories.clone(),
        });
        monitoring_service::sample_gpu(&structs::hardware_archive::MonitorResources {
          system: system.clone(),
          cpu_history: cpu_history.clone(),
          memory_history: memory_history.clone(),
          process_cpu_histories: process_cpu_histories.clone(),
          process_memory_histories: process_memory_histories.clone(),
          nv_gpu_usage_histories: nv_gpu_usage_histories.clone(),
          nv_gpu_temperature_histories: nv_gpu_temperature_histories.clone(),
          nv_gpu_dedicated_memory_histories: nv_gpu_dedicated_memory_histories.clone(),
        });
      }
      #[cfg(target_os = "linux")]
      {
        // Linux では GPU 履歴を未使用のため空のマップを再利用
        use std::collections::{HashMap, VecDeque};
        use std::sync::{Arc, Mutex};
        monitoring_service::sample_system(&structs::hardware_archive::MonitorResources {
          system: system.clone(),
          cpu_history: cpu_history.clone(),
          memory_history: memory_history.clone(),
          process_cpu_histories: process_cpu_histories.clone(),
          process_memory_histories: process_memory_histories.clone(),
          nv_gpu_usage_histories: Arc::new(Mutex::new(
            HashMap::<String, VecDeque<f32>>::new(),
          )),
          nv_gpu_temperature_histories: Arc::new(Mutex::new(HashMap::<
            String,
            VecDeque<i32>,
          >::new())),
          nv_gpu_dedicated_memory_histories: Arc::new(Mutex::new(HashMap::<
            String,
            VecDeque<i32>,
          >::new())),
        });
      }
    }
  }
}
