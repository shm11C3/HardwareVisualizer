use hardviz_core::settings::StorageSmartSettings as CoreStorageSmartSettings;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Wire-format mirror of [`hardviz_core::settings::StorageSmartSettings`]. The
/// canonical definition lives in `hardviz_core::settings` so the
/// SMART snapshot worker doesn't need to know about Tauri or specta.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(default, rename_all = "camelCase")]
pub struct StorageSmartSettings {
  pub enabled: bool,
  pub retention_days: u32,
}

impl Default for StorageSmartSettings {
  fn default() -> Self {
    CoreStorageSmartSettings::default().into()
  }
}

impl From<CoreStorageSmartSettings> for StorageSmartSettings {
  fn from(value: CoreStorageSmartSettings) -> Self {
    Self {
      enabled: value.enabled,
      retention_days: value.retention_days,
    }
  }
}
