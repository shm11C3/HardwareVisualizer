//! Pure helpers for Super I/O chip-id decoding.
//!
//! These are platform-neutral pure functions so the spec-encoded facts from
//! `docs/specs/sensors/superio-access.md` stay unit-testable on every OS,
//! mirroring [`crate::utils::thermal`]. The Windows `LpcIO` provider performs
//! the actual port I/O and then uses these helpers to interpret the bytes.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-access.md` revision 3
//! - `docs/specs/sensors/superio-nuvoton-nct67xx.md` revision 8
//! - `docs/specs/sensors/superio-ite-it86xx-it87xx.md` revision 2
//!
//! No other external sensor source was used.

use crate::models::{FanSpeedStatus, SensorEnablement};

/// Inclusive plausibility bounds shared by every Super I/O byte temperature:
/// the ITE revision 2 policy and the Nuvoton revision 8 filter (S1 Table 8-1,
/// +125 down to -55 degrees C) use the same window.
const SUPER_IO_MIN_PLAUSIBLE_TEMPERATURE: f32 = -55.0;
const SUPER_IO_MAX_PLAUSIBLE_TEMPERATURE: f32 = 125.0;

/// Super I/O configuration index/data port pair for an `LpcIO` slot.
///
/// A board straps its Super I/O chip to one of two pairs: slot 0 is the
/// primary `0x2E`/`0x2F` pair and slot 1 is the secondary `0x4E`/`0x4F` pair
/// (`superio-access.md` "Configuration port pairs").
pub const fn slot_ports(slot: u8) -> Option<(u16, u16)> {
  match slot {
    0 => Some((0x2E, 0x2F)),
    1 => Some((0x4E, 0x4F)),
    _ => None,
  }
}

/// Combine the chip-id high (`0x20`) and low (`0x21`) configuration registers
/// into a single 16-bit identifier.
pub const fn chip_id(id_high: u8, id_low: u8) -> u16 {
  ((id_high as u16) << 8) | id_low as u16
}

/// Whether a chip-id reading means "no responding Super I/O chip".
///
/// `superio-access.md` ("Common configuration registers"): reading `0xFF` (or
/// `0x00`) from both id registers means no chip answered on that port pair.
pub const fn is_absent_id(id_high: u8, id_low: u8) -> bool {
  matches!((id_high, id_low), (0x00, 0x00) | (0xFF, 0xFF))
}

/// Decode a Nuvoton bank 4 byte temperature as signed degrees C.
///
/// Implemented from `docs/specs/sensors/superio-nuvoton-nct67xx.md`
/// revision 8, scoped to the enabled normal HM bank 4 byte temperatures.
pub const fn decode_nuvoton_temperature_byte(raw: u8) -> f32 {
  (raw as i8) as f32
}

/// Decode a Nuvoton direct RPM high/low register pair.
///
/// Implemented from `docs/specs/sensors/superio-nuvoton-nct67xx.md`
/// revision 8, which defines the enabled direct RPM path as high byte then low
/// byte, unsigned RPM.
pub const fn decode_nuvoton_direct_rpm(high: u8, low: u8) -> u32 {
  ((high as u32) << 8) | low as u32
}

/// Generic bank 4 byte temperature registers `0x90`-`0x95`.
const NUVOTON_ALL_TEMPERATURE_REGISTERS: &[(&str, u8)] = &[
  ("SYSTIN", 0x90),
  ("CPUTIN", 0x91),
  ("AUXTIN0", 0x92),
  ("AUXTIN1", 0x93),
  ("AUXTIN2", 0x94),
  ("AUXTIN3", 0x95),
];

/// Bank 4 `0x96`, surfaced with the deliberately generic label `Temperature 7`
/// because the spec assigns it no physical input name.
const NUVOTON_TEMPERATURE_7_REGISTER: (&str, u8) = ("Temperature 7", 0x96);

/// Bank 4 `0x90`-`0x96` for the rows whose profile reads the seventh byte.
const NUVOTON_SEVEN_TEMPERATURE_REGISTERS: &[(&str, u8)] = &[
  ("SYSTIN", 0x90),
  ("CPUTIN", 0x91),
  ("AUXTIN0", 0x92),
  ("AUXTIN1", 0x93),
  ("AUXTIN2", 0x94),
  ("AUXTIN3", 0x95),
  NUVOTON_TEMPERATURE_7_REGISTER,
];

/// The `0xD42A` order: revision 7 exposed only AUXTIN1 (`0x93`) and AUXTIN3
/// (`0x95`); revision 8 appends the remaining bank 4 bytes after them in
/// register order so consumers that depend on list position keep working
/// ("Input identity and ordering stability").
const NUVOTON_D42A_TEMPERATURE_REGISTERS: &[(&str, u8)] = &[
  ("AUXTIN1", 0x93),
  ("AUXTIN3", 0x95),
  ("SYSTIN", 0x90),
  ("CPUTIN", 0x91),
  ("AUXTIN0", 0x92),
  ("AUXTIN2", 0x94),
  NUVOTON_TEMPERATURE_7_REGISTER,
];

/// The `0xD802` Experimental sub-scope temperatures, read after the Verified
/// set of the same sample.
const NUVOTON_D802_EXPERIMENTAL_TEMPERATURE_REGISTERS: &[(&str, u8)] =
  &[NUVOTON_TEMPERATURE_7_REGISTER];

/// Direct 16-bit RPM high/low pairs in bank 4. A profile reads the first
/// `fan_inputs` entries with the row's own enablement and then
/// `experimental_fan_inputs` further entries as an Experimental sub-scope; the
/// seventh pair (`0xCE`/`0xCF`) is populated only on seven-input rows.
const NUVOTON_DIRECT_RPM_REGISTERS: [(&str, u8, u8); 7] = [
  ("Fan 1", 0xC0, 0xC1),
  ("Fan 2", 0xC2, 0xC3),
  ("Fan 3", 0xC4, 0xC5),
  ("Fan 4", 0xC6, 0xC7),
  ("Fan 5", 0xC8, 0xC9),
  ("Fan 6", 0xCA, 0xCB),
  ("Fan 7", 0xCE, 0xCF),
];

