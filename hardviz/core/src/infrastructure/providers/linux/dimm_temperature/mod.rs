//! Linux DDR5 DIMM temperature through an already bound `spd5118` hub.
//!
//! Each route follows its own clean-room specification under
//! `docs/specs/sensors/memory-ddr5-*-spd5118.md`. All routes share the explicit
//! detailed inventory request and its dmidecode completion contract.

mod board_profile;
mod inventory;
mod sysfs;
#[cfg(test)]
mod test_fixture;

use std::path::Path;

use crate::models::hardware::DimmTemperature;

pub(crate) fn collect(raw_inventory: &str, stderr_empty: bool) -> DimmTemperature {
  collect_from(Path::new("/sys"), raw_inventory, stderr_empty)
}

fn collect_from(sys: &Path, raw_inventory: &str, stderr_empty: bool) -> DimmTemperature {
  board_profile::collect_from(sys, raw_inventory, stderr_empty)
}

/// Accepts a complete signed decimal millidegree value that is positive, at
/// most 125 000 and a multiple of the driver's 250 m°C step.
fn decode_temperature(value: &str) -> Option<f32> {
  let millidegrees = value.parse::<i32>().ok()?;
  (millidegrees > 0 && millidegrees <= 125_000 && millidegrees % 250 == 0)
    .then_some(millidegrees as f32 / 1000.0)
}
