//! Pure helpers for Super I/O configuration access and chip-id decoding.
//!
//! These are pure functions so the spec-encoded facts from
//! `docs/specs/sensors/superio-access.md` stay unit-testable. The `LpcIO`
//! providers in this module perform the actual port I/O and then use these
//! helpers to interpret the bytes.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-access.md` revision 3 (revision 4 for the
//!   slot-1 ITE exit withhold)
//!
//! No other external sensor source was used.

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

/// Raw slot-1 chip IDs for which the ITE configuration-mode exit write is
/// withheld.
///
/// `superio-access.md` revision 4, "Slot-1 ITE exit withhold rule"
/// (Experimental, lead-only facts F1-F3 and F8): `0x8733`, `0x8790`,
/// `0x8695` and the `0x8883` re-enter report. Matching is exact; no mask or
/// range generalization is permitted.
pub const ITE_SLOT1_EXIT_WITHHOLD_CHIP_IDS: [u16; 4] = [0x8733, 0x8790, 0x8695, 0x8883];

/// Whether the ITE exit write (register `0x02`, value `0x02`) must not be
/// issued after reading `chip_id` on `slot`.
///
/// The rule applies only to slot 1 (`0x4E`/`0x4F`); slot 0 always exits.
pub fn should_withhold_ite_exit(slot: u8, chip_id: u16) -> bool {
  slot == 1 && ITE_SLOT1_EXIT_WITHHOLD_CHIP_IDS.contains(&chip_id)
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
  fn ite_exit_is_withheld_for_exact_hazard_ids_on_slot_one() {
    for id in [0x8733, 0x8790, 0x8695, 0x8883] {
      assert!(should_withhold_ite_exit(1, id), "slot 1 id 0x{id:04X}");
    }
  }

  #[test]
  fn ite_exit_is_never_withheld_on_slot_zero() {
    for id in [0x8733, 0x8790, 0x8695, 0x8883] {
      assert!(!should_withhold_ite_exit(0, id), "slot 0 id 0x{id:04X}");
    }
  }

  #[test]
  fn ite_exit_withhold_matches_exactly_without_masks() {
    assert!(!should_withhold_ite_exit(1, 0x8728));
    assert!(!should_withhold_ite_exit(1, 0x8734));
    assert!(!should_withhold_ite_exit(1, 0x8792));
    assert!(!should_withhold_ite_exit(1, 0xFFFF));
  }
}
