use std::sync::OnceLock;

use serde_json::Value;

fn tauri_conf() -> &'static Value {
  static CONF: OnceLock<Value> = OnceLock::new();
  CONF.get_or_init(|| {
    let raw = include_str!("../../tauri.conf.json");
    serde_json::from_str(raw).unwrap_or(Value::Null)
  })
}

///
/// Get Tauri bundle identifier from `src-tauri/tauri.conf.json`.
///
pub fn get_identifier() -> String {
  tauri_conf()
    .get("identifier")
    .and_then(|v| v.as_str())
    .unwrap_or("HardwareVisualizer")
    .to_string()
}

///
/// Get application version from `src-tauri/tauri.conf.json`.
///
pub fn get_app_version() -> String {
  tauri_conf()
    .get("version")
    .and_then(|v| v.as_str())
    .unwrap_or(env!("CARGO_PKG_VERSION"))
    .to_string()
}
