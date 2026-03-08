use crate::enums;
use crate::models::hardware::{GpuMemoryUsage, GpuUsageResult, NameValue};
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

///
/// Get realtime GPU memory usage (best-effort).
///
/// "Best-effort" means that this function will try to query GPU memory
/// statistics on the current platform, but may return `Ok(None)` on
/// platforms where this information is not available or not yet
/// implemented by the underlying platform layer, instead of treating
/// that as an error.
///
/// This is only supported on platforms whose `Platform` implementation
/// exposes GPU memory metrics; other platforms will simply yield
/// `Ok(None)` to indicate that the feature is unsupported.
///
/// The returned [`GpuMemoryUsage`] contains aggregate GPU memory sizes
/// (for example, total and used memory) expressed in memory units
/// defined by that type (typically mebibytes, MiB). See
/// [`GpuMemoryUsage`] for field-level details.
///
pub async fn fetch_gpu_memory_usage() -> Result<Option<GpuMemoryUsage>, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;

  platform.get_gpu_memory_usage().await
}
