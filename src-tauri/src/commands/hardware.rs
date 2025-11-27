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
/// - param state: `tauri::State<AppState>` Application state
/// - return: `i32` GPU usage (%)
///
#[command]
#[specta::specta]
pub async fn get_gpu_usage() -> Result<i32, String> {
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
/// ## Get GPU fan speed
///
#[command]
#[specta::specta]
pub async fn get_nvidia_gpu_cooler() -> Result<Vec<models::hardware::NameValue>, String> {
  use crate::services::gpu_service;

  gpu_service::fetch_nvidia_gpu_cooler().await
}

///
/// ## Get CPU usage history
///
/// - param state: `tauri::State<AppState>` Application state
/// - param seconds: `u32` Number of seconds to retrieve
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
/// - param state: `tauri::State<AppState>` Application state
/// - param seconds: `u32` Number of seconds to retrieve
///
#[command]
#[specta::specta]
pub fn get_gpu_usage_history(
  state: tauri::State<'_, HardwareMonitorState>,
  seconds: u32,
) -> Vec<f32> {
  use crate::services::monitoring_service;

  monitoring_service::gpu_usage_history(&state, seconds)
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
