//! Decoding of SwitchBot Meter BLE advertisement service data (#2044).
//!
//! Pure byte decoding, deliberately free of any transport type: the
//! Windows radio layer hands this module a service-data UUID and its
//! payload, and everything below is arithmetic. That keeps the part of
//! this provider that can be wrong about a temperature testable from a
//! fixed byte string on every platform, with no adapter and no meter.
//!
//! # Source
//!
//! Layout and formulas come from SwitchBot's own published BLE
//! documentation, `OpenWonderLabs/SwitchBotAPI-BLE`, file
//! `devicetypes/meter.md`, section "(New) Broadcast Message" (device
//! type table in the same file; Meter Plus's device type from the
//! repository `README.md` device type table). Only the broadcast
//! (advertisement) half of that document is used here - the connection,
//! pairing, and command halves are out of scope, because this provider
//! never connects to the meter.
//!
//! The vendor document's Outdoor Temperature/Humidity Sensor section is
//! explicitly labelled "unofficial and community provided" and states
//! the payload moved, so that model is intentionally **not** decoded
//! here: shipping a decode we cannot source would risk archiving wrong
//! temperatures under a confident label.

/// SwitchBot service-data UUIDs whose payload is read as a meter frame.
///
/// The device type in byte 0 identifies the model, but only inside a
/// SwitchBot-owned service-data entry. Without this gate a passing
/// advertisement from any unrelated vendor whose payload happened to
/// begin with `0x54` would decode into a plausible-looking room
/// temperature, which is worse than reading nothing at all.
///
/// All three appear in the vendor documentation: `meter.md` names
/// `cba20d00-...` and `fee7` as the meter's scan-response UUIDs, and the
/// sample advertisement in the same file's outdoor-sensor section shows
/// current firmware broadcasting under `fd3d`.
pub const SWITCHBOT_SERVICE_UUIDS: [u128; 3] = [
  // 0000fd3d-0000-1000-8000-00805f9b34fb
  0x0000fd3d_0000_1000_8000_00805f9b34fb,
  // 0000fee7-0000-1000-8000-00805f9b34fb
  0x0000fee7_0000_1000_8000_00805f9b34fb,
  // cba20d00-224d-11e6-9fb8-0002a5d5c51b
  0xcba20d00_224d_11e6_9fb8_0002a5d5c51b,
];

/// Shortest service-data payload that can carry a full reading.
///
/// Bytes 0-5 are all the broadcast format defines for a meter; the
/// vendor document allows up to eight bytes, so a longer payload is
/// normal and its tail is ignored rather than treated as a mismatch.
const MINIMUM_SERVICE_DATA_LEN: usize = 6;

/// Which SwitchBot model produced a decoded frame.
///
/// Kept as a decode-level fact rather than a user-facing type: it is
/// what the device type byte said, and it is useful in a log line when a
/// reading looks wrong. Nothing downstream branches on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchBotMeterModel {
  /// WoSensorTH, device type `0x54` 'T' (normal) or `0x74` 't' (add
  /// mode). Sold as SwitchBot Meter.
  Meter,
  /// Device type `0x69` 'i'. Sold as SwitchBot Meter Plus. It shares the
  /// meter's service-data layout; only the device type byte differs.
  MeterPlus,
}

impl SwitchBotMeterModel {
  /// The model for a device type byte, or `None` when the frame is some
  /// other SwitchBot product (a Bot, a Curtain, a Hub) that happens to
  /// broadcast under the same service UUID.
  fn from_device_type(device_type: u8) -> Option<Self> {
    match device_type {
      0x54 | 0x74 => Some(Self::Meter),
      0x69 => Some(Self::MeterPlus),
      _ => None,
    }
  }
}

/// One decoded meter broadcast.
///
/// Temperature is always Celsius, never optional: a frame that cannot
/// produce a trustworthy temperature does not become a
/// [`SwitchBotMeterFrame`] at all. Humidity is optional so that an
/// out-of-range humidity costs only the humidity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchBotMeterFrame {
  pub model: SwitchBotMeterModel,
  pub temperature_celsius: f32,
  pub humidity_percent: Option<f32>,
}

