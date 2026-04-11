use crate::commands::settings;
use crate::enums::error::BackendError;
use crate::models;
use crate::models::hardware::{HardwareMonitorState, NetworkInfo, ProcessInfo, SysInfo};
use tauri::command;

///
/// ## Get process list
///
#[command]
#[specta::specta]
pub fn get_process_list(
  state: tauri::State<'_, HardwareMonitorState>,
) -> Vec<ProcessInfo> {
  use crate::services::process_service;

  process_service::collect_process_list(&state)
}

///
/// ## Get CPU usage (%)
///
/// - param state: `tauri::State<AppState>` Application state
/// - return: `i32` CPU usage (%)
///
#[command]
#[specta::specta]
pub fn get_cpu_usage(state: tauri::State<'_, HardwareMonitorState>) -> i32 {
  use crate::services::cpu_service;

  cpu_service::overall_cpu_usage(&state)
}

#[command]
#[specta::specta]
pub fn get_processors_usage(state: tauri::State<'_, HardwareMonitorState>) -> Vec<f32> {
  use crate::services::cpu_service;

  cpu_service::per_cpu_usage(&state)
}

///
/// ## Get system information
///
#[command]
#[specta::specta]
pub async fn get_hardware_info(
  state: tauri::State<'_, HardwareMonitorState>,
) -> Result<SysInfo, String> {
  use crate::services::hardware_service;

  hardware_service::collect_hardware_info(state.inner()).await
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
/// - param state: `tauri::State<AppState>` Application state
/// - return: `i32` Memory usage (%)
///
#[command]
#[specta::specta]
pub fn get_memory_usage(state: tauri::State<'_, HardwareMonitorState>) -> i32 {
  use crate::services::memory_service;

  memory_service::memory_usage_percent(&state)
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
/// ## Get CPU usage history
///
/// - param state: `tauri::State<AppState>` Application state
/// - param seconds: `u32` Number of seconds to retrieve
/// - **Platform support**: Currently implemented only on macOS. On other
///   platforms, or where the underlying APIs are not available, this will
///   return `Ok(None)` instead of failing.
/// - **Best-effort behavior**: If the GPU memory metrics cannot be queried
///   (e.g. unsupported hardware, missing permissions, or transient errors),
///   the function returns `Ok(None)` to indicate that the data is not
///   available, rather than treating this as a hard error.
/// - **Return format**: When successful, the `GpuMemoryUsage` fields contain
///   human-readable, formatted size strings (for example, `"1.5 GB"`) rather
///   than raw byte counts.
///
/// Returns:
/// - `Ok(Some(GpuMemoryUsage))` when GPU memory usage data is available.
/// - `Ok(None)` when the metric is unsupported or currently unavailable.
/// - `Err(String)` only for unexpected internal failures.
///
#[command]
#[specta::specta]
pub async fn get_gpu_memory_usage()
-> Result<Option<models::hardware::GpuMemoryUsage>, String> {
  use crate::services::gpu_service;

  gpu_service::fetch_gpu_memory_usage().await
}

///
/// ## Get realtime GPU memory usage (best-effort)
///
/// This command attempts to retrieve current GPU memory usage information
/// on a best-effort, platform-dependent basis.
///
#[command]
#[specta::specta]
pub fn get_cpu_usage_history(
  state: tauri::State<'_, HardwareMonitorState>,
  seconds: u32,
) -> Vec<f32> {
  use crate::services::monitoring_service;

  monitoring_service::cpu_usage_history(&state, seconds)
}

///
/// ## Get memory usage history
///
/// - param state: `tauri::State<AppState>` Application state
/// - param seconds: `u32` Number of seconds to retrieve
///
#[command]
#[specta::specta]
pub fn get_memory_usage_history(
  state: tauri::State<'_, HardwareMonitorState>,
  seconds: u32,
) -> Vec<f32> {
  use crate::services::monitoring_service;

  monitoring_service::memory_usage_history(&state, seconds)
}

///
/// ## Get GPU usage history
///
/// - param state: `tauri::State<HardwareMonitorState>` Application state
/// - param gpu_id: `String` GPU identifier (e.g. "nvapi:0")
/// - param seconds: `u32` Number of seconds to retrieve
///
#[command]
#[specta::specta]
pub fn get_gpu_usage_history(
  state: tauri::State<'_, HardwareMonitorState>,
  gpu_id: String,
  seconds: u32,
) -> Vec<f32> {
  use crate::services::monitoring_service;

  monitoring_service::gpu_usage_history(&state, &gpu_id, seconds)
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
