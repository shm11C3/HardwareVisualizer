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

  /// The device the ambient source reads, chosen by the user on the
  /// settings screen, or `None` until one has been chosen.
  ///
  /// Stored as the device's Bluetooth address, twelve lowercase hex
  /// digits - see [`is_device_id`]. Nothing is read until a choice
  /// exists: several devices in one room can read degrees apart, so
  /// which one is read is the user's decision rather than a race between
  /// advertisers. Cleared when the source is turned off.
  pub switchbot_meter_device: Option<String>,
}

impl EnvironmentalSensorSettings {
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

/// Whether a stored value is a device address this build can match:
/// twelve lowercase hex digits, the form the scan identifies devices by.
pub fn is_device_id(value: &str) -> bool {
  value.len() == 12
    && value
      .bytes()
      .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// The form a chosen device is stored in, or `None` when `value` is not
/// a device address at all.
///
/// Case is folded rather than refused because the address is the same
/// device either way; anything that is not twelve hex digits is refused
/// rather than stored, so the settings file can only ever hold a value
/// the scan can match.
pub fn normalize_device_id(value: &str) -> Option<String> {
  let normalized = value.to_ascii_lowercase();
  is_device_id(&normalized).then_some(normalized)
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

  fn enabled_with(device: Option<&str>) -> EnvironmentalSensorSettings {
    EnvironmentalSensorSettings {
      switchbot_meter_enabled: true,
      switchbot_meter_device: device.map(str::to_string),
    }
  }

  /// Nothing is chosen until the user picks a device.
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

  // -- what may be stored as the chosen device --

  /// The scan identifies devices by lowercase address, so a choice has
  /// to be stored the same way or it could never match what is heard.
  #[test]
  fn a_chosen_device_is_stored_as_its_lowercase_address() {
    assert_eq!(
      normalize_device_id("AABBCCDDA1B2").as_deref(),
      Some("aabbccdda1b2")
    );
    assert_eq!(
      normalize_device_id("aabbccdda1b2").as_deref(),
      Some("aabbccdda1b2")
    );
  }

  /// Anything else - a transport Debug string, a colon-separated address,
  /// a truncated or padded one - is refused rather than stored, so the
  /// settings file can only ever hold a value the scan can match.
  #[test]
  fn anything_but_twelve_hex_digits_is_refused_as_a_choice() {
    for value in [
      "",
      "PeripheralId(AA:BB:CC:DD:A1:B2)",
      "aa:bb:cc:dd:a1:b2",
      "aabbccdda1b",
      "aabbccdda1b2c",
      "aabbccdda1bg",
      " aabbccdda1b2",
    ] {
      assert_eq!(normalize_device_id(value), None, "{value:?}");
    }
  }

  /// A stored id that is not lowercase can only have been written by
  /// hand. It would never match a device, so it reads as nothing chosen
  /// - the same answer as any other unmatched form.
  #[test]
  fn an_uppercase_stored_id_reads_as_nothing_chosen() {
    let settings = enabled_with(Some("AABBCCDDA1B2"));
    assert_eq!(settings.chosen_device(), None);
    assert_eq!(
      enabled_with(Some("aabbccdda1b2")).chosen_device(),
      Some("aabbccdda1b2")
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