/// Decode a meter reading from one service-data entry, or `None` when
/// this entry is not a readable SwitchBot meter frame.
///
/// `service_uuid` is the 128-bit form of the advertised service-data
/// UUID. Returning `None` is the normal case: a scan sees every nearby
/// advertisement, and almost none of them are meters.
pub fn decode_service_data(
  service_uuid: u128,
  data: &[u8],
) -> Option<SwitchBotMeterFrame> {
  if !SWITCHBOT_SERVICE_UUIDS.contains(&service_uuid) {
    return None;
  }
  decode_meter_service_data(data)
}

/// Decode the payload itself, with the service UUID already accepted.
///
/// Split out so the byte arithmetic can be exercised directly, and so
/// the UUID gate stays one visible decision rather than being tangled
/// into the field extraction.
fn decode_meter_service_data(data: &[u8]) -> Option<SwitchBotMeterFrame> {
  if data.len() < MINIMUM_SERVICE_DATA_LEN {
    return None;
  }

  // Byte 0 bit[7] is reserved (encryption flag in later firmware), so
  // the model lives in the low seven bits and the top bit must not be
  // allowed to hide a meter behind an unknown device type.
  let model = SwitchBotMeterModel::from_device_type(data[0] & 0x7F)?;

  // Byte 3 bit[3:0] is documented as 0-9. A value above 9 means this
  // payload is not laid out the way we think it is, and a mis-scaled
  // temperature archived under a confident label is exactly the failure
  // ambient data exists to avoid - so the whole frame is refused rather
  // than the digit clamped.
  let temperature_decimal = data[3] & 0x0F;
  if temperature_decimal > 9 {
    return None;
  }

  let temperature_integer = data[4] & 0x7F;
  let temperature_magnitude =
    f32::from(temperature_integer) + f32::from(temperature_decimal) * 0.1;

  // Byte 4 bit[7] is a sign flag with the opposite polarity to the usual
  // convention: the vendor document defines 0 as subzero and 1 as above
  // zero. Reading it the intuitive way would silently mirror every
  // reading through zero, so this branch is deliberately explicit.
  let above_zero = data[4] & 0x80 != 0;
  let temperature_celsius = if above_zero {
    temperature_magnitude
  } else {
    -temperature_magnitude
  };

  Some(SwitchBotMeterFrame {
    model,
    temperature_celsius,
    // Byte 5 bit[7] is the meter's own *display* scale. The broadcast
    // integer and decimal are Celsius whichever way that bit points, so
    // converting on it would corrupt readings from any meter whose
    // owner pressed the °F button on the back. Byte 5 contributes
    // humidity only.
    humidity_percent: decode_humidity(data[5]),
  })
}

/// Relative humidity from byte 5, or `None` when it is outside the
/// documented range.
///
/// A bad humidity drops only the humidity, matching how the ambient
/// registry normalizes readings: temperature is the reading that
/// explains machine temperatures, and losing it over a suspect
/// second field would trade a good measurement for a bad one.
fn decode_humidity(byte: u8) -> Option<f32> {
  let humidity = byte & 0x7F;
  (humidity <= 99).then(|| f32::from(humidity))
}

#[cfg(test)]
mod tests {
  use super::*;

  const FD3D: u128 = SWITCHBOT_SERVICE_UUIDS[0];

  /// A meter frame built field by field, so each test can state the one
  /// byte it is about instead of restating a whole packet.
  ///
  /// Defaults describe 24.5 °C / 48 % on a Meter: device type 'T',
  /// battery 100, no alerts, temperature above zero, Celsius display.
  fn frame_bytes(
    device_type: u8,
    temperature_decimal: u8,
    temperature_integer_with_sign: u8,
    humidity_with_scale: u8,
  ) -> [u8; 6] {
    [
      device_type,
      0x00,
      0x64,
      temperature_decimal,
      temperature_integer_with_sign,
      humidity_with_scale,
    ]
  }

  fn meter_at(temperature_decimal: u8, integer_with_sign: u8) -> [u8; 6] {
    frame_bytes(0x54, temperature_decimal, integer_with_sign, 48)
  }

  // -- service UUID gate --

  #[test]
  fn a_meter_frame_under_the_current_switchbot_service_uuid_decodes() {
    let decoded = decode_service_data(FD3D, &meter_at(5, 0x80 | 24));
    assert_eq!(decoded.map(|frame| frame.temperature_celsius), Some(24.5));
  }

