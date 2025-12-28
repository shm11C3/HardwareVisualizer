#[cfg(target_os = "macos")]
use std::sync::OnceLock;
use tauri::Config;

///
/// Get the Config structure
///
#[cfg(not(target_os = "macos"))]
pub fn get_config() -> Config {
  let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
  context.config().clone()
}

#[cfg(target_os = "macos")]
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Initialize the Config structure (called only once at application startup)
/// Note: Subsequent calls after the first initialization will be ignored.
#[cfg(target_os = "macos")]
pub fn init_config(config: Config) {
  CONFIG
    .set(config)
    .expect("Config already initialized");
}

///
/// Get the Config structure
///
#[cfg(target_os = "macos")]
pub fn get_config() -> Config {
  #[cfg(test)]
  {
    // Return default Config in test environment
    if CONFIG.get().is_none() {
      return Config::default();
    }
  }
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
