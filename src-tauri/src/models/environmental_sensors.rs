use hardviz_core::settings::EnvironmentalSensorSettings as CoreEnvironmentalSensorSettings;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Wire-format mirror of
/// [`hardviz_core::settings::EnvironmentalSensorSettings`] (#2044).
///
/// The canonical definition lives in `hardviz_core::settings` so the
/// ambient provider registration doesn't need to know about Tauri or
/// specta. This App-side struct exists only because the frontend wire
/// format requires `specta::Type`.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(default, rename_all = "camelCase")]
pub struct EnvironmentalSensorSettings {
  pub switchbot_meter_enabled: bool,
}

impl Default for EnvironmentalSensorSettings {
  fn default() -> Self {
    CoreEnvironmentalSensorSettings::default().into()
  }
}

impl From<CoreEnvironmentalSensorSettings> for EnvironmentalSensorSettings {
  fn from(value: CoreEnvironmentalSensorSettings) -> Self {
    Self {
      switchbot_meter_enabled: value.switchbot_meter_enabled,
    }
  }
}

// Only the Core → App direction has a real consumer (the
// `ClientSettings` response). Mutation goes through
// `commands::settings::update_core_settings`, which operates on
// `CoreSettings` directly rather than converting the wire mirror back.

#[cfg(test)]
mod tests {
  use super::*;

  /// The wire mirror must not quietly disagree with Core about the
  /// default, or the settings screen would show a scan the app is not
  /// running.
  #[test]
  fn the_wire_default_matches_the_core_default() {
    assert_eq!(
      EnvironmentalSensorSettings::default().switchbot_meter_enabled,
      CoreEnvironmentalSensorSettings::default().switchbot_meter_enabled
    );
  }

  #[test]
  fn an_enabled_scan_survives_the_conversion() {
    let core = CoreEnvironmentalSensorSettings {
      switchbot_meter_enabled: true,
    };
    let wire: EnvironmentalSensorSettings = core.into();
    assert!(wire.switchbot_meter_enabled);
  }
}
