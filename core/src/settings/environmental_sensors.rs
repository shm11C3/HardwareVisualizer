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

  /// Transport identifier of the meter this machine is bound to, or
  /// `None` before one has been chosen.
  ///
  /// Without this the choice of meter would be remade every launch. The
  /// provider latches to the first meter that advertises, which is fine
  /// with one meter and wrong with two: the app would wander between
  /// rooms across restarts and blend their histories, which is exactly
  /// what the one-meter rule exists to prevent. Remembering the device
  /// makes the binding a decision rather than a race.
  ///
  /// Cleared when the user turns the source off, so switching the toggle
  /// off and on again is how they re-bind to a different meter.
  pub switchbot_meter_device: Option<String>,
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

  /// Nothing is bound until a meter has actually been found.
  #[test]
  fn no_meter_is_remembered_by_default() {
    assert_eq!(
      EnvironmentalSensorSettings::default().switchbot_meter_device,
      None
    );
  }

  /// The binding is what makes the choice of meter survive a restart, so
  /// it has to come back off disk intact.
  #[test]
  fn a_remembered_meter_round_trips() {
    let settings = EnvironmentalSensorSettings {
      switchbot_meter_enabled: true,
      switchbot_meter_device: Some("PeripheralId(AA:BB:CC:DD:A1:B2)".to_string()),
    };

    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"switchbotMeterDevice\""));
    assert_eq!(
      serde_json::from_str::<EnvironmentalSensorSettings>(&json).unwrap(),
      settings
    );
  }

  /// A settings file written before the binding existed must not be
  /// treated as bound to something.
  #[test]
  fn an_older_settings_block_without_a_binding_reads_as_unbound() {
    let settings: EnvironmentalSensorSettings =
      serde_json::from_str(r#"{"switchbotMeterEnabled": true}"#).unwrap();

    assert!(settings.switchbot_meter_enabled);
    assert_eq!(settings.switchbot_meter_device, None);
  }

  #[test]
  fn an_enabled_scan_round_trips_in_camel_case() {
    let settings = EnvironmentalSensorSettings {
      switchbot_meter_enabled: true,
      switchbot_meter_device: None,
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
