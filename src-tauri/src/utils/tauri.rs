use std::sync::OnceLock;

use serde_json::Value;

const RELEASE_IDENTIFIER: &str = "HardwareVisualizer";
const DEV_IDENTIFIER: &str = "HardwareVisualizerDev";
const DEV_PRODUCT_NAME: &str = "HardwareVisualizerDev";
const DEV_BINARY_NAME: &str = "hardware-visualizer-dev";

fn parse_tauri_conf(raw: &str) -> Value {
  serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn release_identifier_from(conf: &Value) -> String {
  conf
    .get("identifier")
    .and_then(|v| v.as_str())
    .unwrap_or(RELEASE_IDENTIFIER)
    .to_string()
}

fn runtime_identifier_from(conf: &Value) -> String {
  if cfg!(debug_assertions) {
    DEV_IDENTIFIER.to_string()
  } else {
    release_identifier_from(conf)
  }
}

fn app_version_from(conf: &Value) -> String {
  conf
    .get("version")
    .and_then(|v| v.as_str())
    .unwrap_or(env!("CARGO_PKG_VERSION"))
    .to_string()
}

fn tauri_conf() -> &'static Value {
  static CONF: OnceLock<Value> = OnceLock::new();
  CONF.get_or_init(|| {
    let raw = include_str!("../../tauri.conf.json");
    parse_tauri_conf(raw)
  })
}

///
/// Get the app identifier used for app-owned data paths.
///
/// Debug builds intentionally use a development-only identifier so local
/// settings, databases, stores, logs, and single-instance state do not collide
/// with the installed release app.
///
pub fn get_identifier() -> String {
  runtime_identifier_from(tauri_conf())
}

///
/// Apply runtime-only app identity overrides that cannot be expressed in
/// `tauri.conf.json` conditionally.
///
pub fn apply_runtime_config(config: &mut tauri::Config) {
  if cfg!(debug_assertions) {
    config.identifier = DEV_IDENTIFIER.to_string();
    config.product_name = Some(DEV_PRODUCT_NAME.to_string());
    config.main_binary_name = Some(DEV_BINARY_NAME.to_string());

    for window in &mut config.app.windows {
      if window.title == RELEASE_IDENTIFIER {
        window.title = DEV_PRODUCT_NAME.to_string();
      }
    }
  }
}

///
/// Get application version from `src-tauri/tauri.conf.json`.
///
pub fn get_app_version() -> String {
  app_version_from(tauri_conf())
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

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
  fn test_get_app_version_fallback_when_missing_or_invalid_type() {
    let expected = env!("CARGO_PKG_VERSION").to_string();

    assert_eq!(app_version_from(&json!({})), expected);
    assert_eq!(app_version_from(&json!({"version": null})), expected);
    assert_eq!(app_version_from(&json!({"version": 123})), expected);
    assert_eq!(app_version_from(&Value::Null), expected);
  }

  #[test]
  fn test_release_identifier_matches_tauri_conf_json() {
    let conf: serde_json::Value =
      serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    let expected = conf
      .get("identifier")
      .and_then(|v| v.as_str())
      .expect("tauri.conf.json must contain string 'identifier'")
      .to_string();

    assert_eq!(release_identifier_from(&conf), expected);
  }

  #[test]
  fn test_get_identifier_uses_build_flavor() {
    #[cfg(debug_assertions)]
    assert_eq!(get_identifier(), DEV_IDENTIFIER);

    #[cfg(not(debug_assertions))]
    assert_eq!(get_identifier(), RELEASE_IDENTIFIER);
  }

  #[test]
  fn test_release_identifier_fallback_when_missing_or_invalid_type() {
    let expected = RELEASE_IDENTIFIER.to_string();

    assert_eq!(release_identifier_from(&json!({})), expected);
    assert_eq!(
      release_identifier_from(&json!({"identifier": null})),
      expected
    );
    assert_eq!(
      release_identifier_from(&json!({"identifier": 123})),
      expected
    );
    assert_eq!(release_identifier_from(&Value::Null), expected);
  }

  #[test]
  fn test_parse_tauri_conf_invalid_json_falls_back_to_defaults() {
    let conf = parse_tauri_conf("not valid json");
    assert_eq!(conf, Value::Null);

    assert_eq!(app_version_from(&conf), env!("CARGO_PKG_VERSION"));
    assert_eq!(release_identifier_from(&conf), RELEASE_IDENTIFIER);
  }

  #[test]
  fn test_getters_are_stable_across_multiple_calls() {
    let v1 = get_app_version();
    let v2 = get_app_version();
    assert_eq!(v1, v2);

    let id1 = get_identifier();
    let id2 = get_identifier();
    assert_eq!(id1, id2);
  }
}
