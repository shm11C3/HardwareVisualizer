use crate::models;

pub async fn call_libre_hardware_monitor_api(
  settings: &models::settings::Settings,
) -> Result<(), String> {
  use crate::infrastructure::providers::api::libre_hardware_monitor_provider::LibreHardwareMonitorProvider;

  let lhm_settings = settings
    .libre_hardware_monitor_import
    .as_ref()
    .ok_or_else(|| "Libre Hardware Monitor import settings not found".to_string())?;

  let provider = LibreHardwareMonitorProvider::from_settings(lhm_settings)
    .map_err(|e| format!("Failed to create provider: {}", e))?;

  provider
    .test_connection()
    .await
    .map_err(|e| format!("Connection test failed: {}", e))?;

  Ok(())
}
