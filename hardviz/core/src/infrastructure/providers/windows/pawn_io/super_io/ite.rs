//! Scoped ITE Environment Controller profiles and pure decoding helpers.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-ite-it86xx-it87xx.md` revision 2
//! - `docs/specs/sensors/superio-ite-primary-lpc-expansion.md` revision 8
//! - `docs/specs/sensors/superio-ite-it8728f-ex-ga-b75m-d3h-fans.md` revision 2
//!
//! No other external sensor source was used.

use super::is_plausible_super_io_temperature;

/// TMPIN byte registers with their one-based channel for the CR51 eligibility
/// check.
const ITE_TEMPERATURE_REGISTERS: &[(&str, u8, u8)] = &[
  ("TMPIN1", 0x29, 1),
  ("TMPIN2", 0x2A, 2),
  ("TMPIN3", 0x2B, 3),
];

/// Fan counter low / high registers in channel order. The fan name is the key
/// in the live display and archive, so a missing input must not shift the
/// names of its siblings.
const ITE_FAN_REGISTERS: &[(&str, u8, u8)] = &[
  ("FAN1", 0x0D, 0x18),
  ("FAN2", 0x0E, 0x19),
  ("FAN3", 0x0F, 0x1A),
];

/// The IT8728F DX/EX primary controller reads only its first two counters,
/// under the generic motherboard fan names.
const ITE_IT8728F_DX_EX_FAN_REGISTERS: &[(&str, u8, u8)] =
  &[("Fan 1", 0x0D, 0x18), ("Fan 2", 0x0E, 0x19)];

pub(super) const ITE_EC_FAN_MODE_REGISTER: u8 = 0x0C;

/// How the Environment Controller base is derived from CR60/61 and validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IteEcBaseRule {
  /// The raw CR60/61 value, 8-byte aligned inside `0x0100..=0x0FF0` and not
  /// `0x0CF8`.
  Modern,
  /// CR60/61 masked to `0x0FF8`, with the legacy validity checks.
  Legacy,
}

/// The register procedure a profile follows. The ordered steps of each
/// procedure stay in code; this only selects which one applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IteEcAccess {
  /// One monitoring-state gate per sample, then the profile's inputs.
  Unbanked,
  /// Bank-0 reads bracketed by EC `0x06` bank checks, with every temperature
  /// read twice and no source-selection read. The bank is never written.
  BankGuarded,
}

/// A chip revision whose CR22 code selects a profile with more inputs than the
/// chip row alone.
///
/// Unlike [`IteEcProfile::required_revision`], a mismatch or an unreadable CR22
/// keeps the chip row, so the row's own inputs stay available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IteRevisionVariant {
  pub(super) slot: u8,
  /// The CR22 low nibble that selects the variant.
  pub(super) revision: u8,
  pub(super) profile: &'static IteEcProfile,
}

/// One scoped ITE Environment Controller read profile, selected by raw chip ID.
/// Every scoped ITE profile is Experimental.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IteEcProfile {
  /// Chip label surfaced in diagnostics, failure text and the source label.
  pub(super) chip_label: &'static str,
  /// Slots on which the raw chip ID selects this row.
  pub(super) slots: &'static [u8],
  pub(super) chip_id: u16,
  /// CR22 low nibble that must match before logical-device selection.
  pub(super) required_revision: Option<u8>,
  /// Whether discovery reads CR22 before logical-device selection for
  /// diagnostics only. Neither its value nor a failed read selects or refuses
  /// the profile.
  pub(super) reports_diagnostic_revision: bool,
  pub(super) ec_base_rule: IteEcBaseRule,
  /// Whether EC `0x00` bit 7 rejects the controller during discovery.
  pub(super) discovery_rejects_configuration_bit7: bool,
  /// Whether EC `0x00` bit 7 rejects a sample.
  pub(super) sampling_rejects_configuration_bit7: bool,
  /// Whether discovery requires EC `0x58` to hold the ITE vendor byte.
  pub(super) checks_vendor_byte: bool,
  /// Whether the FAN2 / FAN3 routing registers are read with the GPIO logical
  /// device selected, after which the EC logical device is reselected.
  pub(super) routes_in_gpio_ldn: bool,
  /// Temperature inputs; empty when the profile reads none.
  pub(super) temperature_registers: &'static [(&'static str, u8, u8)],
  /// Fan inputs in channel order; empty when the profile reads none.
  pub(super) fan_registers: &'static [(&'static str, u8, u8)],
  /// Whether FAN1 is a tachometer input. FAN2 and FAN3 follow their routing
  /// registers.
  pub(super) fan1_routed: bool,
  /// The EC register gating counters per channel, if the profile has one.
  pub(super) fan_mode_register: Option<u8>,
  pub(super) revision_variant: Option<IteRevisionVariant>,
  pub(super) access: IteEcAccess,
}

