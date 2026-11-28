//! Linux DDR5 DIMM temperature through an already bound `spd5118` hub.
//!
//! Each route follows its own clean-room specification under
//! `docs/specs/sensors/memory-ddr5-*-spd5118.md`. All routes share the explicit
//! detailed inventory request and its dmidecode completion contract.

mod board_profile;
mod endpoint_attributes;
mod inventory;
mod sole_module;
#[cfg(test)]
mod test_fixture;

use std::path::Path;

use crate::models::hardware::DimmTemperature;

pub(crate) fn collect(raw_inventory: &str, stderr_empty: bool) -> DimmTemperature {
  collect_from(Path::new("/sys"), raw_inventory, stderr_empty)
}

/// Outcome of the board-profile route.
enum BoardRoute {
  /// The route applies to this system and its result is final.
  Applies(DimmTemperature),
  /// The route does not apply. The status still counts if no other route
  /// produces a reading.
  Declined(DimmTemperature),
}

#[cfg(test)]
impl BoardRoute {
  fn into_temperature(self) -> DimmTemperature {
    match self {
      Self::Applies(temperature) | Self::Declined(temperature) => temperature,
    }
  }
}

/// The Protectli VP2430 route takes precedence over the sole-module route.
/// The ODROID-H4 rev 3 route stays a separate input: either it or the
/// sole-module route may produce the reading.
fn collect_from(sys: &Path, raw_inventory: &str, stderr_empty: bool) -> DimmTemperature {
  match board_profile::route(sys, raw_inventory, stderr_empty) {
    BoardRoute::Applies(temperature) => temperature,
    BoardRoute::Declined(board) => combine(
      board,
      sole_module::collect_from(sys, raw_inventory, stderr_empty),
    ),
  }
}

/// A reading wins. Otherwise a system is unsupported only when every route
/// places it outside its scope.
fn combine(board: DimmTemperature, sole_module: DimmTemperature) -> DimmTemperature {
  match (board, sole_module) {
    (_, available @ DimmTemperature::Available { .. })
    | (available @ DimmTemperature::Available { .. }, _) => available,
    (DimmTemperature::Unsupported, DimmTemperature::Unsupported) => {
      DimmTemperature::Unsupported
    }
    _ => DimmTemperature::Unavailable,
  }
}

/// Accepts a complete signed decimal millidegree value that is positive, at
/// most 125 000 and a multiple of the driver's 250 m°C step.
fn decode_temperature(value: &str) -> Option<f32> {
  let millidegrees = value.parse::<i32>().ok()?;
  (millidegrees > 0 && millidegrees <= 125_000 && millidegrees % 250 == 0)
    .then_some(millidegrees as f32 / 1000.0)
}

#[cfg(test)]
mod tests {
  use super::test_fixture::*;
  use super::*;
  use std::fs;

  /// Bank Locator line that only the sole-module route adds to its label, so
  /// the label shows which route produced a reading.
  const BANK: &str = "\tBank Locator: BANK 0\n";

  fn with_bank(records: &str) -> String {
    records.replace("\tType: DDR5\n", &format!("{BANK}\tType: DDR5\n"))
  }

  fn sole_module_reading(temperature_celsius: f32) -> DimmTemperature {
    DimmTemperature::Available {
      locator: "DIMM 0 / BANK 0".into(),
      temperature_celsius,
    }
  }

  // Spec "Synthetic fixture acceptance": Board `Protectli` / `VP2430` passing
  // the VP2430 gates. VP2430 route applies. No EEPROM read.
  #[test]
  fn a_qualified_vp2430_uses_its_own_route_without_the_eeprom() {
    let fixture = Fixture::protectli();
    let client = fixture.add_client(7, "0050", 0);
    let records = with_bank(INVENTORY);
    // The sole-module route would confirm this identity and add the bank.
    assert_eq!(fixture.collect_sole(&records), sole_module_reading(40.25));
    assert_eq!(fixture.collect_all(&records), available(40.25));

    // The VP2430 result is final once its endpoint qualifies, also when the
    // reading then fails.
    let hwmon = client.join("hwmon/hwmon0");
    write_attr(&hwmon, "temp1_input", "125250");
    assert_eq!(fixture.collect_all(&records), DimmTemperature::Unavailable);
    write_attr(&hwmon, "temp1_input", "40250");

    fs::remove_file(client.join("eeprom")).unwrap();
    assert_eq!(fixture.collect_all(&records), available(40.25));
  }

