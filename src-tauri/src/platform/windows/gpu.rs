use crate::infrastructure;

pub async fn get_gpu_usage() -> Result<f32, String> {
  // NVAPI から取得できた場合は NVAPI 優先
  if let Ok(usage) = infrastructure::nvapi::get_nvidia_gpu_usage().await {
    return Ok((usage * 100.0).round());
  }

  match infrastructure::wmi_provider::query_gpu_usage_by_device_and_engine("3D").await {
    Ok(usage) => Ok((usage * 100.0).round()),
    Err(e) => Err(format!(
      "Failed to get GPU usage from both NVIDIA API and WMI: {e:?}"
    )),
  }
}
