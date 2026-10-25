//! Pure helpers for the ITE IT8728F Environment Controller decode.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-ite-it86xx-it87xx.md` revision 2
//!
//! No other external sensor source was used.

use super::is_plausible_super_io_temperature;

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

#[cfg(test)]
mod tests {
  use super::*;

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
}
