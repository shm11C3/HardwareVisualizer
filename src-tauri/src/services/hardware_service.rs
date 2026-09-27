use crate::models::hardware::SysInfo;
use crate::services::motherboard_service;
use crate::{log_error, log_warn};
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
    PlatformFactory::shared().map_err(|e| format!("Failed to create platform: {e}"))?;

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
      log_error!(
        "gpu_info_failed",
        "collect_hardware_info",
        Some(e.to_string())
      );
      None
    }
  };

  let memory = match memory_res {
    Ok(v) => Some(v.into()),
    Err(e) => {
      log_error!(
        "memory_info_failed",
        "collect_hardware_info",
        Some(e.to_string())
      );
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
      log_error!(
        "motherboard_info_failed",
        "collect_hardware_info",
        Some(e.to_string())
      );
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

pub async fn get_storage_health_latest_records() -> Result<
  Vec<hardviz_core::models::hardware::StorageHealthRecord>,
  hardviz_core::infrastructure::database::dispatch::DispatchError,
> {
  hardviz_core::infrastructure::database::dispatch::storage_health::latest_records().await
}

pub async fn get_super_io_chip_id_diagnostics()
-> hardviz_core::models::hardware::SuperIoChipIdDiagnostics {
  tokio::task::spawn_blocking(|| {
    let platform =
      PlatformFactory::shared().map_err(|e| format!("Failed to create platform: {e}"))?;
    Ok::<_, String>(platform.get_super_io_chip_id_diagnostics())
  })
  .await
  .map_err(|e| format!("Failed to join Super I/O diagnostic task: {e}"))
  .and_then(|result| result)
  .unwrap_or_else(
    |error| hardviz_core::models::hardware::SuperIoChipIdDiagnostics {
      platform_supported: cfg!(target_os = "windows"),
      pawnio: None,
      slots: Vec::new(),
      error: Some(error),
    },
  )
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

pub async fn refresh_storage_devices(
  retention_days: u32,
  identity_hash_key: [u8; hardviz_core::settings::STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES],
  collector: Option<std::sync::Arc<hardviz_core::collector::LiveStorageHealthCollector>>,
  guidance_sink: Option<hardviz_core::persistence::ExternalComponentGuidanceSink>,
  ensure_writable: impl FnOnce() -> Result<(), String>,
) -> Result<Vec<hardviz_core::models::hardware::StorageHealthRecord>, String> {
  let Some(collector) = collector else {
    return Err("Storage device re-detection is unavailable".to_string());
  };

  let active_device_ids = match tokio::task::spawn_blocking(move || {
    collector.refresh_devices()
  })
  .await
  {
    Ok(Ok(device_ids)) => device_ids,
    Ok(Err(e)) => {
      log_warn!(
        "Storage device re-detection enumeration failed; continuing without deactivation",
        "services::hardware_service::refresh_storage_devices",
        Some(e)
      );
      Vec::new()
    }
    Err(e) => {
      log_warn!(
        "Storage device re-detection enumeration task failed; continuing without deactivation",
        "services::hardware_service::refresh_storage_devices",
        Some(e.to_string())
      );
      Vec::new()
    }
  };

  // #2271: the command's own `ensure_database_writable` check happens
  // before the (blocking, I/O-bound) device enumeration above; a conversion
  // can start and reach `Converting` in that gap. Re-checking here, right
  // before the write, closes the window down to this call's own await
  // instead of leaving it open for the whole enumeration.
  ensure_writable()?;

  hardviz_core::persistence::refresh_storage_health_for_date(
    retention_days,
    &hardviz_core::persistence::local_storage_health_date_string(),
    &identity_hash_key,
    active_device_ids,
    guidance_sink.as_ref(),
  )
  .await?;

  hardviz_core::infrastructure::database::dispatch::storage_health::latest_records()
    .await
    .map_err(|e| format!("Failed to fetch refreshed storage health records: {e}"))
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

  #[tokio::test]
  async fn refresh_storage_devices_errors_when_collector_is_absent() {
    let error = refresh_storage_devices(1, [0x42; 32], None, None, || Ok(()))
      .await
      .expect_err("missing collector should be reported");

    assert!(error.contains("unavailable"));
  }

  /// #2271: the collector-absent check happens before `ensure_writable` is
  /// ever called, so this proves the guard added for the conversion-window
  /// race does not shadow that earlier, unrelated refusal.
  #[tokio::test]
  async fn refresh_storage_devices_does_not_consult_ensure_writable_when_collector_is_absent()
   {
    let error = refresh_storage_devices(1, [0x42; 32], None, None, || {
      panic!("ensure_writable must not be called without a collector")
    })
    .await
    .expect_err("missing collector should be reported");

    assert!(error.contains("unavailable"));
  }

  /// #2271: a conversion can reach `Converting` during the (blocking)
  /// device enumeration this function awaits before its write. Re-checking
  /// writability right before the write - not only at the command's own
  /// entry, before enumeration - must refuse instead of writing through a
  /// window a caller believed was already closed.
  #[tokio::test]
  async fn refresh_storage_devices_refuses_the_write_when_ensure_writable_fails_after_enumeration()
   {
    let collector = Arc::new(LiveStorageHealthCollector::new([0x42; 32]));

    let error = refresh_storage_devices(1, [0x42; 32], Some(collector), None, || {
      Err("the database is unavailable for writing right now".to_string())
    })
    .await
    .expect_err("a writability check that fails after enumeration must refuse the write");

    assert!(error.contains("unavailable for writing"));
  }

  #[cfg(not(target_os = "windows"))]
  #[tokio::test]
  async fn super_io_chip_id_diagnostics_reports_unsupported_platform() {
    let diagnostics = get_super_io_chip_id_diagnostics().await;

    assert!(!diagnostics.platform_supported);
    assert!(diagnostics.pawnio.is_none());
    assert!(diagnostics.slots.is_empty());
    assert!(diagnostics.error.unwrap().contains("Windows only"));
  }
}
