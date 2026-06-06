use hardviz_core::settings::HardwareArchiveSettings as CoreHardwareArchiveSettings;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Wire-format mirror of [`hardviz_core::settings::HardwareArchiveSettings`].
///
/// The canonical definition lives in `hardviz_core::settings` so the
/// archive worker (and any future Core consumer) doesn't need to know
/// about Tauri or specta. This App-side struct exists only because the
/// frontend wire format requires `specta::Type`.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(default, rename_all = "camelCase")]
pub struct HardwareArchiveSettings {
  pub enabled: bool,
  pub scheduled_data_deletion: bool,
  pub retention_days: u32,
}

impl Default for HardwareArchiveSettings {
  fn default() -> Self {
    CoreHardwareArchiveSettings::default().into()
  }
}

impl From<CoreHardwareArchiveSettings> for HardwareArchiveSettings {
  fn from(value: CoreHardwareArchiveSettings) -> Self {
    Self {
      enabled: value.enabled,
      scheduled_data_deletion: value.scheduled_data_deletion,
      retention_days: value.retention_days,
    }
  }
}

// Only the Core → App direction has a real consumer (the
// `ClientSettings` response). The reverse direction is unnecessary
// because mutation goes through `commands::settings::update_core_settings`
// which operates on `CoreSettings` directly, never converting from the
// wire mirror back into the Core type.
