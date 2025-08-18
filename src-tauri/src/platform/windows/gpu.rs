use crate::enums;
use crate::infrastructure;
use crate::utils;
use crate::{log_error, log_internal};

pub async fn get_gpu_usage() -> Result<f32, String> {
  // NVAPI から取得できた場合は NVAPI 優先
  if let Ok(usage) =
    infrastructure::providers::nvapi_provider::get_nvidia_gpu_usage().await
  {
    return Ok((usage * 100.0).round());
  }

  match infrastructure::providers::wmi_provider::query_gpu_usage_by_device_and_engine(
    "3D",
  )
  .await
  {
    Ok(usage) => Ok((usage * 100.0).round()),
    Err(e) => Err(format!(
      "Failed to get GPU usage from both NVIDIA API and WMI: {e:?}"
    )),
  }
}

pub async fn get_gpu_temperature(
  temperature_unit: enums::settings::TemperatureUnit,
) -> Result<Vec<crate::models::hardware::NameValue>, String> {
  match infrastructure::providers::nvapi_provider::get_nvidia_gpu_temperature().await {
    Ok(temps) => {
      let temps = temps
        .iter()
        .map(|temp| {
          let value = utils::formatter::format_temperature(
            enums::settings::TemperatureUnit::Celsius,
            temperature_unit.clone(),
            temp.value,
          );

          crate::models::hardware::NameValue {
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
