use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct HardwareArchiveSettings {
  pub enabled: bool,
  pub refresh_interval_days: u32,
}
