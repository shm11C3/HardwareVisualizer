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
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(default, rename_all = "camelCase")]
pub struct EnvironmentalSensorSettings {
  pub switchbot_meter_enabled: bool,
  /// Which device the ambient source reads, or `None` until one is
  /// chosen. Unlike the rest of this struct it reaches the frontend
  /// because choosing is the interaction: several sensors in one room
  /// can read degrees apart, so the screen must be able to say which one
  /// is selected and offer the others.
  pub switchbot_meter_device: Option<String>,
}

/// One SwitchBot device the radio is hearing, offered for selection.
///
/// Carries the reading rather than a model name because model identity
/// cannot be trusted from these broadcasts, and because the temperature
/// is what actually tells the user which device sits near the intake.
#[derive(Debug, Serialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct AmbientSensorCandidate {
  /// Full address, the value to pass back when selecting this device.
  pub device_id: String,
  /// Last four hex digits - enough to tell devices apart, and the tail
  /// owners tend to name them by.
  pub short_id: String,
  pub temperature_celsius: f32,
  pub humidity_percent: Option<f32>,
  /// Whether this is the device currently selected.
  pub selected: bool,
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
      // Normalized rather than copied: a value an older build stored in
      // another format can never match a device again, and showing it as
      // a selection would leave the screen claiming a sensor that cannot
      // exist.
      switchbot_meter_device: value.chosen_device().map(str::to_string),
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
