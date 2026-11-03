use hardviz_core::platform::factory::PlatformFactory;
use serde::Serialize;
use specta::Type;

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SensorServiceStatus {
  connected: bool,
  has_readings: bool,
  can_configure: bool,
  detail: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_sensor_service_status() -> Result<SensorServiceStatus, String> {
  tauri::async_runtime::spawn_blocking(|| {
    let platform = PlatformFactory::shared().map_err(|e| e.to_string())?;
    let status = platform.sensor_service_status();

    Ok(SensorServiceStatus {
      connected: status.connected,
      has_readings: status.has_readings,
      can_configure: status.can_configure,
      detail: status.detail,
    })
  })
  .await
  .map_err(|e| e.to_string())?
}

#[tauri::command]
#[specta::specta]
pub async fn configure_sensor_service() -> Result<(), String> {
  tauri::async_runtime::spawn_blocking(|| {
    let platform = PlatformFactory::shared().map_err(|e| e.to_string())?;
    platform.configure_sensor_service()
  })
  .await
  .map_err(|e| e.to_string())?
}
