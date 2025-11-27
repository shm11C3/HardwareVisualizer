use crate::enums;
use crate::models::hardware::NameValue;
use crate::platform::factory::PlatformFactory;

///
/// Get GPU usage (%) and return as rounded integer
/// For multiple GPUs, depends on Platform implementation policy
///
pub async fn fetch_gpu_usage() -> Result<i32, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  let usage = platform.get_gpu_usage().await?;
  Ok(usage.round() as i32)
}

///
/// Get list of GPU temperatures
/// `temperature_unit` assumes user setting (Celsius/Fahrenheit etc.)
///
pub async fn fetch_gpu_temperature(
  temperature_unit: enums::settings::TemperatureUnit,
) -> Result<Vec<NameValue>, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;

  platform
    .get_gpu_temperature(temperature_unit)
    .await
    .map_err(|e| format!("Failed to get GPU temperature: {e:?}"))
}

///
/// Get NVIDIA GPU fan speed (not implemented)
/// Always returns Err as planned for future implementation
///
pub async fn fetch_nvidia_gpu_cooler() -> Result<Vec<NameValue>, String> {
  Err("Failed to get GPU cooler status: This function is not implemented".to_string())
}
