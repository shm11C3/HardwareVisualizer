use std::sync::OnceLock;

use serde_json::Value;

const RELEASE_IDENTIFIER: &str = "HardwareVisualizer";
const RELEASE_PRODUCT_NAME: &str = "HardwareVisualizer";
const DEV_IDENTIFIER: &str = "HardwareVisualizerDev";
const DEV_PRODUCT_NAME: &str = "HardwareVisualizerDev";
const E2E_IDENTIFIER: &str = "HardwareVisualizerE2E";
const E2E_PRODUCT_NAME: &str = "HardwareVisualizerE2E";
const E2E_ENV: &str = "HARDVIZ_E2E";

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

fn is_e2e_runtime() -> bool {
  std::env::var(E2E_ENV).as_deref() == Ok("1")
}

fn runtime_identifier_for(conf: &Value, is_debug: bool, is_e2e: bool) -> String {
  if is_e2e {
    E2E_IDENTIFIER.to_string()
  } else if is_debug {
    DEV_IDENTIFIER.to_string()
  } else {
    release_identifier_from(conf)
  }
}

fn runtime_identifier_from(conf: &Value) -> String {
  runtime_identifier_for(conf, cfg!(debug_assertions), is_e2e_runtime())
}

fn runtime_product_name_for(is_debug: bool, is_e2e: bool) -> Option<String> {
  if is_e2e {
    Some(E2E_PRODUCT_NAME.to_string())
  } else if is_debug {
    Some(DEV_PRODUCT_NAME.to_string())
  } else {
    None
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
  let is_e2e = is_e2e_runtime();

  if let Some(product_name) = runtime_product_name_for(cfg!(debug_assertions), is_e2e) {
    config.identifier = runtime_identifier_from(tauri_conf());
    config.product_name = Some(product_name.clone());

    for window in &mut config.app.windows {
      if matches!(
        window.title.as_str(),
        RELEASE_PRODUCT_NAME | DEV_PRODUCT_NAME | E2E_PRODUCT_NAME
      ) {
        window.title = product_name.clone();
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
    assert_eq!(get_identifier(), release_identifier_from(tauri_conf()));
  }

  #[test]
  fn test_runtime_identifier_e2e_overrides_debug_and_release_identity() {
    let conf = json!({ "identifier": "ReleaseApp" });

    assert_eq!(runtime_identifier_for(&conf, true, true), E2E_IDENTIFIER);
    assert_eq!(runtime_identifier_for(&conf, false, true), E2E_IDENTIFIER);
  }

  #[test]
  fn test_runtime_identity_falls_back_to_dev_then_release() {
    let conf = json!({ "identifier": "ReleaseApp" });

    assert_eq!(runtime_identifier_for(&conf, true, false), DEV_IDENTIFIER);
    assert_eq!(runtime_identifier_for(&conf, false, false), "ReleaseApp");
  }

  #[test]
  fn test_runtime_product_name_e2e_overrides_debug_identity() {
    assert_eq!(
      runtime_product_name_for(true, true),
      Some(E2E_PRODUCT_NAME.to_string())
    );
    assert_eq!(
      runtime_product_name_for(false, true),
      Some(E2E_PRODUCT_NAME.to_string())
    );
    assert_eq!(
      runtime_product_name_for(true, false),
      Some(DEV_PRODUCT_NAME.to_string())
    );
    assert_eq!(runtime_product_name_for(false, false), None);
  }

  #[test]
  fn test_dev_identity_matches_tauri_dev_conf_json() {
    let conf: serde_json::Value =
      serde_json::from_str(include_str!("../../tauri.dev.conf.json")).unwrap();
    let window_title = conf
      .get("app")
      .and_then(|app| app.get("windows"))
      .and_then(|windows| windows.as_array())
      .and_then(|windows| windows.first())
      .and_then(|window| window.get("title"))
      .and_then(|title| title.as_str())
      .expect("tauri.dev.conf.json must contain app.windows[0].title");

    assert_eq!(
      conf.get("identifier").and_then(|v| v.as_str()),
      Some(DEV_IDENTIFIER)
    );
    assert_eq!(
      conf.get("productName").and_then(|v| v.as_str()),
      Some(DEV_PRODUCT_NAME)
    );
    assert_eq!(window_title, DEV_PRODUCT_NAME);
    assert!(
      conf.get("mainBinaryName").is_none(),
      "tauri.dev.conf.json should inherit the release mainBinaryName"
    );
  }

  #[test]
  fn test_e2e_identity_matches_tauri_e2e_conf_json() {
    let conf: serde_json::Value =
      serde_json::from_str(include_str!("../../tauri.e2e.conf.json")).unwrap();
    let window_title = conf
      .get("app")
      .and_then(|app| app.get("windows"))
      .and_then(|windows| windows.as_array())
      .and_then(|windows| windows.first())
      .and_then(|window| window.get("title"))
      .and_then(|title| title.as_str())
      .expect("tauri.e2e.conf.json must contain app.windows[0].title");

    assert_eq!(
      conf.get("identifier").and_then(|v| v.as_str()),
      Some(E2E_IDENTIFIER)
    );
    assert_eq!(
      conf.get("productName").and_then(|v| v.as_str()),
      Some(E2E_PRODUCT_NAME)
    );
    assert_eq!(window_title, E2E_PRODUCT_NAME);
    assert!(
      conf.get("mainBinaryName").is_none(),
      "tauri.e2e.conf.json should inherit the release mainBinaryName"
    );
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
