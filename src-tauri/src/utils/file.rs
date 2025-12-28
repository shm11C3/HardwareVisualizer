use std::path::PathBuf;

use crate::utils::tauri as tauri_utils;

#[cfg(test)]
use mockall::automock;

// Trait to abstract environment variable access
#[cfg_attr(test, automock)]
pub trait EnvProvider {
  fn get_var(&self, key: &str) -> Result<String, std::env::VarError>;
}

// Actual environment variable access
pub struct RealEnvProvider;

impl EnvProvider for RealEnvProvider {
  fn get_var(&self, key: &str) -> Result<String, std::env::VarError> {
    std::env::var(key)
  }
}

///
/// Get directory name under `AppData/Roaming`
///
#[cfg(target_os = "windows")]
pub fn get_app_data_dir(sub_item: &str) -> PathBuf {
  get_app_data_dir_with_env(&RealEnvProvider, sub_item)
}

#[cfg(target_os = "windows")]
pub fn get_app_data_dir_with_env<E: EnvProvider>(env: &E, sub_item: &str) -> PathBuf {
  // Create directory based on identifier from tauri.conf.json
  let identifier = tauri_utils::get_identifier();

  let app_data = PathBuf::from(env.get_var("APPDATA").unwrap());
  app_data.join(identifier).join(sub_item)
}

///
/// Get directory name under `~/.config/<identifier>` (Linux / macOS)
///
#[cfg(not(target_os = "windows"))]
pub fn get_app_data_dir(sub_item: &str) -> PathBuf {
  get_app_data_dir_with_env(&RealEnvProvider, sub_item)
}

#[cfg(not(target_os = "windows"))]
pub fn get_app_data_dir_with_env<E: EnvProvider>(env: &E, sub_item: &str) -> PathBuf {
  use std::path::Path;

  let identifier = tauri_utils::get_identifier();

  let home = env.get_var("HOME").unwrap_or_else(|_| ".".to_string());
  Path::new(&home)
    .join(".config")
    .join(identifier)
    .join(sub_item)
}
