use serde::{Deserialize, Serialize};

/// Core-owned hardware archive settings.
///
/// Persisted as the `hardwareArchive` key in the shared `settings.json`.
/// The struct intentionally lives in `hwviz_core` (not the App crate) so
/// that the archive worker and any future Core consumer can read these
/// values without depending on Tauri.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HardwareArchiveSettings {
  pub enabled: bool,
  pub scheduled_data_deletion: bool,
  pub refresh_interval_days: u32,
}

impl Default for HardwareArchiveSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      refresh_interval_days: 30,
      scheduled_data_deletion: true,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn defaults_match_documented_values() {
    let s = HardwareArchiveSettings::default();
    assert!(s.enabled);
    assert!(s.scheduled_data_deletion);
    assert_eq!(s.refresh_interval_days, 30);
  }

  #[test]
  fn missing_fields_fall_back_to_defaults() {
    let s: HardwareArchiveSettings =
      serde_json::from_str(r#"{"enabled": false}"#).unwrap();
    assert!(!s.enabled);
    // Missing fields preserve defaults thanks to #[serde(default)]
    assert!(s.scheduled_data_deletion);
    assert_eq!(s.refresh_interval_days, 30);
  }

  #[test]
  fn serializes_in_camel_case() {
    let s = HardwareArchiveSettings::default();
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"refreshIntervalDays\""));
    assert!(json.contains("\"scheduledDataDeletion\""));
  }
}