const NUVOTON_HM_IO_SPACE_LOCK_BIT: u8 = 1 << 4;
const NCT6701D_MIN_HM_BASE: u16 = 0x0100;
const NCT6701D_INVALID_HM_BASE_BITS: u16 = 0xF007;
const NUVOTON_GENERIC_SOURCE_LABEL: &str = "Nuvoton / Super I/O";
/// The leads classify a raw chip ID into a family by ignoring the low three
/// bits of CR21, which S1 p. 362 describes as the IC version digit.
const NUVOTON_FAMILY_ID_MASK: u16 = 0xFFF8;

/// How raw bank 4 temperature bytes are screened before decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NuvotonTemperatureFilter {
  /// Every raw byte decodes as signed 8-bit degrees C.
  None,
  /// The `0xD806` / NCT6701D invalid-byte filter: `0x00`, `0xA0`, and
  /// `0x7E`-`0x80` are reported unavailable instead of decoded.
  Nct6701dInvalidBytes,
}

/// How the discovered `CR60/61` normal HM base is validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NuvotonHmBaseCheck {
  /// The common non-zero, non-all-ones, non-overflowing base check only.
  Standard,
  /// The `0xD806` / NCT6701D check: additionally reject a base below `0x100`
  /// or with any bit of `0xF007` set. Such a base is reported unavailable;
  /// `CR64/65` is never used as a fallback.
  Nct6701dUsableBase,
}

/// One scoped Nuvoton normal-HM read profile, selected by raw chip ID.
#[derive(Debug, PartialEq, Eq)]
pub struct NuvotonHmProfile {
  /// Chip label surfaced in diagnostics and failure text.
  pub chip_label: &'static str,
  /// Source label attached to successful readings (no experimental marker).
  pub source_label: &'static str,
  /// Verified or Experimental enablement from the spec's scoped tables.
  pub enablement: SensorEnablement,
  /// Number of direct RPM pairs, starting at `0xC0`/`0xC1`, read with the
  /// row's own enablement. A failed read aborts the sample.
  pub fan_inputs: usize,
  /// Number of further direct RPM pairs read as an Experimental sub-scope
  /// after the row's own set. A failed read leaves only the sub-scope inputs
  /// unavailable and keeps the values already read.
  pub experimental_fan_inputs: usize,
  /// Whether global `CR28` bit 4 gates hardware-monitor availability.
  pub lock_gate: bool,
  /// Bank 4 byte temperature registers read with the row's own enablement.
  pub temperature_registers: &'static [(&'static str, u8)],
  /// Bank 4 byte temperature registers read as an Experimental sub-scope after
  /// the row's own set.
  pub experimental_temperature_registers: &'static [(&'static str, u8)],
  /// Raw temperature byte screening applied before decoding.
  pub temperature_filter: NuvotonTemperatureFilter,
  /// Normal HM base validation applied after reading `CR60/61`.
  pub hm_base_check: NuvotonHmBaseCheck,
}

impl NuvotonHmProfile {
  /// Whether failures of this profile are surfaced as experimental.
  pub const fn is_experimental(&self) -> bool {
    matches!(self.enablement, SensorEnablement::Experimental)
  }

  /// The direct RPM pairs read with the row's own enablement, in register
  /// order.
  pub fn direct_rpm_registers(&self) -> &'static [(&'static str, u8, u8)] {
    &NUVOTON_DIRECT_RPM_REGISTERS[..self.fan_inputs]
  }

  /// The direct RPM pairs read as an Experimental sub-scope, in register
  /// order, following [`Self::direct_rpm_registers`].
  pub fn experimental_direct_rpm_registers(&self) -> &'static [(&'static str, u8, u8)] {
    &NUVOTON_DIRECT_RPM_REGISTERS
      [self.fan_inputs..self.fan_inputs + self.experimental_fan_inputs]
  }

  /// Whether this profile reads any Experimental sub-scope register.
  pub const fn has_experimental_sub_scope(&self) -> bool {
    self.experimental_fan_inputs > 0
      || !self.experimental_temperature_registers.is_empty()
  }
}

/// An inclusive raw chip-ID range mapped to one named profile. Exact IDs use
/// equal bounds; only the NCT6779D row spans a range.
struct NuvotonProfileEntry {
  first_id: u16,
  last_id: u16,
  profile: NuvotonHmProfile,
}

/// One lead family value (raw ID masked with [`NUVOTON_FAMILY_ID_MASK`]) and
/// the conservative profile applied to its unnamed versions.
struct NuvotonFamilyEntry {
  family_id: u16,
  profile: NuvotonHmProfile,
}

const fn verified_profile(
  chip_label: &'static str,
  source_label: &'static str,
) -> NuvotonHmProfile {
  NuvotonHmProfile {
    chip_label,
    source_label,
    enablement: SensorEnablement::Verified,
    fan_inputs: 6,
    experimental_fan_inputs: 0,
    lock_gate: false,
    temperature_registers: NUVOTON_ALL_TEMPERATURE_REGISTERS,
    experimental_temperature_registers: &[],
    temperature_filter: NuvotonTemperatureFilter::None,
    hm_base_check: NuvotonHmBaseCheck::Standard,
  }
}

const fn experimental_profile(
  chip_label: &'static str,
  fan_inputs: usize,
  lock_gate: bool,
) -> NuvotonHmProfile {
  NuvotonHmProfile {
    chip_label,
    source_label: NUVOTON_GENERIC_SOURCE_LABEL,
    enablement: SensorEnablement::Experimental,
    fan_inputs,
    experimental_fan_inputs: 0,
    lock_gate,
    temperature_registers: NUVOTON_ALL_TEMPERATURE_REGISTERS,
    experimental_temperature_registers: &[],
    temperature_filter: NuvotonTemperatureFilter::None,
    hm_base_check: NuvotonHmBaseCheck::Standard,
  }
}

