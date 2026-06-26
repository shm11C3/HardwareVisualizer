//! Pure helpers for Super I/O chip-id decoding.
//!
//! These are platform-neutral pure functions so the spec-encoded facts from
//! `docs/specs/sensors/superio-access.md` stay unit-testable on every OS,
//! mirroring [`crate::utils::thermal`]. The Windows `LpcIO` provider performs
//! the actual port I/O and then uses these helpers to interpret the bytes.
//!
//! Implemented from docs/specs/sensors/superio-access.md (chip-id register and
//! configuration-port facts). No other external sensor source was used.

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
}
