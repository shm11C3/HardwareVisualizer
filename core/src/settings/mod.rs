//! Core-owned settings.
//!
//! [`CoreSettings`] holds the fields whose values change Core behavior
//! (for example archive workers' enabled/cadence/retention).
//! It is intentionally a strict subset of the on-disk `settings.json` —
//! UI-only fields (`theme`, `language`, `lineGraph*`, `burnInShift*`,
//! `temperatureUnit`, etc.) are owned by the App crate and stay
//! invisible to Core. Both crates can read and write the same JSON file:
//! [`CoreSettings::save_to_path`] merges its own keys into any existing
//! object so App-owned keys are preserved.

pub mod environmental_sensors;
pub mod hardware_archive;
pub mod storage_health;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

pub use environmental_sensors::EnvironmentalSensorSettings;
pub use hardware_archive::HardwareArchiveSettings;
pub use storage_health::{
  STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES, StorageHealthIdentitySettings,
  StorageHealthSettings,
};

/// JSON key under which [`HardwareArchiveSettings`] is persisted.
const HARDWARE_ARCHIVE_KEY: &str = "hardwareArchive";
/// JSON key under which [`StorageHealthSettings`] is persisted.
const STORAGE_HEALTH_KEY: &str = "storageHealth";
/// Legacy JSON key accepted for unreleased Storage Health development builds.
const LEGACY_STORAGE_SMART_KEY: &str = "storageSmart";
/// JSON key under which [`StorageHealthIdentitySettings`] is persisted.
const STORAGE_HEALTH_IDENTITY_KEY: &str = "storageHealthIdentity";
/// JSON key under which [`EnvironmentalSensorSettings`] is persisted.
const ENVIRONMENTAL_SENSORS_KEY: &str = "environmentalSensors";
/// Legacy JSON key accepted for unreleased Storage Health development builds.
const LEGACY_STORAGE_SMART_IDENTITY_KEY: &str = "storageSmartIdentity";

/// Write the keys this version owns into `key`'s section, leaving any
/// other key already in that section untouched.
///
/// The same merge [`CoreSettings::save_to_path`] performs at the top
/// level, one level deeper. At the top level it protects App-owned keys
/// from Core; here it protects keys owned by a *future* version from
/// this one. `environmentalSensors` gains a key per vendor, so a user
/// who runs a newer build, enables a second sensor, then downgrades
/// would otherwise have that choice silently deleted the first time this
/// version saved - and would find it gone on upgrading back.
///
/// A section that is absent or is not an object is replaced outright:
/// there is nothing to preserve, and a non-object there is not a
/// settings section this code can reason about.
///
/// Only the environmental section uses this so far. The older sections
/// still replace wholesale; changing them is a behavior change to
/// working code and belongs in its own commit, not this one.
fn merge_owned_keys_into_section(
  document: &mut serde_json::Map<String, serde_json::Value>,
  key: &str,
  owned: serde_json::Value,
) {
  let serde_json::Value::Object(owned) = owned else {
    return;
  };

  match document.get_mut(key) {
    Some(serde_json::Value::Object(existing)) => existing.extend(owned),
    _ => {
      document.insert(key.to_string(), serde_json::Value::Object(owned));
    }
  }
}

