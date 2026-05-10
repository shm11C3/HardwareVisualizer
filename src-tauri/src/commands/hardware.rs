use crate::commands::settings;
use crate::enums::error::BackendError;
use crate::models;
use crate::models::hardware::{NetworkInfo, ProcessInfo, SysInfo};
use hardviz_core::collector::HistoryStore;
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
/// ## Get latest Storage SMART snapshots for the dashboard
///
#[command]
#[specta::specta]
pub async fn get_storage_smart_latest_snapshots()
-> Result<Vec<models::hardware::StorageSmartDashboardSnapshot>, String> {
  hardviz_core::infrastructure::database::storage_smart::latest_snapshot_records()
    .await
    .map(|records| records.into_iter().map(Into::into).collect())
    .map_err(|e| format!("Failed to fetch latest storage SMART snapshots: {e}"))
}
