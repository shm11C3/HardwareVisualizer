//! Core-owned settings.
//!
//! [`CoreSettings`] holds the fields whose values change Core behavior
//! (currently the hardware archive worker's enabled/cadence/retention).
//! It is intentionally a strict subset of the on-disk `settings.json` —
//! UI-only fields (`theme`, `language`, `lineGraph*`, `burnInShift*`,
//! `temperatureUnit`, etc.) are owned by the App crate and stay
//! invisible to Core. Both crates can read and write the same JSON file:
//! [`CoreSettings::save_to_path`] merges its own keys into any existing
//! object so App-owned keys are preserved.

pub mod hardware_archive;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

pub use hardware_archive::HardwareArchiveSettings;

/// JSON key under which [`HardwareArchiveSettings`] is persisted.
const HARDWARE_ARCHIVE_KEY: &str = "hardwareArchive";

/// Subset of the on-disk settings document that Core consumes.
///
/// Deserializing from a settings file that contains additional App-owned
/// keys is supported: unknown keys are silently ignored, and missing
/// Core keys fall back to their defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CoreSettings {
  pub hardware_archive: HardwareArchiveSettings,
}

impl CoreSettings {
  /// Load `CoreSettings` from a JSON file, returning defaults if the
  /// file does not exist. Field-level errors fall back to defaults so a
  /// malformed App-only key does not block Core startup.
  pub fn load_from_path(path: &Path) -> Result<Self, String> {
    if !path.exists() {
      return Ok(Self::default());
    }

    let input = fs::read_to_string(path)
      .map_err(|e| format!("Failed to read settings file: {e}"))?;

    Self::parse_with_recovery(&input)
  }

  /// Parse Core settings from a JSON string. Falls back to per-field
  /// recovery when a strict deserialize fails so an unrelated invalid
  /// App-owned key doesn't poison the Core view.
  fn parse_with_recovery(input: &str) -> Result<Self, String> {
    if let Ok(parsed) = serde_json::from_str::<Self>(input) {
      return Ok(parsed);
    }

    let value: serde_json::Value = serde_json::from_str(input)
      .map_err(|e| format!("Settings file is not valid JSON: {e}"))?;
    let map = match value {
      serde_json::Value::Object(map) => map,
      _ => return Err("Settings file must be a JSON object".to_string()),
    };

    let mut settings = Self::default();
    if let Some(v) = map.get(HARDWARE_ARCHIVE_KEY)
      && let Ok(parsed) = serde_json::from_value::<HardwareArchiveSettings>(v.clone())
    {
      settings.hardware_archive = parsed;
    }
    Ok(settings)
  }

  /// Persist `CoreSettings` to disk, merging into any existing JSON
  /// object so App-owned keys survive. Writes are atomic via a temp
  /// file + rename.
  pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
    let parent = path
      .parent()
      .ok_or_else(|| "Settings path has no parent directory".to_string())?;

    if !parent.as_os_str().is_empty() && !parent.exists() {
      fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create configuration directory: {e}"))?;
    }

    let mut document = match fs::read_to_string(path) {
      Ok(input) => match serde_json::from_str::<serde_json::Value>(&input) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
      },
      Err(_) => serde_json::Map::new(),
    };

    document.insert(
      HARDWARE_ARCHIVE_KEY.to_string(),
      serde_json::to_value(&self.hardware_archive)
        .map_err(|e| format!("Failed to serialize hardware archive settings: {e}"))?,
    );

    let serialized = serde_json::to_string(&serde_json::Value::Object(document))
      .map_err(|e| format!("Failed to serialize settings: {e}"))?;

    let mut temp_file = tempfile::NamedTempFile::new_in(parent)
      .map_err(|e| format!("Failed to create temporary settings file: {e}"))?;
    temp_file
      .write_all(serialized.as_bytes())
      .map_err(|e| format!("Failed to write temporary settings file: {e}"))?;
    temp_file
      .persist(path)
      .map_err(|e| format!("Failed to persist settings file: {e}"))?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  #[test]
  fn load_from_missing_path_yields_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("missing.json");
    let s = CoreSettings::load_from_path(&path).unwrap();
    assert_eq!(s, CoreSettings::default());
  }

  #[test]
  fn load_ignores_app_only_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "version": "0.1.0",
        "language": "en",
        "theme": "dark",
        "burnInShift": true,
        "temperatureUnit": "F",
        "hardwareArchive": {
          "enabled": false,
          "refreshIntervalDays": 7,
          "scheduledDataDeletion": false
        }
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(!s.hardware_archive.enabled);
    assert_eq!(s.hardware_archive.refresh_interval_days, 7);
    assert!(!s.hardware_archive.scheduled_data_deletion);
  }

  #[test]
  fn load_recovers_when_unrelated_field_is_invalid() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    // `theme` is invalid for the (App-side) Settings struct, but Core
    // doesn't care — it should still parse hardwareArchive correctly.
    fs::write(
      &path,
      r#"{
        "theme": "nonexistent_theme",
        "hardwareArchive": {"enabled": false, "refreshIntervalDays": 14, "scheduledDataDeletion": true}
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(!s.hardware_archive.enabled);
    assert_eq!(s.hardware_archive.refresh_interval_days, 14);
  }

  #[test]
  fn load_returns_err_on_invalid_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, "not json {{{").unwrap();
    assert!(CoreSettings::load_from_path(&path).is_err());
  }

  #[test]
  fn save_preserves_app_only_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "language": "ja",
        "theme": "light",
        "hardwareArchive": {"enabled": true, "refreshIntervalDays": 30, "scheduledDataDeletion": true}
      }"#,
    )
    .unwrap();

    let mut s = CoreSettings::load_from_path(&path).unwrap();
    s.hardware_archive.enabled = false;
    s.hardware_archive.refresh_interval_days = 90;
    s.save_to_path(&path).unwrap();

    let written: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(written.get("language").and_then(|v| v.as_str()), Some("ja"));
    assert_eq!(written.get("theme").and_then(|v| v.as_str()), Some("light"));
    let hw = written.get("hardwareArchive").unwrap();
    assert_eq!(hw.get("enabled").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
      hw.get("refreshIntervalDays").and_then(|v| v.as_u64()),
      Some(90)
    );
  }

  #[test]
  fn save_creates_file_and_parent_when_missing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nested").join("settings.json");
    let s = CoreSettings::default();
    s.save_to_path(&path).unwrap();

    assert!(path.exists());
    let written: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(written.get("hardwareArchive").is_some());
  }

  #[test]
  fn round_trip_preserves_values() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");

    let s = CoreSettings {
      hardware_archive: HardwareArchiveSettings {
        enabled: false,
        scheduled_data_deletion: false,
        refresh_interval_days: 365,
      },
    };
    s.save_to_path(&path).unwrap();
    let loaded = CoreSettings::load_from_path(&path).unwrap();
    assert_eq!(s, loaded);
  }
}