const MODERN: IteEcProfile = IteEcProfile {
  chip_label: "",
  slots: &[0],
  chip_id: 0,
  required_revision: None,
  reports_diagnostic_revision: false,
  access: IteEcAccess::Unbanked,
  ec_base_rule: IteEcBaseRule::Modern,
  discovery_rejects_configuration_bit7: true,
  sampling_rejects_configuration_bit7: true,
  checks_vendor_byte: true,
  routes_in_gpio_ldn: true,
  temperature_registers: ITE_TEMPERATURE_REGISTERS,
  fan_registers: ITE_FAN_REGISTERS,
  fan1_routed: true,
  fan_mode_register: None,
  revision_variant: None,
};

const LEGACY: IteEcProfile = IteEcProfile {
  ec_base_rule: IteEcBaseRule::Legacy,
  discovery_rejects_configuration_bit7: false,
  sampling_rejects_configuration_bit7: false,
  checks_vendor_byte: false,
  routes_in_gpio_ldn: false,
  ..MODERN
};

pub(super) const IT8620E: IteEcProfile = IteEcProfile {
  chip_label: "IT8620E",
  chip_id: 0x8620,
  ..MODERN
};

pub(super) const IT8628E: IteEcProfile = IteEcProfile {
  chip_label: "IT8628E",
  chip_id: 0x8628,
  ..MODERN
};

pub(super) const IT8716F_D: IteEcProfile = IteEcProfile {
  chip_label: "IT8716F D",
  chip_id: 0x8716,
  required_revision: Some(0x01),
  temperature_registers: &[],
  fan_mode_register: Some(ITE_EC_FAN_MODE_REGISTER),
  ..LEGACY
};

/// IT8718F FAN1-3 on the IT8716F D mechanism, limited to the inputs whose
/// 16-bit counter firmware enabled in EC `0x0C` bits 0-2. An input left in
/// 8-bit mode has no documented divisor and is omitted; EC `0x0C` is never
/// written. CR22 is recorded for diagnostics and never gates.
pub(super) const IT8718F: IteEcProfile = IteEcProfile {
  chip_label: "IT8718F",
  chip_id: 0x8718,
  reports_diagnostic_revision: true,
  sampling_rejects_configuration_bit7: true,
  checks_vendor_byte: true,
  temperature_registers: &[],
  fan_mode_register: Some(ITE_EC_FAN_MODE_REGISTER),
  ..LEGACY
};

pub(super) const IT8728F: IteEcProfile = IteEcProfile {
  chip_label: "IT8728F/EX",
  slots: &[0, 1],
  chip_id: 0x8728,
  fan_registers: &[],
  fan1_routed: false,
  // CR22[3:0] = 0x1 is the code DX and EX share; AX-CX report 0x0.
  revision_variant: Some(IteRevisionVariant {
    slot: 0,
    revision: 0x01,
    profile: &IT8728F_DX_EX,
  }),
  ..LEGACY
};

/// Reached only through [`IT8728F`]'s revision variant.
pub(super) const IT8728F_DX_EX: IteEcProfile = IteEcProfile {
  chip_label: "IT8728F/EX",
  slots: &[0],
  chip_id: 0x8728,
  fan_registers: ITE_IT8728F_DX_EX_FAN_REGISTERS,
  ..LEGACY
};

/// FX has only the FAN2 and FAN3 tachometer inputs, regardless of EC 0x13[4].
pub(super) const IT8772E_FX: IteEcProfile = IteEcProfile {
  chip_label: "IT8772E FX",
  chip_id: 0x8772,
  required_revision: Some(0x02),
  sampling_rejects_configuration_bit7: true,
  checks_vendor_byte: true,
  temperature_registers: &[],
  fan1_routed: false,
  ..LEGACY
};

