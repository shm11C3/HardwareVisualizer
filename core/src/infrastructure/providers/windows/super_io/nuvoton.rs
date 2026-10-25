//! Pure helpers and the scoped chip-profile table for the Nuvoton NCT67xx
//! normal hardware-monitor decode.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-nuvoton-nct67xx.md` revision 7
//!
//! No other external sensor source was used.

use crate::models::{FanSpeedStatus, SensorEnablement};

/// Decode a Nuvoton bank 4 byte temperature as signed degrees C.
///
/// Implemented from `docs/specs/sensors/superio-nuvoton-nct67xx.md`
/// revision 7, scoped to the enabled normal HM bank 4 byte temperatures.
pub const fn decode_nuvoton_temperature_byte(raw: u8) -> f32 {
  (raw as i8) as f32
}

/// Decode a Nuvoton direct RPM high/low register pair.
///
/// Implemented from `docs/specs/sensors/superio-nuvoton-nct67xx.md`
/// revision 7, which defines the enabled direct RPM path as high byte then low
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

/// The `0xD42A` subset: only AUXTIN1 (`0x93`) and AUXTIN3 (`0x95`) are read
/// because the raw ID cannot distinguish its two candidate identities.
const NUVOTON_D42A_TEMPERATURE_REGISTERS: &[(&str, u8)] =
  &[("AUXTIN1", 0x93), ("AUXTIN3", 0x95)];

/// Direct 16-bit RPM high/low pairs in bank 4. A profile reads only the first
/// `fan_inputs` entries; the seventh pair (`0xCE`/`0xCF`) is populated only on
/// the seven-input lead-only rows.
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
  /// Number of populated direct RPM pairs starting at `0xC0`/`0xC1`.
  pub fan_inputs: usize,
  /// Whether global `CR28` bit 4 gates hardware-monitor availability.
  pub lock_gate: bool,
  /// Bank 4 byte temperature registers read for this profile.
  pub temperature_registers: &'static [(&'static str, u8)],
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

  /// The populated direct RPM pairs, in register order.
  pub fn direct_rpm_registers(&self) -> &'static [(&'static str, u8, u8)] {
    &NUVOTON_DIRECT_RPM_REGISTERS[..self.fan_inputs]
  }
}

/// An inclusive raw chip-ID range mapped to one profile. Exact IDs use equal
/// bounds; only the NCT6779D row spans a range.
struct NuvotonProfileEntry {
  first_id: u16,
  last_id: u16,
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
    lock_gate: false,
    temperature_registers: NUVOTON_ALL_TEMPERATURE_REGISTERS,
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
    lock_gate,
    temperature_registers: NUVOTON_ALL_TEMPERATURE_REGISTERS,
    temperature_filter: NuvotonTemperatureFilter::None,
    hm_base_check: NuvotonHmBaseCheck::Standard,
  }
}

const fn exact_id(id: u16, profile: NuvotonHmProfile) -> NuvotonProfileEntry {
  NuvotonProfileEntry {
    first_id: id,
    last_id: id,
    profile,
  }
}

/// Scoped-enablement rows of `superio-nuvoton-nct67xx.md` revision 7. Any raw
/// ID not listed here, including other members of the lead ranges, stays
/// Unsupported.
const NUVOTON_HM_PROFILES: [NuvotonProfileEntry; 15] = [
  exact_id(0xD802, verified_profile("NCT6799D", "NCT6799D / Super I/O")),
  exact_id(
    0xD421,
    experimental_profile("NCT6796D/NCT6796D-E", 6, false),
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
  exact_id(0xD423, experimental_profile("NCT6796D", 6, true)),
  exact_id(0xD429, experimental_profile("NCT6798D", 7, true)),
  exact_id(
    0xD42A,
    NuvotonHmProfile {
      temperature_registers: NUVOTON_D42A_TEMPERATURE_REGISTERS,
      ..experimental_profile("NCT6796D-R / NCT5585D", 7, true)
    },
  ),
  exact_id(0xD42B, experimental_profile("NCT6798D", 7, true)),
  exact_id(0xD451, experimental_profile("NCT6797D", 7, true)),
  exact_id(0xD801, experimental_profile("NCT6796D-S", 7, true)),
  exact_id(
    0xD806,
    NuvotonHmProfile {
      temperature_filter: NuvotonTemperatureFilter::Nct6701dInvalidBytes,
      hm_base_check: NuvotonHmBaseCheck::Nct6701dUsableBase,
      ..experimental_profile("NCT6701D", 7, true)
    },
  ),
];

/// Every scoped Nuvoton profile, for callers that recognize profile failure
/// text.
pub fn nuvoton_hm_profiles() -> impl Iterator<Item = &'static NuvotonHmProfile> {
  NUVOTON_HM_PROFILES.iter().map(|entry| &entry.profile)
}

/// Select the scoped Nuvoton normal-HM profile for a raw chip ID, or `None`
/// when the ID is outside the enabled runtime scope.
pub fn nuvoton_hm_profile(raw_chip_id: u16) -> Option<&'static NuvotonHmProfile> {
  NUVOTON_HM_PROFILES
    .iter()
    .find(|entry| (entry.first_id..=entry.last_id).contains(&raw_chip_id))
    .map(|entry| &entry.profile)
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

