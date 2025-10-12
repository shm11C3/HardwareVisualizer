use crate::commands::settings;
use crate::enums::error::BackendError;
use crate::models;
use crate::models::hardware::{HardwareMonitorState, NetworkInfo, ProcessInfo, SysInfo};
use tauri::command;

///
/// ## プロセスリストを取得
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
/// ## CPU使用率（%）を取得
///
/// - pram state: `tauri::State<AppState>` アプリケーションの状態
/// - return: `i32` CPU使用率（%）
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
/// ## システム情報を取得
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
/// ## 詳細なメモリ情報を取得
///
/// - return: `models::hardware::MemoryInfo` 詳細なメモリ情報
///
#[command]
#[specta::specta]
pub async fn get_memory_info_detail() -> Result<models::hardware::MemoryInfo, String> {
  use crate::services::memory_service;

  memory_service::fetch_memory_detail().await
}

///
/// ## メモリ使用率（%）を取得
///
/// - pram state: `tauri::State<AppState>` アプリケーションの状態
/// - return: `i32` メモリ使用率（%）
///
#[command]
#[specta::specta]
pub fn get_memory_usage(state: tauri::State<'_, HardwareMonitorState>) -> i32 {
  use crate::services::memory_service;

  memory_service::memory_usage_percent(&state)
}

///
/// ## GPU使用率（%）を取得
///
/// - param state: `tauri::State<AppState>` アプリケーションの状態
/// - return: `i32` GPU使用率（%）
///
#[command]
#[specta::specta]
pub async fn get_gpu_usage() -> Result<i32, String> {
  use crate::services::gpu_service;

  gpu_service::fetch_gpu_usage().await
}

///
/// ## GPU温度を取得
///
#[command]
#[specta::specta]
pub async fn get_gpu_temperature(
  state: tauri::State<'_, settings::AppState>,
  lhm_state: tauri::State<
    '_,
    models::libre_hardware_monitor_state::LibreHardwareMonitorDataState,
  >,
) -> Result<Vec<models::hardware::NameValue>, String> {
  use crate::services::gpu_service;

  // Read required settings and latest LHM snapshot once
  let (temperature_unit, lhm_enabled, lhm_root) = {
    let settings_guard = state.settings.lock().unwrap();
    let temperature_unit = settings_guard.temperature_unit.clone();
    let lhm_enabled = settings_guard
      .libre_hardware_monitor_import
      .as_ref()
      .map(|s| s.enabled)
      .unwrap_or(false);
    drop(settings_guard);

    let latest_guard = lhm_state.latest.lock().unwrap();
    let lhm_root = latest_guard.as_ref().cloned();
    (temperature_unit, lhm_enabled, lhm_root)
  };

  gpu_service::fetch_gpu_temperature_preferring_lhm(
    temperature_unit,
    lhm_enabled,
    lhm_root.as_ref(),
  )
  .await
}

///
/// ## GPUのファン回転数を取得
///
#[command]
#[specta::specta]
pub async fn get_nvidia_gpu_cooler() -> Result<Vec<models::hardware::NameValue>, String> {
  use crate::services::gpu_service;

  gpu_service::fetch_nvidia_gpu_cooler().await
}

///
/// ## CPU使用率の履歴を取得
///
/// - param state: `tauri::State<AppState>` アプリケーションの状態
/// - param seconds: `u32` 取得する秒数
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
/// ## メモリ使用率の履歴を取得
///
/// - param state: `tauri::State<AppState>` アプリケーションの状態
/// - param seconds: `u32` 取得する秒数
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
/// ## GPU使用率の履歴を取得
///
/// - param state: `tauri::State<AppState>` アプリケーションの状態
/// - param seconds: `u32` 取得する秒数
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
/// ## ネットワーク情報を取得
///
#[command]
#[specta::specta]
pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  use crate::services::network_service;

  network_service::fetch_network_info()
}