/// A seven-input Experimental row that also reads bank 4 `0x96`.
const fn experimental_seven_temperature_profile(
  chip_label: &'static str,
  lock_gate: bool,
) -> NuvotonHmProfile {
  NuvotonHmProfile {
    temperature_registers: NUVOTON_SEVEN_TEMPERATURE_REGISTERS,
    ..experimental_profile(chip_label, 7, lock_gate)
  }
}

/// The `0xD806` / NCT6701D quirks: invalid-byte filter and usable-base check.
const fn with_nct6701d_quirks(profile: NuvotonHmProfile) -> NuvotonHmProfile {
  NuvotonHmProfile {
    temperature_filter: NuvotonTemperatureFilter::Nct6701dInvalidBytes,
    hm_base_check: NuvotonHmBaseCheck::Nct6701dUsableBase,
    ..profile
  }
}

const fn exact_id(id: u16, profile: NuvotonHmProfile) -> NuvotonProfileEntry {
  NuvotonProfileEntry {
    first_id: id,
    last_id: id,
    profile,
  }
}

const fn family(family_id: u16, profile: NuvotonHmProfile) -> NuvotonFamilyEntry {
  NuvotonFamilyEntry { family_id, profile }
}

/// Named rows of `superio-nuvoton-nct67xx.md` revision 8 ("ID-matching
/// policy" step 1): the Verified `0xD802` row with its Experimental
/// sub-scopes, the exact `0xD421` row, the NCT6779D range, and the lead-only
/// exact rows. Matched before the family ranges.
const NUVOTON_HM_PROFILES: [NuvotonProfileEntry; 15] = [
  exact_id(
    0xD802,
    NuvotonHmProfile {
      experimental_fan_inputs: 1,
      experimental_temperature_registers: NUVOTON_D802_EXPERIMENTAL_TEMPERATURE_REGISTERS,
      ..verified_profile("NCT6799D", "NCT6799D / Super I/O")
    },
  ),
  exact_id(
    0xD421,
    experimental_profile("NCT6796D/NCT6796D-E", 7, false),
  ),
  NuvotonProfileEntry {
    first_id: 0xC560,
    last_id: 0xC567,
    profile: experimental_profile("NCT6779D", 5, false),
  },
  exact_id(0xC803, experimental_profile("NCT6791D", 6, true)),
  exact_id(0xC911, experimental_profile("NCT6792D", 6, true)),
  exact_id(0xC913, experimental_profile("NCT6792D", 6, true)),
  exact_id(0xD121, experimental_profile("NCT6793D", 6, true)),
  exact_id(0xD352, experimental_profile("NCT6795D", 6, true)),
  exact_id(0xD423, experimental_profile("NCT6796D", 7, true)),
  exact_id(
    0xD429,
    experimental_seven_temperature_profile("NCT6798D", true),
  ),
  exact_id(
    0xD42A,
    NuvotonHmProfile {
      temperature_registers: NUVOTON_D42A_TEMPERATURE_REGISTERS,
      ..experimental_profile("NCT6796D-R / NCT5585D", 7, true)
    },
  ),
  exact_id(
    0xD42B,
    experimental_seven_temperature_profile("NCT6798D", true),
  ),
  exact_id(0xD451, experimental_profile("NCT6797D", 7, true)),
  exact_id(
    0xD801,
    experimental_seven_temperature_profile("NCT6796D-S", true),
  ),
  exact_id(
    0xD806,
    with_nct6701d_quirks(experimental_profile("NCT6701D", 7, true)),
  ),
];

/// Unrecognized-version range profiles of `superio-nuvoton-nct67xx.md`
/// revision 8 ("ID-matching policy" step 2). Each unnamed version of a lead
/// family range gets the conservative profile of that range: the fewest fan
/// pairs of its named members, the intersection of their temperature sets,
/// and every safety gate any named member applies. The NCT6779D family
/// (`0xC560`) is not listed: its named range already covers `0xC560`-`0xC567`
/// and `0xC568`-`0xC56F` stays Unsupported.
const NUVOTON_UNRECOGNIZED_VERSION_PROFILES: [NuvotonFamilyEntry; 8] = [
  family(
    0xC800,
    experimental_profile("NCT6791D family (unrecognized version)", 6, true),
  ),
  family(
    0xC910,
    experimental_profile("NCT6792D family (unrecognized version)", 6, true),
  ),
  family(
    0xD120,
    experimental_profile("NCT6793D family (unrecognized version)", 6, true),
  ),
  family(
    0xD350,
    experimental_profile("NCT6795D family (unrecognized version)", 6, true),
  ),
  family(
    0xD420,
    experimental_profile("NCT6796D family (unrecognized version)", 7, true),
  ),
  family(
    0xD428,
    experimental_seven_temperature_profile(
      "NCT6798D family (unrecognized version)",
      true,
    ),
  ),
  family(
    0xD450,
    experimental_profile("NCT6797D family (unrecognized version)", 7, true),
  ),
  family(
    0xD800,
    with_nct6701d_quirks(experimental_profile(
      "NCT6799D family (unrecognized version)",
      7,
      true,
    )),
  ),
];

/// Every scoped Nuvoton profile (named rows first, then the unrecognized-version
/// range profiles), for callers that recognize profile failure text.
pub fn nuvoton_hm_profiles() -> impl Iterator<Item = &'static NuvotonHmProfile> {
  NUVOTON_HM_PROFILES
    .iter()
    .map(|entry| &entry.profile)
    .chain(
      NUVOTON_UNRECOGNIZED_VERSION_PROFILES
        .iter()
        .map(|entry| &entry.profile),
    )
}

