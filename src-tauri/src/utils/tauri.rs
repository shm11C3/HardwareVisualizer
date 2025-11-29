use std::sync::OnceLock;
use tauri::Config;

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Initialize the Config structure (called only once at application startup)
/// Note: Subsequent calls after the first initialization will be ignored.
pub fn init_config(config: Config) {
  CONFIG.set(config).ok();
}

///
/// Get the Config structure
///
pub fn get_config() -> Config {
  CONFIG.get().expect("Config not initialized").clone()
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
