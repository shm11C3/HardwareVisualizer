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

/// Byte 0 bit[7], which marks the payload as encrypted.
///
/// The vendor names this bit inconsistently - `meter.md`'s byte 0 row is
/// labelled "Enc Type Dev Type" while calling bit[7] "Reserved", and the
/// repository `README.md` service-data table labels the same bit
/// "Enc type". Neither states the polarity outright.
///
/// That ambiguity decides the behavior rather than excusing it. If the
/// bit means what its name says, bytes 3-5 of a marked frame are
/// ciphertext, and ciphertext decodes into a perfectly plausible room
/// temperature whenever its low nibble happens to land in 0-9. There is
/// no way to tell such a value from a real one after the fact, and it
/// would be archived under a confident label and drive Thermal Delta.
/// Refusing the frame costs at most a reading we could have taken -
/// reported honestly as an unavailable source - so the conservative
/// direction is the only defensible one.
const ENCRYPTED_PAYLOAD_FLAG: u8 = 0x80;

/// What one service-data entry turned out to be.
///
/// Three outcomes rather than an `Option`, because "not a meter" and "a
/// meter we must not read" call for different handling: the first is the
/// overwhelmingly common case during a scan and deserves silence, the
/// second is worth telling the user about once, since it explains why a
/// meter they own produces nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeterAdvertisement {
  /// A readable meter frame.
  Reading(SwitchBotMeterFrame),
  /// A meter frame whose payload is marked encrypted, so its bytes
  /// cannot be trusted as a temperature.
  Encrypted(SwitchBotMeterModel),
  /// Not a SwitchBot meter frame at all - a different vendor, a
  /// different SwitchBot product, or a malformed payload.
  Ignored,
}

/// Classify one service-data entry.
///
/// `service_uuid` is the 128-bit form of the advertised service-data
/// UUID. [`MeterAdvertisement::Ignored`] is the normal case: a scan sees
/// every nearby advertisement, and almost none of them are meters.
pub fn decode_service_data(service_uuid: u128, data: &[u8]) -> MeterAdvertisement {
  if !SWITCHBOT_SERVICE_UUIDS.contains(&service_uuid) {
    return MeterAdvertisement::Ignored;
  }
  decode_meter_service_data(data)
}

