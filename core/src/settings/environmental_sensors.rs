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

impl EnvironmentalSensorSettings {
  /// The device to write down for a meter that just latched, or `None`
  /// when nothing should be written.
  ///
  /// The caller must evaluate this against the same settings it is about
  /// to save, while holding the settings lock. The scan outlives a
  /// change to the toggle - it keeps running until the app restarts - so
  /// a binding reported just before the user turned the source off would
  /// otherwise be written back afterwards. That resurrected device would
  /// then be adopted the next time the source was enabled, silently
  /// skipping the re-bind that turning it off was meant to grant.
  ///
  /// Refusing when the source is disabled is what makes "off and on
  /// again" actually clear the binding, so the check belongs beside the
  /// write rather than at the point the binding was observed.
  pub fn binding_to_persist(&self, device_id: &str) -> Option<String> {
    if !self.switchbot_meter_enabled {
      return None;
    }

    if self.switchbot_meter_device.as_deref() == Some(device_id) {
      return None;
    }

    Some(device_id.to_string())
  }

  /// The chosen device, or `None` when nothing usable is stored.
  ///
  /// A device is identified by its Bluetooth address, written as twelve
  /// hex digits. An earlier build stored the transport library's `Debug`
  /// string instead; such a value can never match a device again, and
  /// keeping it would leave the source permanently bound to a sensor
  /// that cannot exist - unavailable forever, with no explanation. It is
  /// read as "nothing chosen" so the user is asked to pick, which is
  /// also what happens on a fresh install.
  pub fn chosen_device(&self) -> Option<&str> {
    self
      .switchbot_meter_device
      .as_deref()
      .filter(|id| is_device_id(id))
  }
}

/// Whether a stored value is a device address this build can match.
fn is_device_id(value: &str) -> bool {
  value.len() == 12 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
  use super::*;

  const METER_A: &str = "PeripheralId(AA:BB:CC:DD:A1:B2)";
  const METER_B: &str = "PeripheralId(AA:BB:CC:DD:C3:D4)";

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

  // -- deciding whether a reported binding gets written --

  fn enabled_with(device: Option<&str>) -> EnvironmentalSensorSettings {
    EnvironmentalSensorSettings {
      switchbot_meter_enabled: true,
      switchbot_meter_device: device.map(str::to_string),
    }
  }

  #[test]
  fn a_first_binding_is_written_down() {
    assert_eq!(
      enabled_with(None).binding_to_persist(METER_A),
      Some(METER_A.to_string())
    );
  }

  /// Regression: the scan keeps running after the user turns the source
  /// off, so a binding reported just before that could be written back
  /// afterwards. The resurrected device would then be adopted on the
  /// next enable, skipping the re-bind that turning it off was meant to
  /// grant.
  #[test]
  fn a_binding_reported_after_the_source_was_disabled_is_not_written_back() {
    let disabled = EnvironmentalSensorSettings {
      switchbot_meter_enabled: false,
      switchbot_meter_device: None,
    };

    assert_eq!(
      disabled.binding_to_persist(METER_A),
      None,
      "turning the source off must actually clear the binding, not have it restored behind the user"
    );
  }

  /// Disabling clears the device, but a late report must not reinstate
  /// one that was still recorded either.
  #[test]
  fn a_late_binding_is_refused_even_when_a_device_is_still_recorded() {
    let disabled = EnvironmentalSensorSettings {
      switchbot_meter_enabled: false,
      switchbot_meter_device: Some(METER_A.to_string()),
    };

    assert_eq!(disabled.binding_to_persist(METER_B), None);
  }

  /// Re-reporting the meter already recorded is not a change, so it must
  /// not cause a settings write on every launch.
  #[test]
  fn re_reporting_the_recorded_meter_writes_nothing() {
    assert_eq!(
      enabled_with(Some(METER_A)).binding_to_persist(METER_A),
      None
    );
  }

  /// After a re-bind the newly latched meter replaces the old one.
  #[test]
  fn a_different_meter_replaces_the_recorded_one_while_enabled() {
    assert_eq!(
      enabled_with(Some(METER_A)).binding_to_persist(METER_B),
      Some(METER_B.to_string())
    );
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