  #[test]
  fn a_meter_frame_under_the_legacy_switchbot_service_uuids_decodes() {
    for uuid in [SWITCHBOT_SERVICE_UUIDS[1], SWITCHBOT_SERVICE_UUIDS[2]] {
      assert!(
        decode_service_data(uuid, &meter_at(5, 0x80 | 24)).is_some(),
        "the vendor document names this UUID for the meter broadcast"
      );
    }
  }

  /// The gate that stops an unrelated vendor's advertisement from being
  /// archived as a room temperature just because its first byte is
  /// `0x54`.
  #[test]
  fn an_identical_payload_under_a_foreign_service_uuid_is_ignored() {
    let foreign = 0x0000180f_0000_1000_8000_00805f9b34fb;
    assert_eq!(decode_service_data(foreign, &meter_at(5, 0x80 | 24)), None);
  }

  // -- device type --

  #[test]
  fn the_meter_decodes_in_both_its_normal_and_add_mode_device_types() {
    for device_type in [0x54, 0x74] {
      let frame = decode_service_data(FD3D, &frame_bytes(device_type, 5, 0x80 | 24, 48))
        .expect("both device types are documented as WoSensorTH");
      assert_eq!(frame.model, SwitchBotMeterModel::Meter);
    }
  }

  #[test]
  fn the_meter_plus_decodes_with_the_same_layout() {
    let frame = decode_service_data(FD3D, &frame_bytes(0x69, 5, 0x80 | 24, 48))
      .expect("Meter Plus shares the meter service data layout");
    assert_eq!(frame.model, SwitchBotMeterModel::MeterPlus);
    assert_eq!(frame.temperature_celsius, 24.5);
  }

  /// Byte 0 bit[7] is reserved, so a meter must still be recognised when
  /// firmware sets it.
  #[test]
  fn the_reserved_high_bit_of_the_device_type_byte_does_not_hide_a_meter() {
    let frame = decode_service_data(FD3D, &frame_bytes(0x80 | 0x54, 5, 0x80 | 24, 48))
      .expect("only bits 6:0 carry the device type");
    assert_eq!(frame.model, SwitchBotMeterModel::Meter);
  }

  #[test]
  fn another_switchbot_product_on_the_same_service_uuid_is_not_a_meter() {
    // 'H' (0x48) is the Bot, which broadcasts under the same UUID.
    assert_eq!(
      decode_service_data(FD3D, &frame_bytes(0x48, 5, 0x80 | 24, 48)),
      None
    );
  }

  // -- temperature --

  #[test]
  fn a_whole_number_temperature_decodes_without_a_fraction() {
    let frame = decode_service_data(FD3D, &meter_at(0, 0x80 | 21)).unwrap();
    assert_eq!(frame.temperature_celsius, 21.0);
  }

  /// The polarity trap: the vendor document defines the flag as 0 =
  /// subzero, 1 = above zero, which is the reverse of the usual "sign
  /// bit set means negative" reading.
  #[test]
  fn a_subzero_temperature_is_negative_when_the_flag_is_clear() {
    let frame = decode_service_data(FD3D, &meter_at(5, 3)).unwrap();
    assert_eq!(frame.temperature_celsius, -3.5);
  }

  #[test]
  fn zero_degrees_decodes_as_zero_in_both_sign_states() {
    assert_eq!(
      decode_service_data(FD3D, &meter_at(0, 0x80))
        .unwrap()
        .temperature_celsius,
      0.0
    );
    assert_eq!(
      decode_service_data(FD3D, &meter_at(0, 0x00))
        .unwrap()
        .temperature_celsius,
      0.0
    );
  }

  #[test]
  fn the_documented_temperature_range_ends_decode() {
    let hottest = decode_service_data(FD3D, &meter_at(9, 0x80 | 127)).unwrap();
    assert_eq!(hottest.temperature_celsius, 127.9);

    let coldest = decode_service_data(FD3D, &meter_at(9, 127)).unwrap();
    assert_eq!(coldest.temperature_celsius, -127.9);
  }

