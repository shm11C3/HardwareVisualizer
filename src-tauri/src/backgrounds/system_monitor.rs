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
  let mut interval =
    tokio::time::interval(Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL));

  loop {
    interval.tick().await;

    {
      let monitor_resources = structs::hardware_archive::MonitorResources {
        system: resources.system.clone(),
        cpu_history: resources.cpu_history.clone(),
        memory_history: resources.memory_history.clone(),
        process_cpu_histories: resources.process_cpu_histories.clone(),
        process_memory_histories: resources.process_memory_histories.clone(),
        nv_gpu_usage_histories: resources.nv_gpu_usage_histories.clone(),
        nv_gpu_temperature_histories: resources.nv_gpu_temperature_histories.clone(),
        nv_gpu_dedicated_memory_histories: resources
          .nv_gpu_dedicated_memory_histories
          .clone(),
      };

      #[cfg(target_os = "windows")]
      {
        monitoring_service::sample_system(&monitor_resources);
        monitoring_service::sample_gpu(&monitor_resources);
      }
      #[cfg(target_os = "linux")]
      {
        monitoring_service::sample_system(&monitor_resources);
      }
    }
  }
}
