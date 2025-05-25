use std::path::PathBuf;
use tauri::generate_context;

///
/// `AppData/Roaming` 配下のディレクトリ名を取得
///
#[cfg(target_os = "windows")]
pub fn get_app_data_dir(sub_item: &str) -> PathBuf {
  let context: tauri::Context<tauri::Wry> = generate_context!();

  // tauri.conf.json の identifier に基づいてディレクトリを作成
  let identifier = context.config().identifier.clone();

  let app_data = PathBuf::from(std::env::var("APPDATA").unwrap());
  app_data.join(identifier).join(sub_item)
}

///
/// `~/.config/<identifier>` 配下のディレクトリ名を取得（Linux / macOS）
///
#[cfg(not(target_os = "windows"))]
pub fn get_app_data_dir(sub_item: &str) -> PathBuf {
  use std::path::Path;

  let context: tauri::Context<tauri::Wry> = generate_context!();
  let identifier = context.config().identifier.clone();

  let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
  Path::new(&home)
    .join(".config")
    .join(identifier)
    .join(sub_item)
}