/// Decode the payload itself, with the service UUID already accepted.
///
/// Split out so the byte arithmetic can be exercised directly, and so
/// the UUID gate stays one visible decision rather than being tangled
/// into the field extraction.
fn decode_meter_service_data(data: &[u8]) -> MeterAdvertisement {
  if data.len() < MINIMUM_SERVICE_DATA_LEN {
    return MeterAdvertisement::Ignored;
  }

  // The model lives in the low seven bits of byte 0.
  let Some(model) = SwitchBotMeterModel::from_device_type(data[0] & 0x7F) else {
    return MeterAdvertisement::Ignored;
  };

  // Identify the model first, then refuse: a user whose meter is
  // broadcasting encrypted deserves a log line naming the device, not
  // silence indistinguishable from having no meter at all.
  if data[0] & ENCRYPTED_PAYLOAD_FLAG != 0 {
    return MeterAdvertisement::Encrypted(model);
  }

  // Byte 3 bit[3:0] is documented as 0-9. A value above 9 means this
  // payload is not laid out the way we think it is, and a mis-scaled
  // temperature archived under a confident label is exactly the failure
  // ambient data exists to avoid - so the whole frame is refused rather
  // than the digit clamped.
  let temperature_decimal = data[3] & 0x0F;
  if temperature_decimal > 9 {
    return MeterAdvertisement::Ignored;
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

  MeterAdvertisement::Reading(SwitchBotMeterFrame {
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

/// SwitchBot's Bluetooth SIG company identifier, gating manufacturer
/// data the same way [`SWITCHBOT_SERVICE_UUIDS`] gates service data.
///
/// Observed on every SwitchBot device in the capture described below,
/// and on nothing else in the same room.
pub const SWITCHBOT_COMPANY_ID: u16 = 0x0969;

/// Where a manufacturer-data payload keeps its reading, per length.
///
/// # Source: first-party observation, not a vendor document
///
/// SwitchBot publishes no broadcast layout for its Hub family (the
/// vendor's `SwitchBotAPI-BLE` repository documents neither Hub 2 nor
/// Hub 3, and the community implementation reads Hub temperature from
/// the cloud API instead). The offsets below come from a capture taken
/// on 2026-09-02 with three devices in one room, each decoded value
/// cross-checked against what the SwitchBot app displayed at the time:
///
/// | manufacturer data | decoded | app displayed |
/// | --- | --- | --- |
/// | `b0e9feb08a1900ff6a97085584039b3280` | 27.3 °C / 50 % | 27.3 °C / 49 % |
/// | `d051fa0f2cd00000000000000c00993600` | 25.0 °C / 54 % | 25.2 °C / 53 % |
/// | `b0e9fe55a7df03640899330022054c20`   | 25.8 °C / 51 % | 25.8 °C / 51 % |
///
/// The Meter's own sign convention (bit 7 set means above zero) holds in
/// all three, which is what makes one shared decoder defensible rather
/// than three guesses. Still unconfirmed, and the reason this path is
/// experimental: sub-zero readings, whether a display-unit bit exists,
/// and what the bytes on either side of the reading carry.
///
/// Model identity is deliberately *not* derived from these payloads. The
/// service-data device type identifies some models (`0x76` was the Hub 2,
/// `0x35` the CO2 meter) but the Hub 3 advertised type `0x00` - the most
/// generic byte there is, and shared with an unrelated device in the same
/// capture. Claiming a model from that would invite exactly the confident
/// mislabelling this module exists to avoid, so devices are identified by
/// their address and their reading instead.
fn reading_offset(payload_len: usize) -> Option<usize> {
  match payload_len {
    // Hub 2 and Hub 3 both broadcast 17 bytes with the reading last but
    // one; the six bytes after the address differ between them (one
    // carries a counter, the other zeros) and are not read.
    17 => Some(13),
    // The CO2 meter's 16-byte payload puts the reading directly after a
    // battery byte, mirroring the documented Meter layout.
    16 => Some(8),
    _ => None,
  }
}

/// Classify one manufacturer-data entry.
///
/// Unlike [`decode_service_data`], this cannot name a model, so a
/// readable frame is reported without one - see [`reading_offset`] for
/// why guessing would be worse than admitting it.
pub fn decode_manufacturer_data(
  company_id: u16,
  data: &[u8],
) -> ManufacturerAdvertisement {
  if company_id != SWITCHBOT_COMPANY_ID {
    return ManufacturerAdvertisement::Ignored;
  }

  let Some(offset) = reading_offset(data.len()) else {
    return ManufacturerAdvertisement::Ignored;
  };
  // Every observed payload opens with the device's own address; the
  // length gate above already guarantees these six bytes exist.
  let address: [u8; 6] = data[..6].try_into().expect("length checked above");
  let (decimal_byte, integer_byte, humidity_byte) =
    (data[offset], data[offset + 1], data[offset + 2]);

  // A device broadcasting zeros where a reading belongs is not reporting
  // a measurement - a SwitchBot bulb in the capture did exactly this.
  // Decoding it would yield a perfectly plausible -0.0 °C / 0 %, which
  // is the kind of confident wrong number ambient data exists to avoid.
  if decimal_byte == 0 && integer_byte == 0 && humidity_byte == 0 {
    return ManufacturerAdvertisement::Ignored;
  }

  // Same refusal as the service-data path: a decimal digit above 9 means
  // this payload is not laid out the way we believe it is.
  let temperature_decimal = decimal_byte & 0x0F;
  if temperature_decimal > 9 {
    return ManufacturerAdvertisement::Ignored;
  }

  let temperature_magnitude =
    f32::from(integer_byte & 0x7F) + f32::from(temperature_decimal) * 0.1;
  let above_zero = integer_byte & 0x80 != 0;

  ManufacturerAdvertisement::Reading(SwitchBotAmbientFrame {
    address,
    temperature_celsius: if above_zero {
      temperature_magnitude
    } else {
      -temperature_magnitude
    },
    humidity_percent: decode_humidity(humidity_byte),
  })
}

/// One decoded manufacturer-data broadcast, with no model attached.
///
/// `address` is the device's own Bluetooth address, carried in the first
/// six bytes of every payload observed. Reading identity out of the
/// frame rather than from the transport handle matters twice over: the
/// identity and the reading then arrive together, needing no correlation
/// between two advertisement events, and the address is a specified
/// value where the transport handle is a library's `Debug` string that
/// could change shape between releases and silently orphan a remembered
/// choice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchBotAmbientFrame {
  pub address: [u8; 6],
  pub temperature_celsius: f32,
  pub humidity_percent: Option<f32>,
}

impl SwitchBotAmbientFrame {
  /// The address as `b0e9feb08a19`, the form persisted in settings.
  pub fn address_id(&self) -> String {
    self.address.iter().map(|b| format!("{b:02x}")).collect()
  }

  /// The last two bytes as `8a19` - enough to tell devices apart in a
  /// picker or a label, and the same tail owners tend to name them by.
  pub fn short_id(&self) -> String {
    format!("{:02x}{:02x}", self.address[4], self.address[5])
  }
}

/// What one manufacturer-data entry turned out to be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ManufacturerAdvertisement {
  Reading(SwitchBotAmbientFrame),
  /// Not a SwitchBot payload, not a length we have observed, or a
  /// payload carrying no reading.
  Ignored,
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

  /// The readable frame, or `None` for anything this decoder refuses.
  ///
  /// Most tests are about the byte arithmetic and only care whether a
  /// reading came out, so they read better against an `Option` than
  /// against the three-way outcome. The tests that are specifically
  /// about refusal match on [`MeterAdvertisement`] directly.
  fn reading(service_uuid: u128, data: &[u8]) -> Option<SwitchBotMeterFrame> {
    match decode_service_data(service_uuid, data) {
      MeterAdvertisement::Reading(frame) => Some(frame),
      _ => None,
    }
  }

  // -- service UUID gate --

  #[test]
  fn a_meter_frame_under_the_current_switchbot_service_uuid_decodes() {
    let decoded = reading(FD3D, &meter_at(5, 0x80 | 24));
    assert_eq!(decoded.map(|frame| frame.temperature_celsius), Some(24.5));
  }

  #[test]
  fn a_meter_frame_under_the_legacy_switchbot_service_uuids_decodes() {
    for uuid in [SWITCHBOT_SERVICE_UUIDS[1], SWITCHBOT_SERVICE_UUIDS[2]] {
      assert!(
        reading(uuid, &meter_at(5, 0x80 | 24)).is_some(),
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
    assert_eq!(reading(foreign, &meter_at(5, 0x80 | 24)), None);
  }

  // -- device type --

  #[test]
  fn the_meter_decodes_in_both_its_normal_and_add_mode_device_types() {
    for device_type in [0x54, 0x74] {
      let frame = reading(FD3D, &frame_bytes(device_type, 5, 0x80 | 24, 48))
        .expect("both device types are documented as WoSensorTH");
      assert_eq!(frame.model, SwitchBotMeterModel::Meter);
    }
  }

  #[test]
  fn the_meter_plus_decodes_with_the_same_layout() {
    let frame = reading(FD3D, &frame_bytes(0x69, 5, 0x80 | 24, 48))
      .expect("Meter Plus shares the meter service data layout");
    assert_eq!(frame.model, SwitchBotMeterModel::MeterPlus);
    assert_eq!(frame.temperature_celsius, 24.5);
  }

  // -- encrypted payloads --

  /// The core of the refusal: an encrypted frame must never become a
  /// temperature.
  ///
  /// These exact bytes are the trap. They are a valid-looking meter
  /// frame - device type 'T', decimal 5, integer 24, humidity 48 - with
  /// only the encryption flag added. Read as plaintext they yield a
  /// wholly believable 24.5 °C, which is precisely why the flag has to
  /// be honored: nothing downstream could ever tell that value was
  /// ciphertext.
  #[test]
  fn an_encrypted_payload_never_becomes_a_reading() {
    let encrypted = frame_bytes(ENCRYPTED_PAYLOAD_FLAG | 0x54, 5, 0x80 | 24, 48);

    assert_eq!(
      reading(FD3D, &encrypted),
      None,
      "ciphertext whose nibbles happen to look like a temperature must not be archived"
    );
    assert_eq!(
      decode_service_data(FD3D, &encrypted),
      MeterAdvertisement::Encrypted(SwitchBotMeterModel::Meter),
      "the model is still identified, so the refusal can be explained"
    );
  }

  #[test]
  fn an_encrypted_meter_plus_payload_is_also_refused() {
    assert_eq!(
      decode_service_data(
        FD3D,
        &frame_bytes(ENCRYPTED_PAYLOAD_FLAG | 0x69, 5, 0x80 | 24, 48)
      ),
      MeterAdvertisement::Encrypted(SwitchBotMeterModel::MeterPlus)
    );
  }

  /// Refusal is driven by the flag alone, so the same payload without it
  /// still reads normally. Without this pairing the test above would
  /// pass just as well if the decoder had broken outright.
  #[test]
  fn clearing_the_encryption_flag_makes_the_same_payload_readable() {
    let plain = frame_bytes(0x54, 5, 0x80 | 24, 48);
    let encrypted = frame_bytes(ENCRYPTED_PAYLOAD_FLAG | 0x54, 5, 0x80 | 24, 48);

    assert_eq!(reading(FD3D, &plain).unwrap().temperature_celsius, 24.5);
    assert_eq!(reading(FD3D, &encrypted), None);
  }

  /// An encrypted frame from something that is not a meter stays
  /// `Ignored`: there is nothing to explain to the user about a Bot.
  #[test]
  fn an_encrypted_non_meter_is_ignored_rather_than_reported() {
    assert_eq!(
      decode_service_data(
        FD3D,
        &frame_bytes(ENCRYPTED_PAYLOAD_FLAG | 0x48, 5, 0x80 | 24, 48)
      ),
      MeterAdvertisement::Ignored
    );
  }

  #[test]
  fn another_switchbot_product_on_the_same_service_uuid_is_not_a_meter() {
    // 'H' (0x48) is the Bot, which broadcasts under the same UUID.
    assert_eq!(reading(FD3D, &frame_bytes(0x48, 5, 0x80 | 24, 48)), None);
  }

  // -- temperature --

  #[test]
  fn a_whole_number_temperature_decodes_without_a_fraction() {
    let frame = reading(FD3D, &meter_at(0, 0x80 | 21)).unwrap();
    assert_eq!(frame.temperature_celsius, 21.0);
  }

  /// The polarity trap: the vendor document defines the flag as 0 =
  /// subzero, 1 = above zero, which is the reverse of the usual "sign
  /// bit set means negative" reading.
  #[test]
  fn a_subzero_temperature_is_negative_when_the_flag_is_clear() {
    let frame = reading(FD3D, &meter_at(5, 3)).unwrap();
    assert_eq!(frame.temperature_celsius, -3.5);
  }

  #[test]
  fn zero_degrees_decodes_as_zero_in_both_sign_states() {
    assert_eq!(
      reading(FD3D, &meter_at(0, 0x80))
        .unwrap()
        .temperature_celsius,
      0.0
    );
    assert_eq!(
      reading(FD3D, &meter_at(0, 0x00))
        .unwrap()
        .temperature_celsius,
      0.0
    );
  }

  #[test]
  fn the_documented_temperature_range_ends_decode() {
    let hottest = reading(FD3D, &meter_at(9, 0x80 | 127)).unwrap();
    assert_eq!(hottest.temperature_celsius, 127.9);

    let coldest = reading(FD3D, &meter_at(9, 127)).unwrap();
    assert_eq!(coldest.temperature_celsius, -127.9);
  }

  /// Regression guard for the classic SwitchBot decode bug: byte 5
  /// bit[7] reports which unit the meter's own screen shows, not which
  /// unit it broadcast. Converting on it would report 76.1 °C for a
  /// 24.5 °C room.
  #[test]
  fn a_meter_set_to_show_fahrenheit_still_broadcasts_celsius() {
    let celsius_display = reading(FD3D, &meter_at(5, 0x80 | 24)).unwrap();
    let fahrenheit_display =
      reading(FD3D, &frame_bytes(0x54, 5, 0x80 | 24, 0x80 | 48)).unwrap();

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
    let frame = reading(FD3D, &alerting).unwrap();
    assert_eq!(frame.temperature_celsius, 24.5);
  }

  /// A decimal digit above 9 is outside the documented encoding, so the
  /// payload is not the frame we think it is.
  #[test]
  fn an_out_of_range_temperature_decimal_refuses_the_whole_frame() {
    assert_eq!(reading(FD3D, &meter_at(0x0A, 0x80 | 24)), None);
  }

  // -- humidity --

  #[test]
  fn humidity_decodes_from_the_low_seven_bits() {
    let frame = reading(FD3D, &meter_at(5, 0x80 | 24)).unwrap();
    assert_eq!(frame.humidity_percent, Some(48.0));
  }

  #[test]
  fn the_documented_humidity_range_ends_decode() {
    for (byte, expected) in [(0u8, 0.0f32), (99, 99.0)] {
      let frame = reading(FD3D, &frame_bytes(0x54, 5, 0x80 | 24, byte)).unwrap();
      assert_eq!(frame.humidity_percent, Some(expected));
    }
  }

  /// Humidity is the optional half of the reading, so a value outside
  /// the documented range costs the humidity and not the temperature
  /// the archive actually needs.
  #[test]
  fn an_out_of_range_humidity_drops_only_the_humidity() {
    let frame = reading(FD3D, &frame_bytes(0x54, 5, 0x80 | 24, 100)).unwrap();
    assert_eq!(frame.humidity_percent, None);
    assert_eq!(frame.temperature_celsius, 24.5);
  }

  // -- payload length --

  #[test]
  fn a_payload_too_short_to_hold_a_reading_is_ignored() {
    let full = meter_at(5, 0x80 | 24);
    for length in 0..MINIMUM_SERVICE_DATA_LEN {
      assert_eq!(
        reading(FD3D, &full[..length]),
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
    let frame = reading(FD3D, &padded).unwrap();
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
      reading(FD3D, &advertisement),
      Some(SwitchBotMeterFrame {
        model: SwitchBotMeterModel::Meter,
        temperature_celsius: 26.7,
        humidity_percent: Some(53.0),
      })
    );
  }

  // -- manufacturer data: the Hub family --
  //
  // Every payload below is a real capture from 2026-09-02, and every
  // expected value is what the SwitchBot app displayed for that device
  // at the time (see `reading_offset`). These are the observation
  // record: if the offsets are ever wrong, these fail.

  fn ambient(company_id: u16, data: &[u8]) -> Option<SwitchBotAmbientFrame> {
    match decode_manufacturer_data(company_id, data) {
      ManufacturerAdvertisement::Reading(frame) => Some(frame),
      ManufacturerAdvertisement::Ignored => None,
    }
  }

  #[test]
  fn a_captured_hub_3_payload_decodes_to_its_displayed_reading() {
    let captured = [
      0xb0, 0xe9, 0xfe, 0xb0, 0x8a, 0x19, 0x00, 0xff, 0x6a, 0x97, 0x08, 0x55, 0x84, 0x03,
      0x9b, 0x32, 0x80,
    ];
    assert_eq!(
      ambient(SWITCHBOT_COMPANY_ID, &captured),
      Some(SwitchBotAmbientFrame {
        address: [0xb0, 0xe9, 0xfe, 0xb0, 0x8a, 0x19],
        temperature_celsius: 27.3,
        humidity_percent: Some(50.0),
      })
    );
  }

  #[test]
  fn a_captured_hub_2_payload_decodes_to_its_displayed_reading() {
    let captured = [
      0xd0, 0x51, 0xfa, 0x0f, 0x2c, 0xd0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x00,
      0x99, 0x36, 0x00,
    ];
    assert_eq!(
      ambient(SWITCHBOT_COMPANY_ID, &captured),
      Some(SwitchBotAmbientFrame {
        address: [0xd0, 0x51, 0xfa, 0x0f, 0x2c, 0xd0],
        temperature_celsius: 25.0,
        humidity_percent: Some(54.0),
      })
    );
  }

  #[test]
  fn a_captured_co2_meter_payload_decodes_to_its_displayed_reading() {
    let captured = [
      0xb0, 0xe9, 0xfe, 0x55, 0xa7, 0xdf, 0x03, 0x64, 0x08, 0x99, 0x33, 0x00, 0x22, 0x05,
      0x4c, 0x20,
    ];
    assert_eq!(
      ambient(SWITCHBOT_COMPANY_ID, &captured),
      Some(SwitchBotAmbientFrame {
        address: [0xb0, 0xe9, 0xfe, 0x55, 0xa7, 0xdf],
        temperature_celsius: 25.8,
        humidity_percent: Some(51.0),
      })
    );
  }

  #[test]
  fn a_payload_from_another_vendor_is_ignored() {
    // Apple's company id, carrying bytes that would otherwise decode.
    let captured = [
      0xb0, 0xe9, 0xfe, 0xb0, 0x8a, 0x19, 0x00, 0xff, 0x6a, 0x97, 0x08, 0x55, 0x84, 0x03,
      0x9b, 0x32, 0x80,
    ];
    assert_eq!(ambient(0x004c, &captured), None);
  }

  #[test]
  fn a_switchbot_device_broadcasting_zeros_reports_no_reading() {
    // A bulb in the same capture. Decoding it would yield -0.0 °C / 0 %,
    // which reads as a measurement and is not one.
    let captured = [
      0xb0, 0xe9, 0xfe, 0xd3, 0xfa, 0xca, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00,
    ];
    assert_eq!(ambient(SWITCHBOT_COMPANY_ID, &captured), None);
  }

  #[test]
  fn an_unobserved_payload_length_is_ignored() {
    // 12 bytes, the shape another SwitchBot device broadcast. We have no
    // observation telling us where a reading would sit, so we do not
    // invent one.
    let captured = [
      0xe5, 0xeb, 0x7e, 0xa4, 0x7a, 0x7a, 0x00, 0xff, 0x6a, 0x97, 0x08, 0x53,
    ];
    assert_eq!(ambient(SWITCHBOT_COMPANY_ID, &captured), None);
  }

  #[test]
  fn a_decimal_digit_above_nine_refuses_the_manufacturer_frame() {
    let mut captured = [
      0xd0, 0x51, 0xfa, 0x0f, 0x2c, 0xd0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x00,
      0x99, 0x36, 0x00,
    ];
    captured[13] = 0x0a;
    assert_eq!(ambient(SWITCHBOT_COMPANY_ID, &captured), None);
  }

  #[test]
  fn a_manufacturer_frame_below_zero_keeps_its_sign() {
    // Same layout with the sign bit clear: the Meter convention reads a
    // cleared bit as below zero, and the hubs shared every other part of
    // it. Unconfirmed on real hardware - the reason this path ships as
    // experimental.
    let mut captured = [
      0xd0, 0x51, 0xfa, 0x0f, 0x2c, 0xd0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x05,
      0x03, 0x36, 0x00,
    ];
    captured[14] = 0x03;
    assert_eq!(
      ambient(SWITCHBOT_COMPANY_ID, &captured),
      Some(SwitchBotAmbientFrame {
        address: [0xd0, 0x51, 0xfa, 0x0f, 0x2c, 0xd0],
        temperature_celsius: -3.5,
        humidity_percent: Some(54.0),
      })
    );
  }
}
