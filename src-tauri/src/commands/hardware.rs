use crate::commands::settings;
use crate::enums;
use crate::enums::error::BackendError;
use crate::services;
use crate::services::nvidia_gpu_service;
use crate::services::system_info_service;
use crate::structs;
use crate::structs::hardware::{HardwareMonitorState, NetworkInfo, ProcessInfo, SysInfo};
use crate::utils;
use crate::{log_error, log_internal};
use sysinfo;
use tauri::command;

#[cfg(target_os = "windows")]
use crate::services::wmi_service;

///
/// ## プロセスリストを取得
///
#[command]
#[specta::specta]
pub fn get_process_list(
  state: tauri::State<'_, HardwareMonitorState>,
) -> Vec<ProcessInfo> {
  let mut system = state.system.lock().unwrap();
  let process_cpu_histories = state.process_cpu_histories.lock().unwrap();
  let process_memory_histories = state.process_memory_histories.lock().unwrap();

  system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

  let num_cores = system.cpus().len() as f32;

  system
    .processes()
    .values()
    .map(|process| {
      let pid = process.pid();

      // 5秒間のCPU使用率の平均を計算
      let cpu_usage = if let Some(history) = process_cpu_histories.get(&pid) {
        let len = history.len().min(5); // 最大5秒分のデータ
        let sum: f32 = history.iter().rev().take(len).sum();
        let avg = sum / len as f32;

        let normalized_avg = avg / num_cores;
        (normalized_avg * 10.0).round() / 10.0
      } else {
        0.0 // 履歴がなければ0を返す
      };

      // 5秒間のメモリ使用率の平均を計算
      let memory_usage = if let Some(history) = process_memory_histories.get(&pid) {
        let len = history.len().min(5); // 最大5秒分のデータ
        let sum: f32 = history.iter().rev().take(len).sum();
        let avg = (sum / 1024.0) / len as f32;

        (avg * 10.0).round() / 10.0 // 小数点1桁で丸める
      } else {
        process.memory() as f32 / 1024.0 // KB → MBに変換
      };

      ProcessInfo {
        pid: pid.as_u32() as i32,                            // プロセスID
        name: process.name().to_string_lossy().into_owned(), // プロセス名を取得
        cpu_usage,                                           // 平均CPU使用率
        memory_usage,                                        // 平均メモリ使用率
      }
    })
    .collect()
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
  let system = state.system.lock().unwrap();
  let cpus = system.cpus();
  let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();

  let usage = total_usage / cpus.len() as f32;
  usage.round() as i32
}

#[command]
#[specta::specta]
pub fn get_processors_usage(state: tauri::State<'_, HardwareMonitorState>) -> Vec<f32> {
  let system = state.system.lock().unwrap();
  let cpus = system.cpus();

  // 各プロセッサのCPU使用率を取得
  cpus.iter().map(|cpu| cpu.cpu_usage()).collect()
}

///
/// ## システム情報を取得
///
#[command]
#[specta::specta]
pub async fn get_hardware_info(
  state: tauri::State<'_, HardwareMonitorState>,
) -> Result<SysInfo, String> {
  let cpu_result = system_info_service::get_cpu_info(state.system.lock().unwrap());

  #[cfg(target_os = "windows")]
  let (memory_result, nvidia_gpus_result, amd_gpus_result, intel_gpus_result) = tokio::join!(
    wmi_service::get_memory_info(),
    nvidia_gpu_service::get_nvidia_gpu_info(),
    services::directx_gpu_service::get_amd_gpu_info(),
    services::directx_gpu_service::get_intel_gpu_info(),
  );

  #[cfg(target_os = "linux")]
  let (memory_result, nvidia_gpus_result) = tokio::join!(
    services::memory::get_memory_info_linux(),
    nvidia_gpu_service::get_nvidia_gpu_info(),
  );

  let mut gpus_result = Vec::new();

  #[cfg(target_os = "linux")]
  for card_id in services::gpu_linux::get_all_card_ids() {
    match services::gpu_linux::detect_gpu_vendor(card_id) {
      services::gpu_linux::GpuVendor::Amd => {
        if let Ok(info) = services::amd_gpu_linux::get_amd_graphic_info(card_id).await {
          gpus_result.push(info);
        }
      }
      services::gpu_linux::GpuVendor::Intel => {
        if let Ok(info) = services::intel_gpu_linux::get_intel_graphic_info(card_id).await
        {
          gpus_result.push(info);
        }
      }
      _ => {}
    }
  }

  // NVIDIA の結果を確認して結合
  match nvidia_gpus_result {
    Ok(nvidia_gpus) => gpus_result.extend(nvidia_gpus),
    Err(e) => log_error!("nvidia_error", "get_all_gpu_info", Some(e)),
  }

  // AMD の結果を確認して結合
  #[cfg(target_os = "windows")]
  match amd_gpus_result {
    Ok(amd_gpus) => gpus_result.extend(amd_gpus),
    Err(e) => log_error!("amd_error", "get_all_gpu_info", Some(e)),
  }

  // Intel の結果を確認して結合
  #[cfg(target_os = "windows")]
  match intel_gpus_result {
    Ok(intel_gpus) => gpus_result.extend(intel_gpus),
    Err(e) => log_error!("intel_error", "get_all_gpu_info", Some(e)),
  }

  let storage_info = system_info_service::get_storage_info()?;

  let sys_info = SysInfo {
    cpu: cpu_result.ok(),
    memory: memory_result.ok(),
    gpus: Some(gpus_result),
    storage: storage_info,
  };

  // すべての情報が失敗した場合にのみエラーメッセージを返す
  if sys_info.cpu.is_none() && sys_info.memory.is_none() && sys_info.gpus.is_none() {
    Err("Failed to get any hardware info".to_string())
  } else {
    Ok(sys_info)
  }
}

