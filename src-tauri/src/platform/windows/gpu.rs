use crate::enums;
use crate::infrastructure;
use crate::utils;
use crate::{log_error, log_internal, log_warn};

pub async fn get_gpu_usage() -> Result<(f32, String), String> {
  // 1. NVAPI (NVIDIA)
  if let Ok(usage) =
    infrastructure::providers::nvapi_provider::get_nvidia_gpu_usage().await
  {
    return Ok(((usage * 100.0).round(), "NVAPI".to_string()));
  }

  // 2. ADL (AMD) – dedicated API for AMD GPUs
  if infrastructure::providers::adl_provider::is_available() {
    if let Ok(usage) = infrastructure::providers::adl_provider::get_amd_gpu_usage().await
    {
      return Ok((usage.round(), "ADL".to_string()));
    }
    log_warn!(
      "adl_fallback",
      "get_gpu_usage",
      Some("ADL usage query failed, falling back to WMI")
    );
  }

  // 3. WMI (generic fallback)
  match infrastructure::providers::wmi_provider::query_gpu_usage_by_device_and_engine(
    "3D",
  )
  .await
  {
    Ok(usage) => Ok(((usage * 100.0).round(), "WMI".to_string())),
    Err(e) => Err(format!(
      "Failed to get GPU usage from NVIDIA API, AMD ADL, and WMI: {e:?}"
    )),
  }
}

pub async fn get_gpu_temperature(
  temperature_unit: enums::settings::TemperatureUnit,
) -> Result<Vec<crate::models::hardware::NameValue>, String> {
  let mut all_temps: Vec<crate::models::hardware::NameValue> = Vec::new();

  // 1. NVAPI (NVIDIA)
  if let Ok(nvidia_temps) =
    infrastructure::providers::nvapi_provider::get_nvidia_gpu_temperature().await
  {
    for temp in &nvidia_temps {
      all_temps.push(crate::models::hardware::NameValue {
        name: temp.name.clone(),
        value: utils::formatter::format_temperature(
          enums::settings::TemperatureUnit::Celsius,
          temperature_unit.clone(),
          temp.value,
        ),
      });
    }
  }

  // 2. ADL (AMD)
  if let Ok(amd_temps) =
    infrastructure::providers::adl_provider::get_amd_gpu_temperatures().await
  {
    for temp in &amd_temps {
      all_temps.push(crate::models::hardware::NameValue {
        name: temp.name.clone(),
        value: utils::formatter::format_temperature(
          enums::settings::TemperatureUnit::Celsius,
          temperature_unit.clone(),
          temp.value,
        ),
      });
    }
  }

  if all_temps.is_empty() {
    Err("Failed to get GPU temperature from any provider".to_string())
  } else {
    Ok(all_temps)
  }
}

pub async fn get_gpu_info() -> Result<Vec<crate::models::hardware::GraphicInfo>, String> {
  let (nvidia_res, amd_res, intel_res) = tokio::join!(
    infrastructure::providers::nvapi_provider::get_nvidia_gpu_info(),
    infrastructure::providers::directx::get_amd_gpu_info(),
    infrastructure::providers::directx::get_intel_gpu_info(),
  );

  fn append(
    tag: &str,
    result: Result<Vec<crate::models::hardware::GraphicInfo>, String>,
    acc: &mut Vec<crate::models::hardware::GraphicInfo>,
  ) {
    match result {
      Ok(list) => acc.extend(list),
      Err(e) => log_error!(tag, "get_gpu_info", Some(e.clone())),
    }
  }

  let mut gpus = Vec::new();

  append("nvidia_error", nvidia_res, &mut gpus);
  append("amd_error", amd_res, &mut gpus);
  append("intel_error", intel_res, &mut gpus);

  Ok(gpus)
}
