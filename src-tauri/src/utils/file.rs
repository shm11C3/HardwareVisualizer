use std::path::PathBuf;
use tauri::generate_context;

#[cfg(test)]
use mockall::automock;

// 環境変数アクセスを抽象化するトレイト
#[cfg_attr(test, automock)]
pub trait EnvProvider {
  fn get_var(&self, key: &str) -> Result<String, std::env::VarError>;
}

// 実際の環境変数アクセス
pub struct RealEnvProvider;

impl EnvProvider for RealEnvProvider {
  fn get_var(&self, key: &str) -> Result<String, std::env::VarError> {
    std::env::var(key)
  }
}

///
/// `AppData/Roaming` 配下のディレクトリ名を取得
///
#[cfg(target_os = "windows")]
pub fn get_app_data_dir(sub_item: &str) -> PathBuf {
  get_app_data_dir_with_env(&RealEnvProvider, sub_item)
}

#[cfg(target_os = "windows")]
pub fn get_app_data_dir_with_env<E: EnvProvider>(env: &E, sub_item: &str) -> PathBuf {
  let context: tauri::Context<tauri::Wry> = generate_context!();

  // tauri.conf.json の identifier に基づいてディレクトリを作成
  let identifier = context.config().identifier.clone();

  let app_data = PathBuf::from(env.get_var("APPDATA").unwrap());
  app_data.join(identifier).join(sub_item)
}

///
/// `~/.config/<identifier>` 配下のディレクトリ名を取得（Linux / macOS）
///
#[cfg(not(target_os = "windows"))]
pub fn get_app_data_dir(sub_item: &str) -> PathBuf {
  get_app_data_dir_with_env(&RealEnvProvider, sub_item)
}

#[cfg(not(target_os = "windows"))]
pub fn get_app_data_dir_with_env<E: EnvProvider>(env: &E, sub_item: &str) -> PathBuf {
  use std::path::Path;

  let context: tauri::Context<tauri::Wry> = generate_context!();
  let identifier = context.config().identifier.clone();

  let home = env.get_var("HOME").unwrap_or_else(|_| ".".to_string());
  Path::new(&home)
    .join(".config")
    .join(identifier)
    .join(sub_item)
}
