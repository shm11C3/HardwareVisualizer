use serde::{Deserialize, Serialize};

pub const DEFAULT_STORAGE_SMART_RETENTION_DAYS: u32 = 1_825;

/// Core-owned SMART daily snapshot settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StorageSmartSettings {
  pub enabled: bool,
  pub retention_days: u32,
}

impl Default for StorageSmartSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      retention_days: DEFAULT_STORAGE_SMART_RETENTION_DAYS,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn defaults_are_longer_than_insight_defaults() {
    let s = StorageSmartSettings::default();
    assert!(s.enabled);
    assert_eq!(s.retention_days, DEFAULT_STORAGE_SMART_RETENTION_DAYS);
    assert!(s.retention_days > 365);
  }

  #[test]
  fn missing_fields_fall_back_to_defaults() {
    let s: StorageSmartSettings = serde_json::from_str(r#"{"enabled": false}"#).unwrap();
    assert!(!s.enabled);
    assert_eq!(s.retention_days, DEFAULT_STORAGE_SMART_RETENTION_DAYS);
  }

  #[test]
  fn serializes_in_camel_case() {
    let s = StorageSmartSettings::default();
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"retentionDays\""));
  }
}
