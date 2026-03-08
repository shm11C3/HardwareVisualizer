use crate::enums;
use crate::models::hardware::{GpuUsageResult, NameValue};
use crate::platform::factory::PlatformFactory;

///
/// Get GPU usage (%) together with the data-source name
/// For multiple GPUs, depends on Platform implementation policy
///
pub async fn fetch_gpu_usage() -> Result<GpuUsageResult, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  let (usage, source) = platform.get_gpu_usage().await?;
  Ok(GpuUsageResult {
    usage: usage.round() as i32,
    source,
  })
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
