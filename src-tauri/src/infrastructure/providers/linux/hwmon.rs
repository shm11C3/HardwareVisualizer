///
/// hwmon sysfs provider for AMD GPU temperatures on Linux
///
/// AMD GPUs with the `amdgpu` kernel driver expose temperature sensors
/// under `/sys/class/drm/card{id}/device/hwmon/hwmon*/temp*_input`.
///
/// Each sensor file reports millidegrees Celsius.  An optional
/// `temp*_label` file beside it contains a human-readable label such as
/// "edge", "junction", "mem" etc.
///
use crate::models::hardware::NameValue;
use std::fs;
use std::path::Path;

/// Read GPU temperatures from the hwmon sysfs interface for a given DRM
/// card identified by its sysfs card index (e.g. `0` for `/sys/class/drm/card0`).
///
/// Returns one `NameValue` per sensor where `name` is
/// `"card{id} {label}"` and `value` is in °C (integer).
pub fn read_hwmon_temperatures(card_id: u8) -> Result<Vec<NameValue>, String> {
  let hwmon_dir = find_hwmon_dir(card_id)?;
  let mut temps = Vec::new();

  // Scan for temp*_input files (temp1_input, temp2_input, …)
  let entries = fs::read_dir(&hwmon_dir)
    .map_err(|e| format!("Failed to read hwmon dir {hwmon_dir}: {e}"))?;

  for entry in entries.flatten() {
    let fname = entry.file_name();
    let fname_str = fname.to_string_lossy();

    if !fname_str.starts_with("temp") || !fname_str.ends_with("_input") {
      continue;
    }

    // Read millidegrees value
    let input_path = entry.path();
    let raw = match fs::read_to_string(&input_path) {
      Ok(v) => v,
      Err(_) => continue,
    };
    let millideg: i64 = match raw.trim().parse() {
      Ok(v) => v,
      Err(_) => continue,
    };
    let celsius = (millideg / 1000) as i32;

    // Basic sanity check – reject clearly out-of-range values
    if !(-40..=256).contains(&celsius) {
      continue;
    }

    // Try to read the label (e.g. "edge", "junction", "mem")
    let label_path = input_path.to_string_lossy().replace("_input", "_label");
    let label = fs::read_to_string(&label_path)
      .map(|s| capitalise_label(s.trim()))
      .unwrap_or_else(|_| fname_str.replace("_input", ""));

    temps.push(NameValue {
      name: format!("card{card_id} {label}"),
      value: celsius,
    });
  }

  if temps.is_empty() {
    Err(format!(
      "No temperature sensors found for card{card_id} in {hwmon_dir}"
    ))
  } else {
    Ok(temps)
  }
}

/// Find the first `hwmon*` subdirectory under
/// `/sys/class/drm/card{id}/device/hwmon/`.
fn find_hwmon_dir(card_id: u8) -> Result<String, String> {
  let base = format!("/sys/class/drm/card{card_id}/device/hwmon");
  let base_path = Path::new(&base);

  if !base_path.exists() {
    return Err(format!("hwmon base path does not exist: {base}"));
  }

  let entries =
    fs::read_dir(base_path).map_err(|e| format!("Failed to read {base}: {e}"))?;

  for entry in entries.flatten() {
    let name = entry.file_name();
    let name_str = name.to_string_lossy();
    if name_str.starts_with("hwmon") {
      return Ok(entry.path().to_string_lossy().to_string());
    }
  }

  Err(format!("No hwmon* directory found under {base}"))
}

/// Capitalise first letter of a label string (e.g. "edge" → "Edge").
fn capitalise_label(s: &str) -> String {
  let mut chars = s.chars();
  match chars.next() {
    None => String::new(),
    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_capitalise_label() {
    assert_eq!(capitalise_label("edge"), "Edge");
    assert_eq!(capitalise_label("junction"), "Junction");
    assert_eq!(capitalise_label("mem"), "Mem");
    assert_eq!(capitalise_label(""), "");
  }
}
