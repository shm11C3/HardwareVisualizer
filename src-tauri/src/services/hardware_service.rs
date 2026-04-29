use crate::log_error;
use crate::models::hardware::SysInfo;
use crate::services::motherboard_service;
use hwviz_core::collector::HistoryStore;
use hwviz_core::infrastructure::providers::sysinfo_provider;
use hwviz_core::platform::factory::PlatformFactory;

///
/// Collect hardware information in aggregate
///
/// - Get CPU / GPU / Memory / Storage / Motherboard respectively
/// - Continue with None (or empty) for individual failures
/// - Return Err if all of CPU / GPU / Memory cannot be obtained
///
pub async fn collect_hardware_info(store: &HistoryStore) -> Result<SysInfo, String> {
  let cpu = sysinfo_provider::get_cpu_info(store.system().lock().unwrap())
    .ok()
    .map(Into::into);

  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;

  // Execute GPU / Memory / Storage / Motherboard in parallel
  let (gpus_res, memory_res, storage_res, motherboard_res) = tokio::join!(
    platform.get_gpu_info(),
    platform.get_memory_info(),
    async { sysinfo_provider::get_storage_info() },
    motherboard_service::fetch_motherboard_info(),
  );

  let gpus = match gpus_res {
    Ok(v) => Some(v.into_iter().map(Into::into).collect()),
    Err(e) => {
      log_error!("gpu_info_failed", "collect_hardware_info", Some(e));
      None
    }
  };

  let memory = match memory_res {
    Ok(v) => Some(v.into()),
    Err(e) => {
      log_error!("memory_info_failed", "collect_hardware_info", Some(e));
      None
    }
  };
  // Storage info follows the same "log and continue" rule as the GPU
  // and memory branches above; the doc-comment on this function
  // promises partial results.
  let storage: Vec<crate::models::hardware::StorageInfo> = match storage_res {
    Ok(v) => v.into_iter().map(Into::into).collect(),
    Err(e) => {
      log_error!("storage_info_failed", "collect_hardware_info", Some(e));
      Vec::new()
    }
  };

  let motherboard = match motherboard_res {
    Ok(v) => Some(v),
    Err(e) => {
      log_error!("motherboard_info_failed", "collect_hardware_info", Some(e));
      None
    }
  };

  if cpu.is_none() && gpus.is_none() && memory.is_none() {
    return Err("Failed to get any hardware info".to_string());
  }

  Ok(SysInfo {
    cpu,
    memory,
    gpus,
    storage,
    motherboard,
  })
}
