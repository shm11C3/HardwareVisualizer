use hardviz_core::platform::factory::PlatformFactory;

#[tauri::command]
#[specta::specta]
pub async fn get_sensor_service_status() -> Result<bool, String> {
  let platform = PlatformFactory::shared().map_err(|e| e.to_string())?;
  Ok(platform.sensor_service_status().await)
}
