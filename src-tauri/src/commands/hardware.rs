use crate::commands::settings;
use crate::enums;
use crate::enums::error::BackendError;
use crate::services::directx_gpu_service;
use crate::services::nvidia_gpu_service;
use crate::services::system_info_service;
use crate::services::wmi_service;
use crate::structs::hardware::NetworkInfo;
use crate::structs::hardware::{GraphicInfo, MemoryInfo, StorageInfo};
use crate::utils;
use crate::{log_error, log_internal};
use serde::{Deserialize, Serialize, Serializer};
use specta::Type;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::command;

pub struct AppState {
  pub system: Arc<Mutex<System>>,
  pub cpu_history: Arc<Mutex<VecDeque<f32>>>,
  pub memory_history: Arc<Mutex<VecDeque<f32>>>,
  pub gpu_history: Arc<Mutex<VecDeque<f32>>>,
  pub process_cpu_histories: Arc<Mutex<HashMap<Pid, VecDeque<f32>>>>,
  pub process_memory_histories: Arc<Mutex<HashMap<Pid, VecDeque<f32>>>>,
}

///
/// システム情報の更新頻度（秒）
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1;

///
/// データを保持する期間（秒）
///
const HISTORY_CAPACITY: usize = 60;

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
  pub pid: i32,
  pub name: String,
  #[serde(serialize_with = "serialize_usage")]
  pub cpu_usage: f32,
  #[serde(serialize_with = "serialize_usage")]
  pub memory_usage: f32,
}

fn serialize_usage<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  if x.fract() == 0.0 {
    s.serialize_str(&format!("{:.0}", x)) // 整数のみ
  } else {
    s.serialize_str(&format!("{:.1}", x)) // 小数点以下1桁まで
  }
}

