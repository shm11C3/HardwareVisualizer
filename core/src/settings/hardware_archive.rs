use serde::{Deserialize, Deserializer, Serialize};

/// Core-owned hardware archive settings.
///
/// Persisted as the `hardwareArchive` key in the shared `settings.json`.
/// The struct intentionally lives in `hardviz_core` (not the App crate) so
/// that the archive worker and any future Core consumer can read these
/// values without depending on Tauri.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareArchiveSettings {
  pub enabled: bool,
  pub scheduled_data_deletion: bool,
  pub retention_days: u32,
}

impl Default for HardwareArchiveSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      retention_days: 30,
      scheduled_data_deletion: true,
    }
  }
}

impl<'de> Deserialize<'de> for HardwareArchiveSettings {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    #[derive(Deserialize)]
    #[serde(default, rename_all = "camelCase")]
    struct WireSettings {
      enabled: bool,
      scheduled_data_deletion: bool,
      retention_days: Option<u32>,
      refresh_interval_days: Option<u32>,
    }

    impl Default for WireSettings {
      fn default() -> Self {
        let defaults = HardwareArchiveSettings::default();
        Self {
          enabled: defaults.enabled,
          scheduled_data_deletion: defaults.scheduled_data_deletion,
          retention_days: None,
          refresh_interval_days: None,
        }
      }
    }

    let wire = WireSettings::deserialize(deserializer)?;
    Ok(Self {
      enabled: wire.enabled,
      scheduled_data_deletion: wire.scheduled_data_deletion,
      retention_days: wire
        .retention_days
        .or(wire.refresh_interval_days)
        .unwrap_or_else(|| HardwareArchiveSettings::default().retention_days),
    })
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
    assert_eq!(s.retention_days, 30);
  }

  #[test]
  fn missing_fields_fall_back_to_defaults() {
    let s: HardwareArchiveSettings =
      serde_json::from_str(r#"{"enabled": false}"#).unwrap();
    assert!(!s.enabled);
    // Missing fields preserve defaults thanks to #[serde(default)]
    assert!(s.scheduled_data_deletion);
    assert_eq!(s.retention_days, 30);
  }

  #[test]
  fn serializes_in_camel_case() {
    let s = HardwareArchiveSettings::default();
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"retentionDays\""));
    assert!(!json.contains("\"refreshIntervalDays\""));
    assert!(json.contains("\"scheduledDataDeletion\""));
  }

  #[test]
  fn deserializes_legacy_refresh_interval_days() {
    let s: HardwareArchiveSettings =
      serde_json::from_str(r#"{"refreshIntervalDays": 14}"#).unwrap();
    assert_eq!(s.retention_days, 14);
  }

  #[test]
  fn retention_days_takes_precedence_over_legacy_refresh_interval_days() {
    let s: HardwareArchiveSettings =
      serde_json::from_str(r#"{"retentionDays": 90, "refreshIntervalDays": 14}"#)
        .unwrap();
    assert_eq!(s.retention_days, 90);
  }
}