/// Decode a bank 4 byte temperature after the profile's invalid-byte filter.
/// `None` means the input is unavailable for this sample.
pub const fn decode_filtered_nuvoton_temperature_byte(
  raw: u8,
  filter: NuvotonTemperatureFilter,
) -> Option<f32> {
  match filter {
    NuvotonTemperatureFilter::Nct6701dInvalidBytes
      if is_nct6701d_invalid_temperature_byte(raw) =>
    {
      None
    }
    _ => Some(decode_nuvoton_temperature_byte(raw)),
  }
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

  const SIX_PAIRS: [u8; 12] = [
    0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB,
  ];
  const SEVEN_PAIRS: [u8; 14] = [
    0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCE, 0xCF,
  ];
  const ALL_TEMPERATURES: [u8; 6] = [0x90, 0x91, 0x92, 0x93, 0x94, 0x95];

  #[test]
  fn nuvoton_profile_table_matches_the_rev7_scoped_enablement_rows() {
    // (raw id, label, enablement, fan inputs, lock gate)
    let rows: [(u16, &str, SensorEnablement, usize, bool); 15] = [
      (0xD802, "NCT6799D", SensorEnablement::Verified, 6, false),
      (
        0xD421,
        "NCT6796D/NCT6796D-E",
        SensorEnablement::Experimental,
        6,
        false,
      ),
      (0xC560, "NCT6779D", SensorEnablement::Experimental, 5, false),
      (0xC803, "NCT6791D", SensorEnablement::Experimental, 6, true),
      (0xC911, "NCT6792D", SensorEnablement::Experimental, 6, true),
      (0xC913, "NCT6792D", SensorEnablement::Experimental, 6, true),
      (0xD121, "NCT6793D", SensorEnablement::Experimental, 6, true),
      (0xD352, "NCT6795D", SensorEnablement::Experimental, 6, true),
      (0xD423, "NCT6796D", SensorEnablement::Experimental, 6, true),
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
    assert_eq!(nuvoton_hm_profiles().count(), rows.len());
  }

  #[test]
  fn nuvoton_d802_and_d421_profiles_keep_the_six_pair_all_temperature_path() {
    let d802 = nuvoton_hm_profile(0xD802).unwrap();
    let d421 = nuvoton_hm_profile(0xD421).unwrap();

    assert_eq!(d802.source_label, "NCT6799D / Super I/O");
    assert_eq!(d421.source_label, "Nuvoton / Super I/O");
    for profile in [d802, d421] {
      assert_eq!(temperature_register_indexes(profile), ALL_TEMPERATURES);
      assert_eq!(direct_rpm_register_indexes(profile), SIX_PAIRS);
      assert!(!profile.lock_gate);
      assert_eq!(profile.temperature_filter, NuvotonTemperatureFilter::None);
      assert_eq!(profile.hm_base_check, NuvotonHmBaseCheck::Standard);
    }
  }

  #[test]
  fn nuvoton_profiles_read_only_the_populated_rpm_pairs() {
    let nct6779d = nuvoton_hm_profile(0xC563).unwrap();
    assert_eq!(direct_rpm_register_indexes(nct6779d), &SIX_PAIRS[..10]);

    for raw_id in [0xC803, 0xC911, 0xC913, 0xD121, 0xD352, 0xD423] {
      let profile = nuvoton_hm_profile(raw_id).unwrap();
      assert_eq!(
        direct_rpm_register_indexes(profile),
        SIX_PAIRS,
        "0x{raw_id:04X}"
      );
    }
    for raw_id in [0xD429, 0xD42A, 0xD42B, 0xD451, 0xD801, 0xD806] {
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
    for profile in nuvoton_hm_profiles() {
      let expected_temperatures: &[u8] = if profile.chip_label == "NCT6796D-R / NCT5585D"
      {
        &[0x93, 0x95]
      } else {
        &ALL_TEMPERATURES
      };
      assert_eq!(
        temperature_register_indexes(profile),
        expected_temperatures,
        "{}",
        profile.chip_label
      );
      assert!(!temperature_register_indexes(profile).contains(&0x96));
    }

    let d42a = nuvoton_hm_profile(0xD42A).unwrap();
    assert_eq!(
      d42a.temperature_registers,
      &[("AUXTIN1", 0x93), ("AUXTIN3", 0x95)]
    );

    for profile in nuvoton_hm_profiles() {
      let is_nct6701d = std::ptr::eq(profile, nuvoton_hm_profile(0xD806).unwrap());
      assert_eq!(
        profile.temperature_filter == NuvotonTemperatureFilter::Nct6701dInvalidBytes,
        is_nct6701d,
        "{}",
        profile.chip_label
      );
      assert_eq!(
        profile.hm_base_check == NuvotonHmBaseCheck::Nct6701dUsableBase,
        is_nct6701d,
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
    assert_eq!(nuvoton_hm_profile(0xC568), None);
    assert_eq!(nuvoton_hm_profile(0xC56F), None);
  }

  #[test]
  fn nuvoton_unlisted_neighbour_ids_stay_unsupported() {
    for raw_id in [
      0xC568, 0xC800, 0xC807, 0xC910, 0xC912, 0xD120, 0xD350, 0xD420, 0xD422, 0xD428,
      0xD450, 0xD800, 0xD803, 0xD807, 0x8728, 0x0000, 0xFFFF,
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
    assert_eq!(
      decode_filtered_nuvoton_temperature_byte(0x80, NuvotonTemperatureFilter::None),
      Some(-128.0)
    );
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
  fn fan_speed_status_preserves_zero_as_inactive() {
    assert_eq!(fan_speed_status_from_rpm(0), FanSpeedStatus::Inactive);
    assert_eq!(fan_speed_status_from_rpm(1), FanSpeedStatus::Active);
  }
}