/// Subset of the on-disk settings document that Core consumes.
///
/// Deserializing from a settings file that contains additional App-owned
/// keys is supported: unknown keys are silently ignored, and missing
/// Core keys fall back to their defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CoreSettings {
  pub hardware_archive: HardwareArchiveSettings,
  pub storage_health: StorageHealthSettings,
  pub storage_health_identity: StorageHealthIdentitySettings,
  pub environmental_sensors: EnvironmentalSensorSettings,
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
    if let Some(v) = map
      .get(STORAGE_HEALTH_KEY)
      .or_else(|| map.get(LEGACY_STORAGE_SMART_KEY))
      && let Ok(parsed) = serde_json::from_value::<StorageHealthSettings>(v.clone())
    {
      settings.storage_health = parsed;
    }
    if let Some(v) = map
      .get(STORAGE_HEALTH_IDENTITY_KEY)
      .or_else(|| map.get(LEGACY_STORAGE_SMART_IDENTITY_KEY))
      && let Ok(parsed) =
        serde_json::from_value::<StorageHealthIdentitySettings>(v.clone())
    {
      settings.storage_health_identity = parsed;
    }
    if let Some(v) = map.get(ENVIRONMENTAL_SENSORS_KEY)
      && let Ok(parsed) = serde_json::from_value::<EnvironmentalSensorSettings>(v.clone())
    {
      settings.environmental_sensors = parsed;
    }
    Ok(settings)
  }

  pub fn ensure_storage_health_identity_key(&mut self) -> Result<bool, String> {
    self.storage_health_identity.ensure_hash_key()
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

    // Refuse to silently overwrite a corrupted settings file — App-owned
    // keys live under the same JSON object, so a "best-effort" reset
    // would drop them. Bail out and let the caller surface the error.
    let mut document = match fs::read_to_string(path) {
      Ok(input) => {
        let parsed: serde_json::Value = serde_json::from_str(&input)
          .map_err(|e| format!("Existing settings file is invalid JSON: {e}"))?;
        match parsed {
          serde_json::Value::Object(map) => map,
          _ => {
            return Err("Existing settings file must be a JSON object".to_string());
          }
        }
      }
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
      Err(e) => return Err(format!("Failed to read existing settings file: {e}")),
    };

    document.insert(
      HARDWARE_ARCHIVE_KEY.to_string(),
      serde_json::to_value(&self.hardware_archive)
        .map_err(|e| format!("Failed to serialize hardware archive settings: {e}"))?,
    );
    document.insert(
      STORAGE_HEALTH_KEY.to_string(),
      serde_json::to_value(&self.storage_health)
        .map_err(|e| format!("Failed to serialize storage health settings: {e}"))?,
    );
    document.insert(
      STORAGE_HEALTH_IDENTITY_KEY.to_string(),
      serde_json::to_value(&self.storage_health_identity).map_err(|e| {
        format!("Failed to serialize storage health identity settings: {e}")
      })?,
    );
    merge_owned_keys_into_section(
      &mut document,
      ENVIRONMENTAL_SENSORS_KEY,
      serde_json::to_value(&self.environmental_sensors)
        .map_err(|e| format!("Failed to serialize environmental sensor settings: {e}"))?,
    );
    document.remove(LEGACY_STORAGE_SMART_KEY);
    document.remove(LEGACY_STORAGE_SMART_IDENTITY_KEY);

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
          "retentionDays": 7,
          "scheduledDataDeletion": false
        },
        "storageHealth": {
          "enabled": false,
          "retentionDays": 730
        }
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(!s.hardware_archive.enabled);
    assert_eq!(s.hardware_archive.retention_days, 7);
    assert!(!s.hardware_archive.scheduled_data_deletion);
    assert!(!s.storage_health.enabled);
    assert_eq!(s.storage_health.retention_days, 730);
  }

  /// A settings file written before #2044 carries no
  /// `environmentalSensors` key, and must not start a radio scan on
  /// upgrade.
  #[test]
  fn a_settings_file_without_the_environmental_key_leaves_the_scan_off() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"hardwareArchive": {"enabled": true}}"#).unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(!s.environmental_sensors.switchbot_meter_enabled);
  }

  #[test]
  fn load_reads_an_enabled_switchbot_meter_scan() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "theme": "dark",
        "environmentalSensors": {"switchbotMeterEnabled": true}
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(s.environmental_sensors.switchbot_meter_enabled);
  }

  /// The whole point of the shared document: Core writing its own block
  /// must not cost the user an App-owned preference.
  #[test]
  fn saving_the_environmental_key_preserves_app_owned_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"theme": "dark", "language": "ja"}"#).unwrap();

    let mut settings = CoreSettings::default();
    settings.environmental_sensors.switchbot_meter_enabled = true;
    settings.save_to_path(&path).unwrap();

    let written: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(written["theme"], "dark");
    assert_eq!(written["language"], "ja");
    assert_eq!(
      written["environmentalSensors"]["switchbotMeterEnabled"],
      true
    );

    assert!(
      CoreSettings::load_from_path(&path)
        .unwrap()
        .environmental_sensors
        .switchbot_meter_enabled
    );
  }

  /// Regression: a downgrade must not delete a newer version's setting.
  ///
  /// `environmentalSensors` gains a key per vendor, so a user who runs a
  /// newer build, enables a second sensor there, then downgrades would
  /// have had that choice erased the first time this version saved -
  /// and would find it missing after upgrading back.
  #[test]
  fn saving_preserves_an_unknown_vendor_key_written_by_a_newer_version() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "environmentalSensors": {
          "switchbotMeterEnabled": false,
          "someFutureVendorEnabled": true
        }
      }"#,
    )
    .unwrap();

    let mut settings = CoreSettings::load_from_path(&path).unwrap();
    settings.environmental_sensors.switchbot_meter_enabled = true;
    settings.save_to_path(&path).unwrap();

    let written: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
      written["environmentalSensors"]["switchbotMeterEnabled"], true,
      "the key this version owns is still written"
    );
    assert_eq!(
      written["environmentalSensors"]["someFutureVendorEnabled"], true,
      "a key this version does not know about must survive the save"
    );
  }

  #[test]
  fn saving_creates_the_environmental_section_when_the_file_has_none() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"theme": "dark"}"#).unwrap();

    CoreSettings::default().save_to_path(&path).unwrap();

    let written: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
      written["environmentalSensors"]["switchbotMeterEnabled"],
      false
    );
  }

  /// A section that is not an object carries nothing worth preserving,
  /// and leaving it in place would keep the settings unreadable forever.
  #[test]
  fn saving_replaces_a_malformed_environmental_section() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"environmentalSensors": "not an object"}"#).unwrap();

    let mut settings = CoreSettings::default();
    settings.environmental_sensors.switchbot_meter_enabled = true;
    settings.save_to_path(&path).unwrap();

    assert!(
      CoreSettings::load_from_path(&path)
        .unwrap()
        .environmental_sensors
        .switchbot_meter_enabled
    );
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
        "hardwareArchive": {"enabled": false, "retentionDays": 14, "scheduledDataDeletion": true},
        "storageHealth": {"enabled": true, "retentionDays": 3650}
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(!s.hardware_archive.enabled);
    assert_eq!(s.hardware_archive.retention_days, 14);
    assert!(s.storage_health.enabled);
    assert_eq!(s.storage_health.retention_days, 3650);
  }

  #[test]
  fn load_accepts_legacy_storage_smart_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "storageSmart": {"enabled": false, "retentionDays": 730},
        "storageSmartIdentity": {"hashKey": "v1:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"}
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(!s.storage_health.enabled);
    assert_eq!(s.storage_health.retention_days, 730);
    assert_eq!(
      s.storage_health_identity.hash_key,
      "v1:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    );
  }

  #[test]
  fn load_prefers_new_storage_health_keys_over_legacy_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "storageHealth": {"enabled": true, "retentionDays": 3650},
        "storageSmart": {"enabled": false, "retentionDays": 730}
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(s.storage_health.enabled);
    assert_eq!(s.storage_health.retention_days, 3650);
  }

  #[test]
  fn load_accepts_legacy_hardware_archive_refresh_interval_days() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "hardwareArchive": {"enabled": false, "refreshIntervalDays": 21, "scheduledDataDeletion": true}
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    assert!(!s.hardware_archive.enabled);
    assert_eq!(s.hardware_archive.retention_days, 21);
    assert!(s.hardware_archive.scheduled_data_deletion);
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
        "hardwareArchive": {"enabled": true, "refreshIntervalDays": 30, "scheduledDataDeletion": true},
        "storageHealth": {"enabled": true, "retentionDays": 1825}
      }"#,
    )
    .unwrap();

    let mut s = CoreSettings::load_from_path(&path).unwrap();
    s.hardware_archive.enabled = false;
    s.hardware_archive.retention_days = 90;
    s.storage_health.retention_days = 3650;
    s.save_to_path(&path).unwrap();

    let written: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(written.get("language").and_then(|v| v.as_str()), Some("ja"));
    assert_eq!(written.get("theme").and_then(|v| v.as_str()), Some("light"));
    let hw = written.get("hardwareArchive").unwrap();
    assert_eq!(hw.get("enabled").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(hw.get("retentionDays").and_then(|v| v.as_u64()), Some(90));
    assert!(hw.get("refreshIntervalDays").is_none());
    let storage = written.get("storageHealth").unwrap();
    assert_eq!(
      storage.get("retentionDays").and_then(|v| v.as_u64()),
      Some(3650)
    );
    assert!(written.get("storageSmart").is_none());
  }

  #[test]
  fn save_removes_legacy_storage_smart_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
      &path,
      r#"{
        "storageSmart": {"enabled": false, "retentionDays": 730},
        "storageSmartIdentity": {"hashKey": "v1:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"}
      }"#,
    )
    .unwrap();

    let s = CoreSettings::load_from_path(&path).unwrap();
    s.save_to_path(&path).unwrap();

    let written: serde_json::Value =
      serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(written.get("storageHealth").is_some());
    assert!(written.get("storageHealthIdentity").is_some());
    assert!(written.get("storageSmart").is_none());
    assert!(written.get("storageSmartIdentity").is_none());
  }

  #[test]
  fn save_refuses_to_overwrite_corrupted_existing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, "this is not json").unwrap();

    let s = CoreSettings::default();
    let err = s.save_to_path(&path).expect_err("must refuse to overwrite");
    assert!(err.contains("invalid JSON"), "unexpected error: {err}");

    // The on-disk content must be left untouched so a human can inspect /
    // recover it instead of losing App-owned keys to a silent reset.
    assert_eq!(fs::read_to_string(&path).unwrap(), "this is not json");
  }

  #[test]
  fn save_refuses_when_existing_file_is_not_an_object() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, "[1, 2, 3]").unwrap();

    let s = CoreSettings::default();
    let err = s
      .save_to_path(&path)
      .expect_err("must refuse non-object root");
    assert!(err.contains("JSON object"), "unexpected error: {err}");
    assert_eq!(fs::read_to_string(&path).unwrap(), "[1, 2, 3]");
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
    assert!(written.get("storageHealth").is_some());
    assert!(written.get("storageHealthIdentity").is_some());
  }

  #[test]
  fn round_trip_preserves_values() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");

    let s = CoreSettings {
      hardware_archive: HardwareArchiveSettings {
        enabled: false,
        scheduled_data_deletion: false,
        retention_days: 365,
      },
      storage_health: StorageHealthSettings {
        enabled: true,
        retention_days: 3_650,
      },
      storage_health_identity: StorageHealthIdentitySettings {
        hash_key: "v1:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
          .to_string(),
      },
      environmental_sensors: EnvironmentalSensorSettings {
        switchbot_meter_enabled: true,
        switchbot_meter_device: Some("PeripheralId(AA:BB:CC:DD:A1:B2)".to_string()),
      },
    };
    s.save_to_path(&path).unwrap();
    let loaded = CoreSettings::load_from_path(&path).unwrap();
    assert_eq!(s, loaded);
  }
}