///
/// ## プロセスリストを取得
///
#[command]
#[specta::specta]
pub fn get_process_list(state: tauri::State<'_, AppState>) -> Vec<ProcessInfo> {
  let mut system = state.system.lock().unwrap();
  let process_cpu_histories = state.process_cpu_histories.lock().unwrap();
  let process_memory_histories = state.process_memory_histories.lock().unwrap();

  system.refresh_processes(ProcessesToUpdate::All, true);

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
pub fn get_cpu_usage(state: tauri::State<'_, AppState>) -> i32 {
  let system = state.system.lock().unwrap();
  let cpus = system.cpus();
  let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();

  let usage = total_usage / cpus.len() as f32;
  usage.round() as i32
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SysInfo {
  pub cpu: Option<system_info_service::CpuInfo>,
  pub memory: Option<MemoryInfo>,
  pub gpus: Option<Vec<GraphicInfo>>,
  pub storage: Vec<StorageInfo>,
}

///
/// ## システム情報を取得
///
#[command]
#[specta::specta]
pub async fn get_hardware_info(
  state: tauri::State<'_, AppState>,
) -> Result<SysInfo, String> {
  let cpu_result = system_info_service::get_cpu_info(state.system.lock().unwrap());
  let memory_result = wmi_service::get_memory_info();
  let nvidia_gpus_result = nvidia_gpu_service::get_nvidia_gpu_info().await;
  let amd_gpus_result = directx_gpu_service::get_amd_gpu_info().await;
  let intel_gpus_result = directx_gpu_service::get_intel_gpu_info().await;

  let mut gpus_result = Vec::new();

  // NVIDIA の結果を確認して結合
  match nvidia_gpus_result {
    Ok(nvidia_gpus) => gpus_result.extend(nvidia_gpus),
    Err(e) => log_error!("nvidia_error", "get_all_gpu_info", Some(e)),
  }

  // AMD の結果を確認して結合
  match amd_gpus_result {
    Ok(amd_gpus) => gpus_result.extend(amd_gpus),
    Err(e) => log_error!("amd_error", "get_all_gpu_info", Some(e)),
  }

  // Intel の結果を確認して結合
  match intel_gpus_result {
    Ok(intel_gpus) => gpus_result.extend(intel_gpus),
    Err(e) => log_error!("intel_error", "get_all_gpu_info", Some(e)),
  }

  let storage_info = system_info_service::get_storage_info()?;

  let sys_info = SysInfo {
    cpu: cpu_result.ok(),
    memory: memory_result.ok(),
    gpus: if gpus_result.is_empty() {
      None
    } else {
      Some(gpus_result)
    },
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
/// ## メモリ使用率（%）を取得
///
/// - pram state: `tauri::State<AppState>` アプリケーションの状態
/// - return: `i32` メモリ使用率（%）
///
#[command]
#[specta::specta]
pub fn get_memory_usage(state: tauri::State<'_, AppState>) -> i32 {
  let system = state.system.lock().unwrap();
  let used_memory = system.used_memory() as f64;
  let total_memory = system.total_memory() as f64;

  ((used_memory / total_memory) * 100.0).round() as i32
}

///
/// ## GPU使用率（%）を取得（Nvidia 限定）
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
  match wmi_service::get_gpu_usage_by_device_and_engine("3D").await {
    Ok(usage) => Ok((usage * 100.0).round() as i32),
    Err(e) => Err(format!(
      "Failed to get GPU usage from both NVIDIA API and WMI: {:?}",
      e
    )),
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
    Err(e) => Err(format!("Failed to get GPU temperature: {:?}", e)),
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
    Err(e) => Err(format!("Failed to get GPU cooler status: {:?}", e)),
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
  state: tauri::State<'_, AppState>,
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
  state: tauri::State<'_, AppState>,
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
  state: tauri::State<'_, AppState>,
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
  wmi_service::get_network_info().map_err(|_| BackendError::UnexpectedError)
}

///
/// ## システム情報の初期化
///
/// - param system: `Arc<Mutex<System>>` システム情報
///
/// - `SYSTEM_INFO_INIT_INTERVAL` 秒ごとにCPU使用率とメモリ使用率を更新
///
pub fn initialize_system(
  system: Arc<Mutex<System>>,
  cpu_history: Arc<Mutex<VecDeque<f32>>>,
  memory_history: Arc<Mutex<VecDeque<f32>>>,
  process_cpu_histories: Arc<Mutex<HashMap<Pid, VecDeque<f32>>>>,
  process_memory_histories: Arc<Mutex<HashMap<Pid, VecDeque<f32>>>>,
) {
  thread::spawn(move || loop {
    {
      let mut sys = match system.lock() {
        Ok(s) => s,
        Err(_) => continue, // エラーハンドリング：ロックが破損している場合はスキップ
      };

      sys.refresh_all();

      let cpu_usage = {
        let cpus = sys.cpus();
        let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
        (total_usage / cpus.len() as f32).round()
      };

      let memory_usage = {
        let used_memory = sys.used_memory() as f64;
        let total_memory = sys.total_memory() as f64;
        (used_memory / total_memory * 100.0).round() as f32
      };

      {
        let mut cpu_hist = cpu_history.lock().unwrap();
        if cpu_hist.len() >= HISTORY_CAPACITY {
          cpu_hist.pop_front();
        }
        cpu_hist.push_back(cpu_usage);
      }

      {
        let mut memory_hist = memory_history.lock().unwrap();
        if memory_hist.len() >= HISTORY_CAPACITY {
          memory_hist.pop_front();
        }
        memory_hist.push_back(memory_usage);
      }

      // 各プロセスごとのCPUおよびメモリ使用率を保存
      {
        let mut process_cpu_histories = process_cpu_histories.lock().unwrap();
        let mut process_memory_histories = process_memory_histories.lock().unwrap();

        for (pid, process) in sys.processes() {
          // CPU使用率の履歴を更新
          let cpu_usage = process.cpu_usage();
          let cpu_history = process_cpu_histories.entry(*pid).or_default();

          if cpu_history.len() >= HISTORY_CAPACITY {
            cpu_history.pop_front();
          }
          cpu_history.push_back(cpu_usage);

          // メモリ使用率の履歴を更新
          let memory_usage = process.memory() as f32 / 1024.0; // KB単位からMB単位に変換
          let memory_history = process_memory_histories.entry(*pid).or_default();

          if memory_history.len() >= HISTORY_CAPACITY {
            memory_history.pop_front();
          }
          memory_history.push_back(memory_usage);
        }
      }
    }

    thread::sleep(Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL));
  });
}