/// The banked modern profile: the IT8620E discovery and base rules with the
/// per-sample bank bracket. CR22 is recorded for diagnostics and never gates.
const BANKED_MODERN: IteEcProfile = IteEcProfile {
  access: IteEcAccess::BankGuarded,
  reports_diagnostic_revision: true,
  ..MODERN
};

pub(super) const IT8686E: IteEcProfile = IteEcProfile {
  chip_label: "IT8686E",
  chip_id: 0x8686,
  ..BANKED_MODERN
};

pub(super) const IT8688E: IteEcProfile = IteEcProfile {
  chip_label: "IT8688E",
  chip_id: 0x8688,
  ..BANKED_MODERN
};

pub(super) const IT8689E: IteEcProfile = IteEcProfile {
  chip_label: "IT8689E",
  chip_id: 0x8689,
  ..BANKED_MODERN
};

/// Raw `0x8721` (IT8721F DX) has no row: its specification row is disabled, so
/// discovery exits right after the chip-ID read.
const ITE_EC_PROFILES: [&IteEcProfile; 9] = [
  &IT8620E,
  &IT8628E,
  &IT8686E,
  &IT8688E,
  &IT8689E,
  &IT8716F_D,
  &IT8718F,
  &IT8728F,
  &IT8772E_FX,
];

/// Match the raw chip ID exactly. A profile with a required revision still
/// needs the CR22 check before logical-device selection.
pub(super) fn ite_ec_profile(slot: u8, id: u16) -> Option<&'static IteEcProfile> {
  ITE_EC_PROFILES
    .into_iter()
    .find(|profile| profile.chip_id == id && profile.slots.contains(&slot))
}

/// Decode a scoped ITE Environment Controller temperature byte as signed
/// degrees C.
pub const fn decode_ite_temperature_byte(raw: u8) -> f32 {
  (raw as i8) as f32
}

/// Whether a temperature is inside the scoped ITE plausibility range.
/// Zero is a valid reading for an enabled physical input.
pub const fn is_plausible_ite_temperature(temperature: f32) -> bool {
  is_plausible_super_io_temperature(temperature)
}

/// EC `0x06` bits that hold the bank field on the banked modern profile. The
/// leads disagree on its width (bits 6:5 or 7:5); requiring all three clear
/// satisfies both.
const ITE_EC_BANK_FIELD_MASK: u8 = 0xE0;

/// Whether an EC `0x06` byte shows bank 0 on the banked modern profile.
pub(super) const fn is_ite_ec_bank_zero(bank_select: u8) -> bool {
  bank_select & ITE_EC_BANK_FIELD_MASK == 0
}

const BANKED_ITE_MIN_TEMPERATURE: f32 = 1.0;

/// Whether a banked-modern temperature is inside +1 to +125 degrees C
/// inclusive. The profile cannot read the source selection, so zero and below
/// (an unconfigured input reads -55) are not readings.
pub(super) const fn is_plausible_banked_ite_temperature(temperature: f32) -> bool {
  temperature >= BANKED_ITE_MIN_TEMPERATURE && is_plausible_ite_temperature(temperature)
}

/// Whether one TMPIN is an enabled physical temperature input.
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

