use crate::{log_error, log_internal, log_warn};
use nvapi;
use nvapi::UtilizationDomain;

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
