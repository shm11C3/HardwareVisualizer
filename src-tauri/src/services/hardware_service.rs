use crate::log_error;
use crate::models::hardware::SysInfo;
use crate::services::motherboard_service;
use hardviz_core::collector::HistoryStore;
use hardviz_core::infrastructure::providers::sysinfo_provider;
use hardviz_core::platform::factory::PlatformFactory;

///
/// Collect hardware information in aggregate
///
/// - Get CPU / GPU / Memory / Storage / Motherboard respectively
/// - Continue with None (or empty) for individual failures
/// - Return Err if all of CPU / GPU / Memory cannot be obtained
///
pub async fn collect_hardware_info(store: &HistoryStore) -> Result<SysInfo, String> {
  // CPU follows the same "log and continue" rule as the GPU / memory /
  // motherboard branches below: a poisoned `HistoryStore::system` mutex
  // or a provider failure is logged and produces `None` instead of
  // aborting the whole call.
  let cpu = match store.system().lock() {
    Ok(system) => match sysinfo_provider::get_cpu_info(system) {
      Ok(v) => Some(v.into()),
      Err(e) => {
        log_error!("cpu_info_failed", "collect_hardware_info", Some(e));
        None
      }
    },
    Err(e) => {
      log_error!(
        "cpu_info_failed",
        "collect_hardware_info",
        Some(format!("HistoryStore.system lock poisoned: {e}"))
      );
      None
    }
  };

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

pub async fn get_storage_health_latest_records()
-> Result<Vec<hardviz_core::models::hardware::StorageHealthRecord>, sqlx::Error> {
  hardviz_core::infrastructure::database::storage_health::latest_records().await
}

///
/// Read Live Storage Health signals from the startup-cached device list
/// (ADR 0006). Nothing is persisted.
///
/// Returns an empty list when the collector is unavailable (Storage
/// Health disabled at startup or an invalid identity key). The read is
/// blocking — one `DeviceIoControl` query per cached device — so it runs
/// on the blocking pool.
///
pub async fn get_live_storage_health(
  collector: Option<std::sync::Arc<hardviz_core::collector::LiveStorageHealthCollector>>,
) -> Result<Vec<hardviz_core::models::hardware::LiveStorageHealth>, String> {
  let Some(collector) = collector else {
    return Ok(Vec::new());
  };

  tokio::task::spawn_blocking(move || collector.read_live())
    .await
    .map_err(|e| format!("Failed to join live storage health read: {e}"))
}

#[cfg(test)]
mod tests {
  use super::*;
  use hardviz_core::collector::LiveStorageHealthCollector;
  use std::sync::Arc;

  #[tokio::test]
  async fn live_storage_health_returns_empty_when_collector_is_absent() {
    let signals = get_live_storage_health(None).await.expect("must not fail");

    assert!(signals.is_empty());
  }

  #[tokio::test]
  async fn live_storage_health_returns_empty_before_enumeration() {
    // An un-enumerated collector has an empty device cache, so the read
    // returns empty without touching any device on every platform.
    let collector = Arc::new(LiveStorageHealthCollector::new([0x42; 32]));

    let signals = get_live_storage_health(Some(collector))
      .await
      .expect("must not fail");

    assert!(signals.is_empty());
  }
}
