use std::sync::OnceLock;

use serde_json::Value;

const DEFAULT_IDENTIFIER: &str = "HardwareVisualizer";

fn parse_tauri_conf(raw: &str) -> Value {
  serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn identifier_from(conf: &Value) -> String {
  conf
    .get("identifier")
    .and_then(|v| v.as_str())
    .unwrap_or(DEFAULT_IDENTIFIER)
    .to_string()
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
/// Get Tauri bundle identifier from `src-tauri/tauri.conf.json`.
///
pub fn get_identifier() -> String {
  identifier_from(tauri_conf())
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

  #[test]
  fn test_get_identifier_fallback_when_missing_or_invalid_type() {
    let expected = DEFAULT_IDENTIFIER.to_string();

    assert_eq!(identifier_from(&json!({})), expected);
    assert_eq!(identifier_from(&json!({"identifier": null})), expected);
    assert_eq!(identifier_from(&json!({"identifier": 123})), expected);
    assert_eq!(identifier_from(&Value::Null), expected);
  }

  #[test]
  fn test_parse_tauri_conf_invalid_json_falls_back_to_defaults() {
    let conf = parse_tauri_conf("not valid json");
    assert_eq!(conf, Value::Null);

    assert_eq!(app_version_from(&conf), env!("CARGO_PKG_VERSION"));
    assert_eq!(identifier_from(&conf), DEFAULT_IDENTIFIER);
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