/// Convert one stable 16-bit count after all profile and channel gates passed.
pub(super) const fn decode_ite_fan_rpm(count: u16) -> Option<u32> {
  match count {
    0..=0x003F => None,
    0xFFFF => Some(0),
    _ => Some(675_000 / count as u32),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn profiles_are_selected_by_slot_and_exact_chip_id() {
    let label = |slot, id| ite_ec_profile(slot, id).map(|profile| profile.chip_label);

    assert_eq!(label(0, 0x8620), Some("IT8620E"));
    assert_eq!(label(0, 0x8628), Some("IT8628E"));
    assert_eq!(label(0, 0x8716), Some("IT8716F D"));
    assert_eq!(label(0, 0x8718), Some("IT8718F"));
    assert_eq!(label(0, 0x8728), Some("IT8728F/EX"));
    assert_eq!(label(1, 0x8728), Some("IT8728F/EX"));
    assert_eq!(label(0, 0x8772), Some("IT8772E FX"));
    assert_eq!(label(1, 0x8620), None);
    assert_eq!(label(1, 0x8718), None);
    // The IT8721F DX row of revision 8 is a disabled candidate.
    assert_eq!(label(0, 0x8721), None);
    assert_eq!(label(1, 0x8721), None);
  }

  /// `superio-ite-primary-lpc-expansion.md` rev 8, "Discovery and
  /// authorization" IT8718F row and the "IT8718F profile" register table.
  #[test]
  fn it8718f_reuses_the_it8716f_d_mechanism_without_a_revision_gate() {
    let profile = ite_ec_profile(0, 0x8718).unwrap();

    assert_eq!(profile.required_revision, None);
    assert!(profile.reports_diagnostic_revision);
    assert_eq!(profile.access, IteEcAccess::Unbanked);
    assert_eq!(profile.ec_base_rule, IT8716F_D.ec_base_rule);
    assert_eq!(profile.routes_in_gpio_ldn, IT8716F_D.routes_in_gpio_ldn);
    assert_eq!(profile.fan_registers, IT8716F_D.fan_registers);
    assert!(profile.fan1_routed);
    assert_eq!(profile.fan_mode_register, Some(ITE_EC_FAN_MODE_REGISTER));
    assert!(profile.temperature_registers.is_empty());
    assert!(profile.checks_vendor_byte);
    assert!(profile.sampling_rejects_configuration_bit7);
    assert!(!profile.discovery_rejects_configuration_bit7);
  }

  #[test]
  fn banked_modern_profiles_are_exact_slot_zero_rows() {
    let profile = |slot, id| ite_ec_profile(slot, id).map(|profile| profile.chip_label);

    for (id, label) in [
      (0x8686, "IT8686E"),
      (0x8688, "IT8688E"),
      (0x8689, "IT8689E"),
    ] {
      assert_eq!(profile(0, id), Some(label));
      assert_eq!(profile(1, id), None);
      let row = ite_ec_profile(0, id).unwrap();
      assert_eq!(row.access, IteEcAccess::BankGuarded);
      assert_eq!(row.required_revision, None);
      assert!(row.reports_diagnostic_revision);
      assert_eq!(row.fan_mode_register, None);
    }
    for id in [0x8696, 0x8698, 0x8665, 0x8655, 0x8625, 0x8613] {
      assert_eq!(profile(0, id), None, "0x{id:04X}");
    }
    assert!(
      [IT8620E, IT8628E, IT8716F_D, IT8718F, IT8728F, IT8772E_FX]
        .iter()
        .all(|profile| profile.access == IteEcAccess::Unbanked)
    );
  }

  #[test]
  fn ite_ec_bank_zero_requires_bits_7_to_5_clear() {
    assert!(is_ite_ec_bank_zero(0x00));
    assert!(is_ite_ec_bank_zero(0x1F));
    for value in [0x20, 0x40, 0x60, 0x80, 0xE0] {
      assert!(!is_ite_ec_bank_zero(value), "0x{value:02X}");
    }
  }

  #[test]
  fn banked_ite_temperature_accepts_only_plus_1_to_plus_125() {
    for (raw, accepted) in [
      (0xC9, false),
      (0x00, false),
      (0x80, false),
      (0x7E, false),
      (0x7D, true),
      (0x01, true),
    ] {
      assert_eq!(
        is_plausible_banked_ite_temperature(decode_ite_temperature_byte(raw)),
        accepted,
        "0x{raw:02X}"
      );
    }
  }

  #[test]
  fn the_revision_variant_is_reached_only_through_its_chip_row() {
    assert!(
      ITE_EC_PROFILES
        .iter()
        .all(|profile| **profile != IT8728F_DX_EX)
    );
    let variant = IT8728F
      .revision_variant
      .expect("IT8728F has a revision variant");
    assert_eq!(variant.profile, &IT8728F_DX_EX);
    assert_eq!(variant.profile.chip_label, IT8728F.chip_label);
    assert_eq!((variant.slot, variant.revision), (0, 0x01));
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
  fn ite_16_bit_fan_count_distinguishes_invalid_active_and_inactive() {
    assert_eq!(decode_ite_fan_rpm(0x0000), None);
    assert_eq!(decode_ite_fan_rpm(0x0001), None);
    assert_eq!(decode_ite_fan_rpm(0x003F), None);
    assert_eq!(decode_ite_fan_rpm(0x0040), Some(10_546));
    assert_eq!(decode_ite_fan_rpm(0x01C2), Some(1_500));
    assert_eq!(decode_ite_fan_rpm(0xFFFE), Some(10));
    assert_eq!(decode_ite_fan_rpm(0xFFFF), Some(0));
  }
}