/// Select the scoped Nuvoton normal-HM profile for a raw chip ID, or `None`
/// when the ID is outside the enabled runtime scope.
///
/// Named IDs resolve first; any other ID whose family value (raw ID masked
/// with `0xFFF8`) is one of the eight lead family ranges resolves to that
/// range's conservative unrecognized-version profile.
pub fn nuvoton_hm_profile(raw_chip_id: u16) -> Option<&'static NuvotonHmProfile> {
  NUVOTON_HM_PROFILES
    .iter()
    .find(|entry| (entry.first_id..=entry.last_id).contains(&raw_chip_id))
    .map(|entry| &entry.profile)
    .or_else(|| {
      let family_id = raw_chip_id & NUVOTON_FAMILY_ID_MASK;
      NUVOTON_UNRECOGNIZED_VERSION_PROFILES
        .iter()
        .find(|entry| entry.family_id == family_id)
        .map(|entry| &entry.profile)
    })
}

/// Whether global `CR28` reports the hardware-monitor I/O space as locked
/// (bit 4 set). Other bits are not interpreted.
pub const fn is_nuvoton_hm_io_space_locked(cr28: u8) -> bool {
  (cr28 & NUVOTON_HM_IO_SPACE_LOCK_BIT) != 0
}

/// Whether a raw `0xD806` / NCT6701D temperature byte is a known invalid value
/// rather than a temperature.
pub const fn is_nct6701d_invalid_temperature_byte(raw: u8) -> bool {
  matches!(raw, 0x00 | 0xA0 | 0x7E..=0x80)
}

/// Whether a `CR60/61` value is a usable `0xD806` / NCT6701D normal HM base.
pub const fn is_nct6701d_usable_hm_base(base: u16) -> bool {
  base >= NCT6701D_MIN_HM_BASE && (base & NCT6701D_INVALID_HM_BASE_BITS) == 0
}

/// Whether a decoded Super I/O byte temperature lies inside the shared
/// plausibility window (-55 to +125 degrees C inclusive). Zero is a valid
/// reading; an implausible value is omitted, never clamped or zeroed.
const fn is_plausible_super_io_temperature(temperature: f32) -> bool {
  temperature >= SUPER_IO_MIN_PLAUSIBLE_TEMPERATURE
    && temperature <= SUPER_IO_MAX_PLAUSIBLE_TEMPERATURE
}

/// Whether a decoded Nuvoton bank 4 byte temperature is plausible.
///
/// Implemented from `docs/specs/sensors/superio-nuvoton-nct67xx.md`
/// revision 8 ("Temperature plausibility bounds", S1 Table 8-1): raw bytes
/// `0x00`-`0x7D` and `0xC9`-`0xFF` are accepted, `0x7E`-`0xC8` are rejected.
/// Applies to every row, including Verified `0xD802`.
pub const fn is_plausible_nuvoton_temperature(temperature: f32) -> bool {
  is_plausible_super_io_temperature(temperature)
}

/// Decode a bank 4 byte temperature after the profile's invalid-byte filter
/// and the plausibility filter. `None` means the input is unavailable for this
/// sample: it is not published as a number, not clamped, not replaced by zero,
/// and does not fail the other inputs.
pub const fn decode_filtered_nuvoton_temperature_byte(
  raw: u8,
  filter: NuvotonTemperatureFilter,
) -> Option<f32> {
  if matches!(filter, NuvotonTemperatureFilter::Nct6701dInvalidBytes)
    && is_nct6701d_invalid_temperature_byte(raw)
  {
    return None;
  }

  let temperature = decode_nuvoton_temperature_byte(raw);
  if is_plausible_nuvoton_temperature(temperature) {
    Some(temperature)
  } else {
    None
  }
}

/// Decode an IT8728F Environment Controller temperature byte as signed
/// degrees C.
///
/// Implemented from `docs/specs/sensors/superio-ite-it86xx-it87xx.md`
/// revision 2, scoped to exact chip id `0x8728` and generic TMPIN1-TMPIN3.
pub const fn decode_ite_temperature_byte(raw: u8) -> f32 {
  (raw as i8) as f32
}

/// Whether an IT8728F temperature is inside the revision 2 plausibility
/// range. Zero is a valid reading for an enabled physical input.
pub const fn is_plausible_ite_temperature(temperature: f32) -> bool {
  is_plausible_super_io_temperature(temperature)
}

/// Whether one IT8728F TMPIN is an enabled physical temperature input.
///
/// `channel` is one-based (`1..=3`). A channel is eligible only when exactly
/// one of its resistor/diode mode bits is set and the SST/PECI route does not
/// target that TMPIN.
pub const fn is_ite_temperature_channel_eligible(config: u8, channel: u8) -> bool {
  let (resistor_bit, diode_bit) = match channel {
    1 => (3, 0),
    2 => (4, 1),
    3 => (5, 2),
    _ => return false,
  };
  let resistor_enabled = (config & (1 << resistor_bit)) != 0;
  let diode_enabled = (config & (1 << diode_bit)) != 0;
  let routed_external_channel = config >> 6;

  resistor_enabled != diode_enabled && routed_external_channel != channel
}

