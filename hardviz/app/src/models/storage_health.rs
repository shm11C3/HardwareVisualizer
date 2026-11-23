use hardviz_core::settings::StorageHealthSettings as CoreStorageHealthSettings;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Wire-format mirror of [`hardviz_core::settings::StorageHealthSettings`]. The
/// canonical definition lives in `hardviz_core::settings` so the
/// storage health record worker doesn't need to know about Tauri or specta.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(default, rename_all = "camelCase")]
pub struct StorageHealthSettings {
  pub enabled: bool,
  pub retention_days: u32,
}

impl Default for StorageHealthSettings {
  fn default() -> Self {
    CoreStorageHealthSettings::default().into()
  }
}

impl From<CoreStorageHealthSettings> for StorageHealthSettings {
  fn from(value: CoreStorageHealthSettings) -> Self {
    Self {
      enabled: value.enabled,
      retention_days: value.retention_days,
    }
  }
}
