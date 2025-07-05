use tauri::Config;

///
/// Config 構造体からアプリケーションのバージョンを取得する
///
pub fn get_app_version(config: &Config) -> String {
  config
    .version
    .clone()
    .unwrap_or_else(|| "unknown".to_string())
}
