use hardviz_core::settings::EnvironmentalSensorSettings as CoreEnvironmentalSensorSettings;
use serde::{Deserialize, Serialize};
use specta::Type;

// Kept to a single doc paragraph: tauri-specta renders a blank `///`
// line as a ` * ` line in the generated JSDoc, whose trailing space
// fails the repository's whitespace check on `src/rspc/bindings.ts`.
/// Wire-format mirror of
/// [`hardviz_core::settings::EnvironmentalSensorSettings`] (#2044). The
/// canonical definition lives in `hardviz_core::settings` so the ambient
/// provider registration doesn't need to know about Tauri or specta.
/// This App-side struct exists only because the frontend wire format
/// requires `specta::Type`.
// The remembered meter binding (`switchbot_meter_device`) is Core-only
// and deliberately absent here. The settings screen has no use for it -
// the toggle is the whole interaction - so shipping a device identifier
// into frontend state would widen where it lives for no benefit.
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
      switchbot_meter_device: None,
    };
    let wire: EnvironmentalSensorSettings = core.into();
    assert!(wire.switchbot_meter_enabled);
  }

  /// The remembered binding is Core-only. It must not reach the wire
  /// type, so the device identifier stays out of frontend state.
  #[test]
  fn the_remembered_device_does_not_cross_into_the_wire_type() {
    let core = CoreEnvironmentalSensorSettings {
      switchbot_meter_enabled: true,
      switchbot_meter_device: Some("PeripheralId(AA:BB:CC:DD:A1:B2)".to_string()),
    };

    let wire: EnvironmentalSensorSettings = core.into();
    let json = serde_json::to_string(&wire).unwrap();

    assert!(wire.switchbot_meter_enabled);
    assert!(
      !json.contains("A1:B2") && !json.contains("switchbotMeterDevice"),
      "the binding is Core bookkeeping, not something the settings screen needs"
    );
  }
}
