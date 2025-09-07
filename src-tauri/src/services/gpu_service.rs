use crate::enums;
use crate::models::hardware::NameValue;
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

///
/// NVIDIA GPU のファン回転数を取得する (未実装)
/// 未来実装予定のため常に Err
///
pub async fn fetch_nvidia_gpu_cooler() -> Result<Vec<NameValue>, String> {
  Err("Failed to get GPU cooler status: This function is not implemented".to_string())
}
