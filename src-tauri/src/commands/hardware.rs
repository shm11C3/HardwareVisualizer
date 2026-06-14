use crate::commands::settings;
use crate::enums::error::BackendError;
use crate::models;
use crate::models::archive_history::{
  ArchiveDataStats, ArchiveRecord, DataArchiveHardwareType, GpuArchiveDataType,
  ProcessStatRecord,
};
use crate::models::hardware::{NetworkInfo, ProcessInfo, SysInfo};
use crate::services::external_component_guidance_service::ExternalComponentGuidanceState;
use chrono::{DateTime, SecondsFormat, Utc};
use hardviz_core::collector::HistoryStore;
use hardviz_core::persistence::HARDWARE_ARCHIVE_INTERVAL_SECONDS;
use std::sync::Arc;
use tauri::command;

///
/// ## Get process list
///
#[command]
#[specta::specta]
pub fn get_process_list(state: tauri::State<'_, Arc<HistoryStore>>) -> Vec<ProcessInfo> {
  state
    .process_list()
    .into_iter()
    .map(ProcessInfo::from)
    .collect()
}

///
/// ## Get CPU usage (%)
///
/// - param state: shared `HistoryStore` from the Core collector.
/// - return: `i32` overall CPU usage (%)
///
#[command]
#[specta::specta]
pub fn get_cpu_usage(state: tauri::State<'_, Arc<HistoryStore>>) -> i32 {
  state.current_cpu_usage_overall()
}

#[command]
#[specta::specta]
pub fn get_processors_usage(state: tauri::State<'_, Arc<HistoryStore>>) -> Vec<f32> {
  state.current_cpu_usage_per_processor()
}

///
/// ## Get system information
///
#[command]
#[specta::specta]
pub async fn get_hardware_info(
  state: tauri::State<'_, Arc<HistoryStore>>,
) -> Result<SysInfo, String> {
  use crate::services::hardware_service;

  hardware_service::collect_hardware_info(state.inner().as_ref()).await
}

///
/// ## Get detailed memory information
///
/// - return: `models::hardware::MemoryInfo` Detailed memory information
///
#[command]
#[specta::specta]
pub async fn get_memory_info_detail() -> Result<models::hardware::MemoryInfo, String> {
  use crate::services::memory_service;

  memory_service::fetch_memory_detail().await
}

///
/// ## Get memory usage (%)
///
/// - param state: shared `HistoryStore`.
/// - return: `i32` Memory usage (%)
///
#[command]
#[specta::specta]
pub fn get_memory_usage(state: tauri::State<'_, Arc<HistoryStore>>) -> i32 {
  state.current_memory_usage_percent()
}

///
/// ## Get GPU usage (%)
///
/// Returns the GPU usage percentage together with the data-source
/// identifier (e.g. "NVAPI", "ADL", "PDH", "DRM (AMD)", "IOKit").
///
#[command]
#[specta::specta]
pub async fn get_gpu_usage() -> Result<models::hardware::GpuUsageResult, String> {
  use crate::services::gpu_service;

  gpu_service::fetch_gpu_usage().await
}

///
/// ## Get GPU temperature
///
#[command]
#[specta::specta]
pub async fn get_gpu_temperature(
  state: tauri::State<'_, settings::AppState>,
) -> Result<Vec<models::hardware::NameValue>, String> {
  use crate::services::gpu_service;

  let temperature_unit = {
    let config = state.settings.lock().unwrap();
    config.temperature_unit.clone()
  };

  gpu_service::fetch_gpu_temperature(temperature_unit).await
}

///
/// ## Get realtime GPU memory usage (best-effort)
///
/// **Platform support**: Currently implemented only on macOS. On other
/// platforms, or where the underlying APIs are not available, this will
/// return `Ok(None)` instead of failing.
///
#[command]
#[specta::specta]
pub async fn get_gpu_memory_usage()
-> Result<Option<models::hardware::GpuMemoryUsage>, String> {
  use crate::services::gpu_service;

  gpu_service::fetch_gpu_memory_usage().await
}