  /// Regression guard for the classic SwitchBot decode bug: byte 5
  /// bit[7] reports which unit the meter's own screen shows, not which
  /// unit it broadcast. Converting on it would report 76.1 °C for a
  /// 24.5 °C room.
  #[test]
  fn a_meter_set_to_show_fahrenheit_still_broadcasts_celsius() {
    let celsius_display = decode_service_data(FD3D, &meter_at(5, 0x80 | 24)).unwrap();
    let fahrenheit_display =
      decode_service_data(FD3D, &frame_bytes(0x54, 5, 0x80 | 24, 0x80 | 48)).unwrap();

    assert_eq!(fahrenheit_display.temperature_celsius, 24.5);
    assert_eq!(
      fahrenheit_display.temperature_celsius,
      celsius_display.temperature_celsius
    );
  }

  /// The alert bits share byte 3 with the temperature decimal, so a
  /// meter reporting a high-temperature alert must not shift its
  /// reading.
  #[test]
  fn temperature_and_humidity_alert_bits_do_not_disturb_the_decimal() {
    let alerting = frame_bytes(0x54, 0b1010_0000 | 5, 0x80 | 24, 48);
    let frame = decode_service_data(FD3D, &alerting).unwrap();
    assert_eq!(frame.temperature_celsius, 24.5);
  }

  /// A decimal digit above 9 is outside the documented encoding, so the
  /// payload is not the frame we think it is.
  #[test]
  fn an_out_of_range_temperature_decimal_refuses_the_whole_frame() {
    assert_eq!(decode_service_data(FD3D, &meter_at(0x0A, 0x80 | 24)), None);
  }

  // -- humidity --

  #[test]
  fn humidity_decodes_from_the_low_seven_bits() {
    let frame = decode_service_data(FD3D, &meter_at(5, 0x80 | 24)).unwrap();
    assert_eq!(frame.humidity_percent, Some(48.0));
  }

  #[test]
  fn the_documented_humidity_range_ends_decode() {
    for (byte, expected) in [(0u8, 0.0f32), (99, 99.0)] {
      let frame =
        decode_service_data(FD3D, &frame_bytes(0x54, 5, 0x80 | 24, byte)).unwrap();
      assert_eq!(frame.humidity_percent, Some(expected));
    }
  }

  /// Humidity is the optional half of the reading, so a value outside
  /// the documented range costs the humidity and not the temperature
  /// the archive actually needs.
  #[test]
  fn an_out_of_range_humidity_drops_only_the_humidity() {
    let frame =
      decode_service_data(FD3D, &frame_bytes(0x54, 5, 0x80 | 24, 100)).unwrap();
    assert_eq!(frame.humidity_percent, None);
    assert_eq!(frame.temperature_celsius, 24.5);
  }

  // -- payload length --

  #[test]
  fn a_payload_too_short_to_hold_a_reading_is_ignored() {
    let full = meter_at(5, 0x80 | 24);
    for length in 0..MINIMUM_SERVICE_DATA_LEN {
      assert_eq!(
        decode_service_data(FD3D, &full[..length]),
        None,
        "a {length}-byte payload cannot carry a temperature"
      );
    }
  }

  /// The vendor document allows up to eight service-data bytes and
  /// leaves the tail device-specific, so extra bytes are not a mismatch.
  #[test]
  fn a_longer_payload_decodes_from_its_documented_prefix() {
    let padded = [0x54, 0x00, 0x64, 5, 0x80 | 24, 48, 0xAB, 0xCD];
    let frame = decode_service_data(FD3D, &padded).unwrap();
    assert_eq!(frame.temperature_celsius, 24.5);
    assert_eq!(frame.humidity_percent, Some(48.0));
  }

  // -- a captured frame, end to end --

  /// A whole realistic advertisement payload decoded in one assertion,
  /// so the fixed byte string this provider is built on is visible in
  /// one place rather than only as field-by-field variations.
  #[test]
  fn a_complete_meter_advertisement_decodes_to_its_reading() {
    // 'T', no group, battery 100 %, no alerts + decimal 7, above zero +
    // 26 °C, Celsius display + 53 %.
    let advertisement = [0x54, 0x00, 0x64, 0x07, 0x9A, 0x35];
    assert_eq!(
      decode_service_data(FD3D, &advertisement),
      Some(SwitchBotMeterFrame {
        model: SwitchBotMeterModel::Meter,
        temperature_celsius: 26.7,
        humidity_percent: Some(53.0),
      })
    );
  }
}
