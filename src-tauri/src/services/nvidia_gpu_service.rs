use crate::structs::hardware::GraphicInfo;
use crate::utils::{self};
use crate::{log_debug, log_error, log_internal, log_warn};
use nvapi;
use nvapi::UtilizationDomain;

use tokio::task::JoinError;
use tokio::task::spawn_blocking;

///
/// GPU情報を取得する
///
pub async fn get_nvidia_gpu_info() -> Result<Vec<GraphicInfo>, String> {
  let handle = spawn_blocking(|| {
    log_debug!("start", "get_nvidia_gpu_info", None::<&str>);

    let gpus = match nvapi::PhysicalGpu::enumerate() {
      Ok(gpus) => gpus,
      Err(e) => {
        log_error!(
          "enumerate_failed",
          "get_nvidia_gpu_info",
          Some(e.to_string())
        );
        return Err(e.to_string());
      }
    };

    if gpus.is_empty() {
      log_debug!("not found", "get_nvidia_gpu_info", Some("gpu is not found"));
    }

    let mut gpu_info_list = Vec::new();

    for gpu in gpus.iter() {
      let name = gpu.full_name().unwrap_or("Unknown".to_string());

      // クロック周波数 (MHz) の取得
      let clock_frequencies =
        match gpu.clock_frequencies(nvapi::ClockFrequencyType::Current) {
          Ok(freqs) => freqs,
          Err(e) => {
            log_error!("clock_failed", "get_nvidia_gpu_info", Some(e.to_string()));
            continue;
          }
        };

      let frequency = match clock_frequencies.get(&nvapi::ClockDomain::Graphics) {
        Some(&nvapi::Kilohertz(freq)) => freq as u64,
        None => {
          log_warn!(
            "clock_not_found",
            "get_nvidia_gpu_info",
            Some("Graphics clock not found")
          );
          0 // デフォルト値として 0 を設定
        }
      };

      // メモリサイズ (MB) の取得
      let memory_info = match gpu.memory_info() {
        Ok(info) => info,
        Err(e) => {
          log_error!(
            "memory_info_failed",
            "get_nvidia_gpu_info",
            Some(e.to_string())
          );
          continue;
        }
      };

      let gpu_id = match gpu.gpu_id() {
        Ok(id) => id.to_string(),
        Err(e) => {
          log_error!("gpu_id_failed", "get_nvidia_gpu_info", Some(e.to_string()));
          continue;
        }
      };

      let gpu_info = GraphicInfo {
        id: gpu_id,
        name,
        vendor_name: "NVIDIA".to_string(),
        clock: frequency as u32,
        memory_size: utils::formatter::RoundedKibibytes {
          kibibytes: memory_info.shared,
          precision: 1,
        }
        .to_string(),
        memory_size_dedicated: utils::formatter::RoundedKibibytes {
          kibibytes: memory_info.dedicated,
          precision: 1,
        }
        .to_string(),
      };

      gpu_info_list.push(gpu_info);
    }

    log_debug!("end", "get_nvidia_gpu_info", None::<&str>);

    Ok(gpu_info_list)
  });

  handle.await.map_err(|e: JoinError| {
    log_error!("join_error", "get_nvidia_gpu_info", Some(e.to_string()));
    nvapi::Status::Error.to_string()
  })?
}

///
/// `PhysicalGpu` からGPU使用率を取得する
///
pub fn get_gpu_usage_from_physical_gpu(gpu: &nvapi::PhysicalGpu) -> f32 {
  let usage = match gpu.usages() {
    Ok(usage) => usage,
    Err(e) => {
      log_error!("usages_failed", "get_gpu_usage", Some(e.to_string()));
      return 0.0;
    }
  };

  if let Some(gpu_usage) = usage.get(&UtilizationDomain::Graphics) {
    let usage_f32 = gpu_usage.0 as f32;
    return usage_f32;
  }

  0.0
}

///
/// `PhysicalGpu` からGPU温度を取得する
///
pub fn get_gpu_temperature_from_physical_gpu(gpu: &nvapi::PhysicalGpu) -> i32 {
  let thermal_settings = gpu.thermal_settings(None).map_err(|e| {
    log_warn!(
      "thermal_settings_failed",
      "get_gpu_temperature",
      Some(&format!("{e:?}"))
    );
    0
  });

  if let Ok(thermal_settings) = thermal_settings {
    return thermal_settings[0].current_temperature.0;
  }

  0
}

///
/// `PhysicalGpu` からGPUメモリ使用率を取得する
///
pub fn get_gpu_dedicated_memory_usage_from_physical_gpu(gpu: &nvapi::PhysicalGpu) -> u32 {
  let memory = match gpu.memory_info() {
    Ok(usage) => usage,
    Err(e) => {
      log_error!("usages_failed", "get_gpu_memory", Some(e.to_string()));
      return 0;
    }
  };

  memory.dedicated.0 - memory.dedicated_available_current.0
}