///
/// ## Get CPU usage history
///
/// - param state: shared `HistoryStore`.
/// - param seconds: `u32` Number of seconds to retrieve
///
#[command]
#[specta::specta]
pub fn get_cpu_usage_history(
  state: tauri::State<'_, Arc<HistoryStore>>,
  seconds: u32,
) -> Vec<f32> {
  state.cpu_history(seconds)
}

///
/// ## Get memory usage history
///
/// - param state: shared `HistoryStore`.
/// - param seconds: `u32` Number of seconds to retrieve
///
#[command]
#[specta::specta]
pub fn get_memory_usage_history(
  state: tauri::State<'_, Arc<HistoryStore>>,
  seconds: u32,
) -> Vec<f32> {
  state.memory_history(seconds)
}

///
/// ## Get GPU usage history
///
/// - param state: shared `HistoryStore`.
/// - param gpu_id: `String` GPU identifier (e.g. "nvapi:0")
/// - param seconds: `u32` Number of seconds to retrieve
///
#[command]
#[specta::specta]
pub fn get_gpu_usage_history(
  state: tauri::State<'_, Arc<HistoryStore>>,
  gpu_id: String,
  seconds: u32,
) -> Vec<f32> {
  state.gpu_history(&gpu_id, seconds)
}

///
/// ## Get network information
///
#[command]
#[specta::specta]
pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  use crate::services::network_service;

  network_service::fetch_network_info()
}

///
/// ## Get latest Storage Health records for the dashboard
///
#[command]
#[specta::specta]
pub async fn get_storage_health_latest_records()
-> Result<Vec<models::hardware::StorageHealthRecord>, String> {
  use crate::services::hardware_service;

  hardware_service::get_storage_health_latest_records()
    .await
    .map(|records| records.into_iter().map(Into::into).collect())
    .map_err(|e| format!("Failed to fetch latest storage health records: {e}"))
}

///
/// ## Get Live Storage Health signals (on demand, never persisted — ADR 0006)
///
/// Reads the startup-cached device list with one `DeviceIoControl` query
/// per device; repeated calls within the core-side minimum interval
/// return the last reading. Returns an empty list when
/// `storageHealth.enabled` is off or on platforms without the cheap
/// native read path (displays fall back to the daily records).
///
#[command]
#[specta::specta]
pub async fn get_live_storage_health(
  state: tauri::State<'_, settings::AppState>,
  workers: tauri::State<'_, crate::workers::WorkersState>,
) -> Result<Vec<models::hardware::LiveStorageHealth>, String> {
  use crate::services::hardware_service;

  let enabled = state.core_settings.lock().unwrap().storage_health.enabled;
  if !enabled {
    return Ok(Vec::new());
  }

  let collector = workers.live_storage_health.lock().unwrap().clone();
  hardware_service::get_live_storage_health(collector)
    .await
    .map(|signals| signals.into_iter().map(Into::into).collect())
}

///
/// ## Re-detect Storage Devices
///
/// Re-detects connected storage devices, refreshes the Live Storage
/// Health cache, collects current Storage Health signals, updates
/// today's Storage Health Records, and returns the latest active records
/// for the dashboard.
///
#[command]
#[specta::specta]
pub async fn refresh_storage_devices(
  state: tauri::State<'_, settings::AppState>,
  workers: tauri::State<'_, crate::workers::WorkersState>,
  guidance_state: tauri::State<'_, Arc<ExternalComponentGuidanceState>>,
) -> Result<Vec<models::hardware::StorageHealthRecord>, String> {
  use crate::services::hardware_service;

  let (retention_days, identity_hash_key) = {
    let settings = state.core_settings.lock().unwrap();
    if !settings.storage_health.enabled {
      return Ok(Vec::new());
    }
    let identity_hash_key = settings.storage_health_identity.hash_key_bytes()?;
    (settings.storage_health.retention_days, identity_hash_key)
  };

  let collector = workers.live_storage_health.lock().unwrap().clone();
  let guidance_state = Arc::clone(guidance_state.inner());
  let guidance_sink: hardviz_core::persistence::ExternalComponentGuidanceSink =
    Arc::new(move |candidates| {
      guidance_state.record_candidates(candidates);
    });

  hardware_service::refresh_storage_devices(
    retention_days,
    identity_hash_key,
    collector,
    Some(guidance_sink),
  )
  .await
  .map(|records| records.into_iter().map(Into::into).collect())
}

