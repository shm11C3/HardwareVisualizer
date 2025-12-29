use std::sync::OnceLock;

use serde_json::Value;

fn tauri_conf() -> &'static Value {
  static CONF: OnceLock<Value> = OnceLock::new();
  CONF.get_or_init(|| {
    let raw = include_str!("../../tauri.conf.json");
    serde_json::from_str(raw).expect("Failed to parse embedded tauri.conf.json")
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_app_version_matches_tauri_conf_json() {
    let conf: serde_json::Value =
      serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    let expected = conf
      .get("version")
      .and_then(|v| v.as_str())
      .expect("tauri.conf.json must contain string 'version'")
      .to_string();

    assert_eq!(get_app_version(), expected);
  }

  #[test]
  fn test_get_identifier_matches_tauri_conf_json() {
    let conf: serde_json::Value =
      serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    let expected = conf
      .get("identifier")
      .and_then(|v| v.as_str())
      .expect("tauri.conf.json must contain string 'identifier'")
      .to_string();

    assert_eq!(get_identifier(), expected);
  }
}
