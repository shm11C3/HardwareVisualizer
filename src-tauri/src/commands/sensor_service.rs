use hardviz_core::platform::factory::PlatformFactory;
use serde::Serialize;
use specta::Type;

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SensorServiceStatus {
  connected: bool,
  can_configure: bool,
  detail: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_sensor_service_status() -> Result<SensorServiceStatus, String> {
  let platform = PlatformFactory::shared().map_err(|e| e.to_string())?;
  let status = platform.sensor_service_status().await;

  Ok(SensorServiceStatus {
    connected: status.connected,
    can_configure: status.can_configure,
    detail: status.detail,
  })
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
