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
) -> Result<Vec<models::hardware::NameValue>, String> {
  use crate::services::gpu_service;

  let temperature_unit = {
    let config = state.settings.lock().unwrap();
    config.temperature_unit.clone()
  };

  gpu_service::fetch_gpu_temperature(temperature_unit).await
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

#[cfg(test)]
mod tests {
  use std::collections::{HashMap, VecDeque};
  use std::sync::{Arc, Mutex};
  use sysinfo::System;
  use tauri::Manager;

  use super::*;

  ///
  /// Test the get_process_list function
  ///
  #[test]
  fn test_get_process_list() {
    let app = tauri::test::mock_app();

    // Mock
    let mut mock_system = System::new_all();
    mock_system.refresh_all();

    let mut cpu_histories = HashMap::new();
    let mut memory_histories = HashMap::new();

    // Add history for any existing process in the system
    if let Some((pid, _)) = mock_system.processes().iter().next() {
      cpu_histories.insert(*pid, VecDeque::from(vec![50.0; 5]));
      memory_histories.insert(*pid, VecDeque::from(vec![1024.0; 5]));
    }

    let app_state = HardwareMonitorState {
      system: Arc::new(Mutex::new(mock_system)),
      cpu_history: Arc::new(Mutex::new(VecDeque::new())),
      memory_history: Arc::new(Mutex::new(VecDeque::new())),
      gpu_history: Arc::new(Mutex::new(VecDeque::new())),
      process_cpu_histories: Arc::new(Mutex::new(cpu_histories)),
      process_memory_histories: Arc::new(Mutex::new(memory_histories)),
      nv_gpu_usage_histories: Arc::new(Mutex::new(HashMap::new())),
      nv_gpu_temperature_histories: Arc::new(Mutex::new(HashMap::new())),
    };

    app.manage(app_state);

    // Act
    let process_list = get_process_list(app.state());

    // Assert - just verify we get a non-empty list
    assert!(!process_list.is_empty(), "Process list should not be empty");

    // Verify that processes have valid data
    for process in &process_list {
      assert!(process.pid > 0, "Process PID should be positive");
      assert!(!process.name.is_empty(), "Process name should not be empty");
    }
  }
}
