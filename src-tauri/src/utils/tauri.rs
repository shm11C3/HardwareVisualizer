use tauri::Config;

///
/// Config 構造体を取得する
///
pub fn get_config() -> Config {
  let context = tauri::generate_context!();
  context.config().clone()
}

///
/// Config 構造体からアプリケーションのバージョンを取得する
///
pub fn get_app_version(config: &Config) -> String {
  config.package.version.clone().unwrap_or_else(|| "unknown".to_string())
}
