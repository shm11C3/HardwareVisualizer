use crate::enums::settings::TemperatureUnit;
use crate::models::hardware::{GraphicInfo, NameValue};

pub async fn get_gpu_usage() -> Result<f32, String> {
  // Fallback implementation for macOS
  Ok(0.0)
}

pub async fn get_gpu_temperature(
  _temperature_unit: TemperatureUnit,
) -> Result<Vec<NameValue>, String> {
  // fallback
  Ok(vec![])
}

pub async fn get_gpu_info() -> Result<Vec<GraphicInfo>, String> {
  Ok(vec![])
}
