use crate::enums;
use crate::models::hardware::NameValue;
use crate::models::libre_hardware_monitor_model::{LibreHardwareMonitorNode, SensorType};
use crate::platform::factory::PlatformFactory;

///
/// GPU 使用率 (%) を取得し四捨五入して整数返却
/// 複数 GPU の場合は Platform 実装側のポリシーに依存
///
pub async fn fetch_gpu_usage() -> Result<i32, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  let usage = platform.get_gpu_usage().await?;
  Ok(usage.round() as i32)
}

///
/// GPU 温度一覧を取得する
/// `temperature_unit` はユーザ設定 (Celsius/Fahrenheit 等) を想定
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

/// Prefer LibreHardwareMonitor snapshot when enabled and available; fallback to platform.
pub async fn fetch_gpu_temperature_preferring_lhm(
  temperature_unit: enums::settings::TemperatureUnit,
  lhm_enabled: bool,
  lhm_root: Option<&LibreHardwareMonitorNode>,
) -> Result<Vec<NameValue>, String> {
  // Local, platform-agnostic temperature converter
  fn convert_temperature(
    current_unit: enums::settings::TemperatureUnit,
    unit: enums::settings::TemperatureUnit,
    value: i32,
  ) -> i32 {
    match (current_unit, unit) {
      (
        enums::settings::TemperatureUnit::Celsius,
        enums::settings::TemperatureUnit::Fahrenheit,
      ) => (value * 9 / 5) + 32,
      (
        enums::settings::TemperatureUnit::Fahrenheit,
        enums::settings::TemperatureUnit::Celsius,
      ) => (value - 32) * 5 / 9,
      _ => value,
    }
  }

  if lhm_enabled && let Some(root) = lhm_root {
    let mut temps: Vec<NameValue> = Vec::new();
    for node in root.find_by_sensor_type(&SensorType::Temperature) {
      if node.text.contains("GPU")
        && let Some(val) = node.get_numeric_value()
      {
        let celsius = val.round() as i32;
        let value = convert_temperature(
          enums::settings::TemperatureUnit::Celsius,
          temperature_unit.clone(),
          celsius,
        );
        temps.push(NameValue {
          name: node.text.clone(),
          value,
        });
      }
    }

    if !temps.is_empty() {
      return Ok(temps);
    }
  }

  // Fallback to platform specific provider
  fetch_gpu_temperature(temperature_unit).await
}

///
/// NVIDIA GPU のファン回転数を取得する (未実装)
/// 未来実装予定のため常に Err
///
pub async fn fetch_nvidia_gpu_cooler() -> Result<Vec<NameValue>, String> {
  Err("Failed to get GPU cooler status: This function is not implemented".to_string())
}