  #[test]
  fn a_vp2430_failing_its_own_gates_can_still_confirm_its_sole_module() {
    let fixture = Fixture::protectli();
    fixture.add_client(6, "0052", 1);
    let records = with_bank(INVENTORY);
    assert_eq!(fixture.collect(&records), DimmTemperature::Unavailable);
    assert_eq!(fixture.collect_all(&records), sole_module_reading(40.25));
  }

  // Spec "Synthetic fixture acceptance": ODROID-H4 with another firmware
  // version or another module part, otherwise passing every gate. Confirmed
  // and available.
  #[test]
  fn odroid_h4_with_other_firmware_or_part_is_confirmed_by_the_sole_module_route() {
    let fixture = Fixture::new();
    let client = fixture.add_client(7, "0052", 3);
    let records = with_bank(INVENTORY);
    let dmi = fixture.sys.join("class/dmi/id");
    write_attr(&dmi, "bios_version", "Dasharo (coreboot+UEFI) v0.9.2");
    assert_eq!(fixture.collect(&records), DimmTemperature::Unsupported);
    assert_eq!(fixture.collect_all(&records), sole_module_reading(40.25));

    write_attr(&dmi, "bios_version", "Dasharo (coreboot+UEFI) v0.9.1");
    let other_part = "M425R2GA3BB0-CWMOD";
    let mut part = [b' '; 30];
    part[..other_part.len()].copy_from_slice(other_part.as_bytes());
    write_spd(&client, 521, &part);
    let records = records.replace("M425R1GB4BB0-CWMOD", other_part);
    assert_eq!(fixture.collect(&records), DimmTemperature::Unsupported);
    assert_eq!(fixture.collect_all(&records), sole_module_reading(40.25));
  }

  #[test]
  fn odroid_h4_rev3_stays_a_separate_input_for_extra_unbound_clients() {
    let fixture = Fixture::new();
    fixture.add_client(7, "0052", 3);
    fixture.add_unbound_client(8, "0053");
    let records = with_bank(INVENTORY);
    assert_eq!(fixture.collect_sole(&records), DimmTemperature::Unsupported);
    assert_eq!(fixture.collect_all(&records), available(40.25));
  }

  #[test]
  fn other_boards_are_unsupported_only_when_every_route_excludes_them() {
    let fixture = Fixture::generic();
    assert_eq!(fixture.collect_all(INVENTORY), DimmTemperature::Unsupported);
    assert_eq!(
      fixture.collect_all(&INVENTORY.replace("DDR5", "DDR4")),
      DimmTemperature::Unsupported
    );
    assert_eq!(
      collect_from(&fixture.sys, INVENTORY, true),
      DimmTemperature::Unavailable
    );

    let client = fixture.add_client(7, "0052", 3);
    assert_eq!(fixture.collect_all(INVENTORY), available(40.25));
    write_attr(&client.join("hwmon/hwmon3"), "temp1_enable", "0");
    assert_eq!(fixture.collect_all(INVENTORY), DimmTemperature::Unavailable);
  }

  // Spec "Read procedure and decode" step 5 and "Synthetic fixture
  // acceptance": `temp1_input` 0, 250, 125000, 125250, 40251.
  #[test]
  fn decode_accepts_only_positive_quarter_degrees_up_to_125_degrees() {
    for (value, expected) in [
      ("0", None),
      ("250", Some(0.25)),
      ("125000", Some(125.0)),
      ("125250", None),
      ("40251", None),
      ("-250", None),
      ("40250 ", None),
      ("", None),
    ] {
      assert_eq!(decode_temperature(value), expected, "{value:?}");
    }
  }
}
