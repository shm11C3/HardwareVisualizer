use serde::{Deserialize, Serialize};

/// Core-owned environmental (ambient) sensor settings (#2044).
///
/// Persisted as the `environmentalSensors` key in the shared
/// `settings.json`. It lives in `hardviz_core` rather than the App crate
/// because Core is what consumes it: the value decides whether a
/// provider is registered with the ambient registry the archive worker
/// reads.
///
/// One key per vendor rather than a single "ambient sensors" switch. The
/// abstraction in #2043 exists so other vendors can follow, and each one
/// is a distinct radio and a distinct piece of hardware the user either
/// owns or does not; a shared flag would turn buying one sensor into
/// scanning for all of them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EnvironmentalSensorSettings {
  /// Whether to listen for SwitchBot Meter advertisements.
  ///
  /// Defaults to **off**, unlike most collection settings, and the
  /// derived `false` is that decision rather than an accident of the
  /// type. Every other source this app reads is inside the machine the
  /// user already pointed it at; this one turns on a radio and listens
  /// to the room. Starting that silently on every install -
  /// overwhelmingly on machines with no meter anywhere near them -
  /// would be taking a permission nobody granted, so the scan begins
  /// only after the user says they have the hardware.
  pub switchbot_meter_enabled: bool,
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The default that matters most in this file: an app that was never
  /// asked must not start scanning.
  #[test]
  fn the_switchbot_scan_is_off_until_the_user_turns_it_on() {
    assert!(!EnvironmentalSensorSettings::default().switchbot_meter_enabled);
  }

  #[test]
  fn missing_fields_fall_back_to_defaults() {
    let settings: EnvironmentalSensorSettings =
      serde_json::from_str("{}").expect("an empty object is a valid settings block");
    assert_eq!(settings, EnvironmentalSensorSettings::default());
  }

  #[test]
  fn an_enabled_scan_round_trips_in_camel_case() {
    let settings = EnvironmentalSensorSettings {
      switchbot_meter_enabled: true,
    };

    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"switchbotMeterEnabled\""));
    assert_eq!(
      serde_json::from_str::<EnvironmentalSensorSettings>(&json).unwrap(),
      settings
    );
  }

  /// A key this version does not know about must not cost the user the
  /// setting they did make - other vendors land in this same block.
  #[test]
  fn an_unknown_vendor_key_does_not_discard_the_known_one() {
    let settings: EnvironmentalSensorSettings = serde_json::from_str(
      r#"{"switchbotMeterEnabled": true, "someFutureVendorEnabled": true}"#,
    )
    .unwrap();

    assert!(settings.switchbot_meter_enabled);
  }
}