///
/// ## Get archived CPU/RAM records
///
#[command]
#[specta::specta]
pub async fn get_data_archive_records(
  hardware_type: DataArchiveHardwareType,
  data_stats: ArchiveDataStats,
  start: String,
  end: String,
) -> Result<Vec<ArchiveRecord>, String> {
  use crate::services::archive_history_service;

  let start = normalize_datetime_string(&start)?;
  let end = normalize_datetime_string(&end)?;

  archive_history_service::fetch_data_archive_records(
    hardware_type.column(data_stats),
    &start,
    &end,
  )
  .await
  .map(|records| records.into_iter().map(Into::into).collect())
}

///
/// ## Get archived GPU records
///
#[command]
#[specta::specta]
pub async fn get_gpu_archive_records(
  data_type: GpuArchiveDataType,
  data_stats: ArchiveDataStats,
  gpu_name: String,
  start: String,
  end: String,
) -> Result<Vec<ArchiveRecord>, String> {
  use crate::services::archive_history_service;

  let start = normalize_datetime_string(&start)?;
  let end = normalize_datetime_string(&end)?;

  archive_history_service::fetch_gpu_archive_records(
    data_type.column(data_stats),
    &gpu_name,
    &start,
    &end,
  )
  .await
  .map(|records| records.into_iter().map(Into::into).collect())
}

///
/// ## Get archived process stats for a period ending at `end_at`
///
#[command]
#[specta::specta]
pub async fn get_process_stats(
  period: u32,
  end_at: String,
) -> Result<Vec<ProcessStatRecord>, String> {
  use crate::services::archive_history_service;

  let end_at = parse_datetime(&end_at)?;
  let adjusted_end_at =
    end_at - chrono::Duration::seconds(HARDWARE_ARCHIVE_INTERVAL_SECONDS as i64);
  let start_time = adjusted_end_at - chrono::Duration::minutes(period as i64);
  let start = format_datetime(start_time);
  let end = format_datetime(adjusted_end_at);

  archive_history_service::fetch_process_stats(&start, &end)
    .await
    .map(|records| records.into_iter().map(Into::into).collect())
}

///
/// ## Get archived process stats between two timestamps
///
#[command]
#[specta::specta]
pub async fn get_process_stats_in_period(
  start: String,
  end: String,
) -> Result<Vec<ProcessStatRecord>, String> {
  use crate::services::archive_history_service;

  let start = normalize_datetime_string(&start)?;
  let end = normalize_datetime_string(&end)?;

  archive_history_service::fetch_process_stats_in_period(&start, &end)
    .await
    .map(|records| records.into_iter().map(Into::into).collect())
}

///
/// ## Get GPU names that have archive records
///
#[command]
#[specta::specta]
pub async fn get_gpu_archive_names() -> Result<Vec<String>, String> {
  use crate::services::archive_history_service;

  archive_history_service::fetch_gpu_archive_names().await
}

fn parse_datetime(input: &str) -> Result<DateTime<Utc>, String> {
  DateTime::parse_from_rfc3339(input)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| format!("Invalid datetime '{input}': {e}"))
}

fn format_datetime(datetime: DateTime<Utc>) -> String {
  datetime.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn normalize_datetime_string(input: &str) -> Result<String, String> {
  parse_datetime(input).map(format_datetime)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn normalize_datetime_string_formats_offset_datetime_as_utc_millis() {
    assert_eq!(
      normalize_datetime_string("2026-06-08T10:30:15.123+09:00").unwrap(),
      "2026-06-08T01:30:15.123Z"
    );
  }

  #[test]
  fn format_datetime_uses_millisecond_precision() {
    let datetime = parse_datetime("2026-06-08T01:30:15Z").unwrap();

    assert_eq!(format_datetime(datetime), "2026-06-08T01:30:15.000Z");
  }

  #[test]
  fn datetime_helpers_report_invalid_input() {
    let input = "not-a-date";

    assert!(parse_datetime(input).unwrap_err().contains(input));
    assert!(
      normalize_datetime_string(input)
        .unwrap_err()
        .contains(input)
    );
  }
}
