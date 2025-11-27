use tauri::Config;

///
/// Get the Config structure
///
pub fn get_config() -> Config {
  let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
  context.config().clone()
}

///
/// Get application version from Config structure
///
pub fn get_app_version(config: &Config) -> String {
  config
    .version
    .clone()
    .unwrap_or_else(|| "unknown".to_string())
}