/// Classify a direct RPM value for display.
///
/// A 0 RPM direct reading is valid and means inactive for display; it is not a
/// read failure and does not prove physical disconnection.
pub const fn fan_speed_status_from_rpm(rpm: u32) -> FanSpeedStatus {
  if rpm == 0 {
    FanSpeedStatus::Inactive
  } else {
    FanSpeedStatus::Active
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn slot_ports_match_super_io_config_pairs() {
    assert_eq!(slot_ports(0), Some((0x2E, 0x2F)));
    assert_eq!(slot_ports(1), Some((0x4E, 0x4F)));
  }

  #[test]
  fn slot_ports_reject_unknown_slots() {
    assert_eq!(slot_ports(2), None);
    assert_eq!(slot_ports(0xFF), None);
  }

  #[test]
  fn chip_id_combines_high_and_low_bytes() {
    // ITE chip ids literally encode the part number, e.g. IT8728F -> 0x8728.
    assert_eq!(chip_id(0x87, 0x28), 0x8728);
    assert_eq!(chip_id(0x00, 0x00), 0x0000);
    assert_eq!(chip_id(0xFF, 0xFF), 0xFFFF);
  }

  #[test]
  fn absent_ids_are_zero_or_all_ones() {
    assert!(is_absent_id(0x00, 0x00));
    assert!(is_absent_id(0xFF, 0xFF));
    assert!(!is_absent_id(0x87, 0x28));
    // A mixed reading is a real (present) responder, not "absent".
    assert!(!is_absent_id(0x00, 0xFF));
    assert!(!is_absent_id(0xFF, 0x00));
  }

  #[test]
  fn nuvoton_temperature_byte_decodes_signed_celsius() {
    assert_eq!(decode_nuvoton_temperature_byte(0x27), 39.0);
    assert_eq!(decode_nuvoton_temperature_byte(0x00), 0.0);
    assert_eq!(decode_nuvoton_temperature_byte(0xF0), -16.0);
  }

  #[test]
  fn nuvoton_direct_rpm_combines_high_then_low_bytes() {
    assert_eq!(decode_nuvoton_direct_rpm(0x0B, 0x08), 2824);
    assert_eq!(decode_nuvoton_direct_rpm(0x02, 0xE2), 738);
    assert_eq!(decode_nuvoton_direct_rpm(0x00, 0x00), 0);
  }

  fn temperature_register_indexes(profile: &NuvotonHmProfile) -> Vec<u8> {
    profile
      .temperature_registers
      .iter()
      .map(|(_, register)| *register)
      .collect()
  }

  fn direct_rpm_register_indexes(profile: &NuvotonHmProfile) -> Vec<u8> {
    profile
      .direct_rpm_registers()
      .iter()
      .flat_map(|(_, high, low)| [*high, *low])
      .collect()
  }

  fn experimental_direct_rpm_register_indexes(profile: &NuvotonHmProfile) -> Vec<u8> {
    profile
      .experimental_direct_rpm_registers()
      .iter()
      .flat_map(|(_, high, low)| [*high, *low])
      .collect()
  }

  const SIX_PAIRS: [u8; 12] = [
    0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB,
  ];
  const SEVEN_PAIRS: [u8; 14] = [
    0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCE, 0xCF,
  ];
  const SEVENTH_PAIR: [u8; 2] = [0xCE, 0xCF];
  const ALL_TEMPERATURES: [u8; 6] = [0x90, 0x91, 0x92, 0x93, 0x94, 0x95];
  const SEVEN_TEMPERATURES: [u8; 7] = [0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96];
  /// `0xD42A` keeps its revision 7 inputs first and appends the rest in
  /// register order.
  const D42A_TEMPERATURES: [u8; 7] = [0x93, 0x95, 0x90, 0x91, 0x92, 0x94, 0x96];

  #[test]
  fn nuvoton_profile_table_matches_the_rev8_named_rows() {
    // (raw id, label, enablement, fan inputs, lock gate)
    let rows: [(u16, &str, SensorEnablement, usize, bool); 15] = [
      (0xD802, "NCT6799D", SensorEnablement::Verified, 6, false),
      (
        0xD421,
        "NCT6796D/NCT6796D-E",
        SensorEnablement::Experimental,
        7,
        false,
      ),
      (0xC560, "NCT6779D", SensorEnablement::Experimental, 5, false),
      (0xC803, "NCT6791D", SensorEnablement::Experimental, 6, true),
      (0xC911, "NCT6792D", SensorEnablement::Experimental, 6, true),
      (0xC913, "NCT6792D", SensorEnablement::Experimental, 6, true),
      (0xD121, "NCT6793D", SensorEnablement::Experimental, 6, true),
      (0xD352, "NCT6795D", SensorEnablement::Experimental, 6, true),
      (0xD423, "NCT6796D", SensorEnablement::Experimental, 7, true),
      (0xD429, "NCT6798D", SensorEnablement::Experimental, 7, true),
      (
        0xD42A,
        "NCT6796D-R / NCT5585D",
        SensorEnablement::Experimental,
        7,
        true,
      ),
      (0xD42B, "NCT6798D", SensorEnablement::Experimental, 7, true),
      (0xD451, "NCT6797D", SensorEnablement::Experimental, 7, true),
      (
        0xD801,
        "NCT6796D-S",
        SensorEnablement::Experimental,
        7,
        true,
      ),
      (0xD806, "NCT6701D", SensorEnablement::Experimental, 7, true),
    ];

    for (raw_id, label, enablement, fan_inputs, lock_gate) in rows {
      let profile = nuvoton_hm_profile(raw_id)
        .unwrap_or_else(|| panic!("0x{raw_id:04X} should resolve to a profile"));
      assert_eq!(profile.chip_label, label, "0x{raw_id:04X}");
      assert_eq!(profile.enablement, enablement, "0x{raw_id:04X}");
      assert_eq!(profile.fan_inputs, fan_inputs, "0x{raw_id:04X}");
      assert_eq!(profile.lock_gate, lock_gate, "0x{raw_id:04X}");
      assert_eq!(
        profile.is_experimental(),
        enablement == SensorEnablement::Experimental
      );
    }
    // 15 named rows plus the eight unrecognized-version range profiles.
    assert_eq!(nuvoton_hm_profiles().count(), rows.len() + 8);
  }

  /// Expected conservative profile of one unrecognized-version range.
  struct RangeExpectation {
    unnamed_ids: &'static [u16],
    label: &'static str,
    fan_inputs: usize,
    temperatures: &'static [u8],
    nct6701d_quirks: bool,
  }

  #[test]
  fn nuvoton_unrecognized_versions_resolve_to_the_range_profile() {
    let ranges = [
      RangeExpectation {
        unnamed_ids: &[0xC800, 0xC802, 0xC805, 0xC807],
        label: "NCT6791D family (unrecognized version)",
        fan_inputs: 6,
        temperatures: &ALL_TEMPERATURES,
        nct6701d_quirks: false,
      },
      RangeExpectation {
        unnamed_ids: &[0xC910, 0xC912, 0xC917],
        label: "NCT6792D family (unrecognized version)",
        fan_inputs: 6,
        temperatures: &ALL_TEMPERATURES,
        nct6701d_quirks: false,
      },
      RangeExpectation {
        unnamed_ids: &[0xD120, 0xD127],
        label: "NCT6793D family (unrecognized version)",
        fan_inputs: 6,
        temperatures: &ALL_TEMPERATURES,
        nct6701d_quirks: false,
      },
      RangeExpectation {
        unnamed_ids: &[0xD350, 0xD357],
        label: "NCT6795D family (unrecognized version)",
        fan_inputs: 6,
        temperatures: &ALL_TEMPERATURES,
        nct6701d_quirks: false,
      },
      RangeExpectation {
        unnamed_ids: &[0xD420, 0xD422, 0xD427],
        label: "NCT6796D family (unrecognized version)",
        fan_inputs: 7,
        temperatures: &ALL_TEMPERATURES,
        nct6701d_quirks: false,
      },
      RangeExpectation {
        unnamed_ids: &[0xD428, 0xD42C, 0xD42F],
        label: "NCT6798D family (unrecognized version)",
        fan_inputs: 7,
        temperatures: &SEVEN_TEMPERATURES,
        nct6701d_quirks: false,
      },
      RangeExpectation {
        unnamed_ids: &[0xD450, 0xD457],
        label: "NCT6797D family (unrecognized version)",
        fan_inputs: 7,
        temperatures: &ALL_TEMPERATURES,
        nct6701d_quirks: false,
      },
      RangeExpectation {
        unnamed_ids: &[0xD800, 0xD803, 0xD807],
        label: "NCT6799D family (unrecognized version)",
        fan_inputs: 7,
        temperatures: &ALL_TEMPERATURES,
        nct6701d_quirks: true,
      },
    ];

    for RangeExpectation {
      unnamed_ids,
      label,
      fan_inputs,
      temperatures,
      nct6701d_quirks,
    } in ranges
    {
      for raw_id in unnamed_ids {
        let profile = nuvoton_hm_profile(*raw_id)
          .unwrap_or_else(|| panic!("0x{raw_id:04X} should resolve to a range profile"));
        assert_eq!(profile.chip_label, label, "0x{raw_id:04X}");
        assert_eq!(profile.source_label, "Nuvoton / Super I/O");
        assert!(profile.is_experimental(), "0x{raw_id:04X}");
        assert_eq!(profile.fan_inputs, fan_inputs, "0x{raw_id:04X}");
        assert_eq!(profile.experimental_fan_inputs, 0, "0x{raw_id:04X}");
        assert!(profile.lock_gate, "0x{raw_id:04X}");
        assert_eq!(
          temperature_register_indexes(profile),
          temperatures,
          "0x{raw_id:04X}"
        );
        assert!(profile.experimental_temperature_registers.is_empty());
        assert_eq!(
          profile.temperature_filter == NuvotonTemperatureFilter::Nct6701dInvalidBytes,
          nct6701d_quirks,
          "0x{raw_id:04X}"
        );
        assert_eq!(
          profile.hm_base_check == NuvotonHmBaseCheck::Nct6701dUsableBase,
          nct6701d_quirks,
          "0x{raw_id:04X}"
        );
      }
    }
  }

  #[test]
  fn nuvoton_range_labels_never_equal_a_named_row_label() {
    let named_labels: Vec<&str> = NUVOTON_HM_PROFILES
      .iter()
      .map(|entry| entry.profile.chip_label)
      .collect();
    for entry in &NUVOTON_UNRECOGNIZED_VERSION_PROFILES {
      assert!(
        !named_labels.contains(&entry.profile.chip_label),
        "{}",
        entry.profile.chip_label
      );
      assert!(
        entry
          .profile
          .chip_label
          .ends_with(" family (unrecognized version)")
      );
    }
  }

  #[test]
  fn nuvoton_named_ids_keep_their_exact_profiles_inside_a_range() {
    // Named members are matched before the range fallback.
    assert_eq!(nuvoton_hm_profile(0xD802).unwrap().chip_label, "NCT6799D");
    assert_eq!(nuvoton_hm_profile(0xD806).unwrap().chip_label, "NCT6701D");
    assert_eq!(
      nuvoton_hm_profile(0xD421).unwrap().chip_label,
      "NCT6796D/NCT6796D-E"
    );
    assert_eq!(nuvoton_hm_profile(0xD423).unwrap().chip_label, "NCT6796D");
    assert_eq!(
      nuvoton_hm_profile(0xD42A).unwrap().chip_label,
      "NCT6796D-R / NCT5585D"
    );
  }

  #[test]
  fn nuvoton_d802_reads_the_verified_set_then_the_experimental_sub_scopes() {
    let d802 = nuvoton_hm_profile(0xD802).unwrap();

    assert_eq!(d802.source_label, "NCT6799D / Super I/O");
    assert_eq!(d802.enablement, SensorEnablement::Verified);
    assert_eq!(temperature_register_indexes(d802), ALL_TEMPERATURES);
    assert_eq!(direct_rpm_register_indexes(d802), SIX_PAIRS);
    assert_eq!(experimental_direct_rpm_register_indexes(d802), SEVENTH_PAIR);
    assert_eq!(
      d802.experimental_direct_rpm_registers(),
      &[("Fan 7", 0xCE, 0xCF)]
    );
    assert_eq!(
      d802.experimental_temperature_registers,
      &[("Temperature 7", 0x96)]
    );
    assert!(d802.has_experimental_sub_scope());
    assert!(!d802.lock_gate);
    assert_eq!(d802.temperature_filter, NuvotonTemperatureFilter::None);
    assert_eq!(d802.hm_base_check, NuvotonHmBaseCheck::Standard);
  }

  #[test]
  fn nuvoton_d421_reads_seven_pairs_without_bank4_0x96_or_the_lock_gate() {
    let d421 = nuvoton_hm_profile(0xD421).unwrap();

    assert_eq!(d421.source_label, "Nuvoton / Super I/O");
    assert_eq!(temperature_register_indexes(d421), ALL_TEMPERATURES);
    assert_eq!(direct_rpm_register_indexes(d421), SEVEN_PAIRS);
    assert!(!d421.has_experimental_sub_scope());
    assert!(!d421.lock_gate);
    assert_eq!(d421.temperature_filter, NuvotonTemperatureFilter::None);
    assert_eq!(d421.hm_base_check, NuvotonHmBaseCheck::Standard);
  }

  #[test]
  fn nuvoton_only_d802_has_experimental_sub_scopes() {
    for profile in nuvoton_hm_profiles() {
      let is_d802 = std::ptr::eq(profile, nuvoton_hm_profile(0xD802).unwrap());
      assert_eq!(
        profile.has_experimental_sub_scope(),
        is_d802,
        "{}",
        profile.chip_label
      );
      assert!(
        profile.fan_inputs + profile.experimental_fan_inputs
          <= NUVOTON_DIRECT_RPM_REGISTERS.len(),
        "{}",
        profile.chip_label
      );
    }
  }

  #[test]
  fn nuvoton_profiles_read_only_the_populated_rpm_pairs() {
    let nct6779d = nuvoton_hm_profile(0xC563).unwrap();
    assert_eq!(direct_rpm_register_indexes(nct6779d), &SIX_PAIRS[..10]);

    for raw_id in [0xC803, 0xC911, 0xC913, 0xD121, 0xD352] {
      let profile = nuvoton_hm_profile(raw_id).unwrap();
      assert_eq!(
        direct_rpm_register_indexes(profile),
        SIX_PAIRS,
        "0x{raw_id:04X}"
      );
    }
    for raw_id in [
      0xD421, 0xD423, 0xD429, 0xD42A, 0xD42B, 0xD451, 0xD801, 0xD806,
    ] {
      let profile = nuvoton_hm_profile(raw_id).unwrap();
      assert_eq!(
        direct_rpm_register_indexes(profile),
        SEVEN_PAIRS,
        "0x{raw_id:04X}"
      );
    }
  }

  #[test]
  fn nuvoton_profiles_scope_temperatures_filters_and_base_checks() {
    // Bank 4 0x96 is read exactly on the rows the spec lists for it.
    for raw_id in [0xD429, 0xD42B, 0xD801] {
      let profile = nuvoton_hm_profile(raw_id).unwrap();
      assert_eq!(
        temperature_register_indexes(profile),
        SEVEN_TEMPERATURES,
        "0x{raw_id:04X}"
      );
      assert_eq!(profile.temperature_registers[6], ("Temperature 7", 0x96));
    }
    for raw_id in [
      0xD421, 0xC560, 0xC803, 0xC911, 0xC913, 0xD121, 0xD352, 0xD423, 0xD451, 0xD806,
    ] {
      let profile = nuvoton_hm_profile(raw_id).unwrap();
      assert_eq!(
        temperature_register_indexes(profile),
        ALL_TEMPERATURES,
        "0x{raw_id:04X}"
      );
    }

    let d42a = nuvoton_hm_profile(0xD42A).unwrap();
    assert_eq!(temperature_register_indexes(d42a), D42A_TEMPERATURES);
    assert_eq!(
      &d42a.temperature_registers[..2],
      &[("AUXTIN1", 0x93), ("AUXTIN3", 0x95)]
    );

    for profile in nuvoton_hm_profiles() {
      let is_nct6701d_profile = profile.chip_label == "NCT6701D"
        || profile.chip_label == "NCT6799D family (unrecognized version)";
      assert_eq!(
        profile.temperature_filter == NuvotonTemperatureFilter::Nct6701dInvalidBytes,
        is_nct6701d_profile,
        "{}",
        profile.chip_label
      );
      assert_eq!(
        profile.hm_base_check == NuvotonHmBaseCheck::Nct6701dUsableBase,
        is_nct6701d_profile,
        "{}",
        profile.chip_label
      );
    }
  }

  #[test]
  fn nuvoton_nct6779d_range_matches_only_c560_through_c567() {
    for raw_id in 0xC560..=0xC567 {
      assert_eq!(
        nuvoton_hm_profile(raw_id).map(|profile| profile.chip_label),
        Some("NCT6779D"),
        "0x{raw_id:04X}"
      );
    }
    assert_eq!(nuvoton_hm_profile(0xC55F), None);
    for raw_id in 0xC568..=0xC56F {
      assert_eq!(nuvoton_hm_profile(raw_id), None, "0x{raw_id:04X}");
    }
  }

  #[test]
  fn nuvoton_ids_outside_every_range_stay_unsupported() {
    for raw_id in [
      0xC568, 0xC56F, 0xC7FF, 0xC808, 0xC90F, 0xC918, 0xD11F, 0xD128, 0xD34F, 0xD358,
      0xD41F, 0xD430, 0xD44F, 0xD458, 0xD7FF, 0xD808, 0x8728, 0x0000, 0xFFFF,
    ] {
      assert_eq!(nuvoton_hm_profile(raw_id), None, "0x{raw_id:04X}");
    }
  }

  #[test]
  fn nuvoton_hm_io_space_lock_reads_only_bit_four() {
    assert!(is_nuvoton_hm_io_space_locked(0x10));
    assert!(is_nuvoton_hm_io_space_locked(0xFF));
    assert!(!is_nuvoton_hm_io_space_locked(0x00));
    assert!(!is_nuvoton_hm_io_space_locked(0xEF));
  }

  #[test]
  fn nct6701d_invalid_temperature_bytes_are_filtered() {
    for raw in [0x00, 0xA0, 0x7E, 0x7F, 0x80] {
      assert!(is_nct6701d_invalid_temperature_byte(raw), "0x{raw:02X}");
      assert_eq!(
        decode_filtered_nuvoton_temperature_byte(
          raw,
          NuvotonTemperatureFilter::Nct6701dInvalidBytes
        ),
        None
      );
    }
    for raw in [0x01, 0x27, 0x7D, 0x81, 0x9F, 0xA1, 0xF0] {
      assert!(!is_nct6701d_invalid_temperature_byte(raw), "0x{raw:02X}");
    }
    assert_eq!(
      decode_filtered_nuvoton_temperature_byte(
        0x27,
        NuvotonTemperatureFilter::Nct6701dInvalidBytes
      ),
      Some(39.0)
    );
    // Without the NCT6701D filter, zero is an ordinary 0 C reading.
    assert_eq!(
      decode_filtered_nuvoton_temperature_byte(0x00, NuvotonTemperatureFilter::None),
      Some(0.0)
    );
    // 0x80 (-128 C) is outside the plausibility window on every row.
    assert_eq!(
      decode_filtered_nuvoton_temperature_byte(0x80, NuvotonTemperatureFilter::None),
      None
    );
  }

  #[test]
  fn nuvoton_temperature_plausibility_accepts_the_datasheet_window_inclusive() {
    // S1 Table 8-1 raw examples: 0x7D = +125, 0x00 = 0, 0xC9 = -55.
    for (raw, expected) in [(0x7D, 125.0), (0x00, 0.0), (0xC9, -55.0), (0x19, 25.0)] {
      let temperature = decode_nuvoton_temperature_byte(raw);
      assert_eq!(temperature, expected, "0x{raw:02X}");
      assert!(is_plausible_nuvoton_temperature(temperature), "0x{raw:02X}");
      assert_eq!(
        decode_filtered_nuvoton_temperature_byte(raw, NuvotonTemperatureFilter::None),
        Some(expected),
        "0x{raw:02X}"
      );
    }
  }

  #[test]
  fn nuvoton_temperature_plausibility_rejects_bytes_just_outside_the_window() {
    // 0x7E = +126 and 0xC8 = -56 bound the rejected raw range 0x7E-0xC8.
    for (raw, decoded) in [(0x7E, 126.0), (0xC8, -56.0), (0x7F, 127.0), (0x80, -128.0)] {
      let temperature = decode_nuvoton_temperature_byte(raw);
      assert_eq!(temperature, decoded, "0x{raw:02X}");
      assert!(
        !is_plausible_nuvoton_temperature(temperature),
        "0x{raw:02X}"
      );
      for filter in [
        NuvotonTemperatureFilter::None,
        NuvotonTemperatureFilter::Nct6701dInvalidBytes,
      ] {
        assert_eq!(
          decode_filtered_nuvoton_temperature_byte(raw, filter),
          None,
          "0x{raw:02X} {filter:?}"
        );
      }
    }
  }

  #[test]
  fn nuvoton_plausibility_matches_the_raw_byte_ranges_of_the_spec() {
    for raw in 0x00..=0xFF_u8 {
      let accepted = matches!(raw, 0x00..=0x7D | 0xC9..=0xFF);
      assert_eq!(
        decode_filtered_nuvoton_temperature_byte(raw, NuvotonTemperatureFilter::None)
          .is_some(),
        accepted,
        "0x{raw:02X}"
      );
    }
  }

  #[test]
  fn nct6701d_hm_base_rejects_low_and_misaligned_values() {
    assert!(is_nct6701d_usable_hm_base(0x0290));
    assert!(is_nct6701d_usable_hm_base(0x0100));
    assert!(!is_nct6701d_usable_hm_base(0x0000));
    assert!(!is_nct6701d_usable_hm_base(0x00F8));
    assert!(!is_nct6701d_usable_hm_base(0x0291));
    assert!(!is_nct6701d_usable_hm_base(0x0294));
    assert!(!is_nct6701d_usable_hm_base(0x1290));
    assert!(!is_nct6701d_usable_hm_base(0xFFFF));
  }

  #[test]
  fn ite_temperature_byte_decodes_signed_celsius() {
    assert_eq!(decode_ite_temperature_byte(0x7D), 125.0);
    assert_eq!(decode_ite_temperature_byte(0x19), 25.0);
    assert_eq!(decode_ite_temperature_byte(0x00), 0.0);
    assert_eq!(decode_ite_temperature_byte(0xFF), -1.0);
    assert_eq!(decode_ite_temperature_byte(0xC9), -55.0);
  }

  #[test]
  fn ite_temperature_plausibility_keeps_zero_and_rejects_outside_bounds() {
    assert!(is_plausible_ite_temperature(-55.0));
    assert!(is_plausible_ite_temperature(0.0));
    assert!(is_plausible_ite_temperature(125.0));
    assert!(!is_plausible_ite_temperature(-56.0));
    assert!(!is_plausible_ite_temperature(126.0));
  }

  #[test]
  fn ite_temperature_channel_requires_one_physical_mode_and_no_external_route() {
    assert!(is_ite_temperature_channel_eligible(0b0000_1000, 1));
    assert!(is_ite_temperature_channel_eligible(0b0000_0010, 2));
    assert!(is_ite_temperature_channel_eligible(0b0010_0000, 3));

    assert!(!is_ite_temperature_channel_eligible(0, 1));
    assert!(!is_ite_temperature_channel_eligible(0b0000_1001, 1));
    assert!(!is_ite_temperature_channel_eligible(0b0100_1000, 1));
    assert!(!is_ite_temperature_channel_eligible(0b1000_0010, 2));
    assert!(!is_ite_temperature_channel_eligible(0b1110_0000, 3));
    assert!(!is_ite_temperature_channel_eligible(0xFF, 4));
  }

  #[test]
  fn fan_speed_status_preserves_zero_as_inactive() {
    assert_eq!(fan_speed_status_from_rpm(0), FanSpeedStatus::Inactive);
    assert_eq!(fan_speed_status_from_rpm(1), FanSpeedStatus::Active);
  }
}