///
/// ## 詳細なメモリ情報を取得（Linux）
///
/// - return: `structs::hardware::MemoryInfo` 詳細なメモリ情報
///
#[command]
#[specta::specta]
pub async fn get_memory_info_detail_linux()
-> Result<structs::hardware::MemoryInfo, String> {
  #[cfg(not(target_os = "linux"))]
  return Err("Detailed memory info is not supported on this OS".to_string());

  #[cfg(target_os = "linux")]
  services::memory::get_memory_info_dmidecode().await
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
  let system = state.system.lock().unwrap();
  let used_memory = system.used_memory() as f64;
  let total_memory = system.total_memory() as f64;

  ((used_memory / total_memory) * 100.0).round() as i32
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
  if let Ok(usage) = nvidia_gpu_service::get_nvidia_gpu_usage().await {
    return Ok((usage * 100.0).round() as i32);
  }

  // NVIDIA APIが失敗した場合、WMIから取得を試みる
  #[cfg(target_os = "windows")]
  match wmi_service::get_gpu_usage_by_device_and_engine("3D").await {
    Ok(usage) => Ok((usage * 100.0).round() as i32),
    Err(e) => Err(format!(
      "Failed to get GPU usage from both NVIDIA API and WMI: {e:?}"
    )),
  }

  #[cfg(target_os = "linux")]
  {
    for card_id in 0..=9 {
      let vendor_path = format!("/sys/class/drm/card{card_id}/device/vendor");
      if !std::path::Path::new(&vendor_path).exists() {
        continue;
      }

      let vendor_id = tokio::fs::read_to_string(&vendor_path)
        .await
        .map_err(|e| format!("Failed to read vendor: {e}"))?
        .trim()
        .to_string();

      match vendor_id.as_str() {
        // AMDのGPUを検出した場合
        "0x1002" => {
          if let Ok(usage) = services::amd_gpu_linux::get_amd_gpu_usage(card_id).await {
            return Ok((usage * 100.0).round() as i32);
          }
        }
        // IntelのGPUを検出した場合
        "0x8086" => {
          if let Ok(usage) = services::intel_gpu_linux::get_intel_gpu_usage(card_id).await
          {
            return Ok((usage * 100.0).round() as i32);
          }
        }
        _ => {}
      }
    }

    Err("Failed to get GPU usage on Linux (non-NVIDIA fallback)".to_string())
  }
}

///
/// ## GPU温度を取得
///
#[command]
#[specta::specta]
pub async fn get_gpu_temperature(
  state: tauri::State<'_, settings::AppState>,
) -> Result<Vec<nvidia_gpu_service::NameValue>, String> {
  let temperature_unit = {
    let config = state.settings.lock().unwrap();
    config.temperature_unit.clone()
  };

  match nvidia_gpu_service::get_nvidia_gpu_temperature().await {
    Ok(temps) => {
      let temps = temps
        .iter()
        .map(|temp| {
          let value = utils::formatter::format_temperature(
            enums::settings::TemperatureUnit::Celsius,
            temperature_unit.clone(),
            temp.value,
          );

          nvidia_gpu_service::NameValue {
            name: temp.name.clone(),
            value,
          }
        })
        .collect();
      Ok(temps)
    }
    Err(e) => Err(format!("Failed to get GPU temperature: {e:?}")),
  }
}

///
/// ## GPUのファン回転数を取得
///
#[command]
#[specta::specta]
pub async fn get_nvidia_gpu_cooler() -> Result<Vec<nvidia_gpu_service::NameValue>, String>
{
  match nvidia_gpu_service::get_nvidia_gpu_cooler_stat().await {
    Ok(temps) => Ok(temps),
    Err(e) => Err(format!("Failed to get GPU cooler status: {e:?}")),
  }
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
  let history = state.cpu_history.lock().unwrap();
  history
    .iter()
    .rev()
    .take(seconds as usize)
    .cloned()
    .collect()
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
  let history = state.memory_history.lock().unwrap();
  history
    .iter()
    .rev()
    .take(seconds as usize)
    .cloned()
    .collect()
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
  let history = state.gpu_history.lock().unwrap();
  history
    .iter()
    .rev()
    .take(seconds as usize)
    .cloned()
    .collect()
}

///
/// ## ネットワーク情報を取得
///
#[command]
#[specta::specta]
pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  #[cfg(target_os = "windows")]
  let result = wmi_service::get_network_info().map_err(|_| BackendError::UnexpectedError);

  #[cfg(target_os = "linux")]
  let result =
    services::ip_linux::get_network_info().map_err(|_| BackendError::UnexpectedError);

  result
}
