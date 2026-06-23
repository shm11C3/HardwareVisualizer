use serde_json::Value;

/// Best-effort hardware overview details extracted from
/// `system_profiler SPHardwareDataType -json`.
///
/// All fields are optional because the output shape can differ between Intel
/// and Apple Silicon Macs, OS versions, and runtime environments. If a field
/// cannot be extracted reliably, it stays as `None`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpHardwareDetails {
  /// Marketing model name, e.g. "MacBook Pro".
  pub machine_name: Option<String>,
  /// Model identifier, e.g. "Mac16,1".
  pub machine_model: Option<String>,
  /// Regional model / part number, e.g. "Z1DS000MAJ/A".
  pub model_number: Option<String>,
  /// System serial number, e.g. "K2QJ212W6H".
  pub serial_number: Option<String>,
  /// System firmware (boot ROM) version, e.g. "18000.120.36".
  pub boot_rom_version: Option<String>,
}

/// Reads the raw JSON output of `system_profiler SPHardwareDataType -json`.
///
/// This is a low-level macOS hardware access helper that relies on an external
/// command. The output shape can differ depending on the machine (e.g. Intel
/// vs Apple Silicon), system settings, and runtime environment.
pub fn get_raw_sphardware_json() -> Result<String, String> {
  let output = std::process::Command::new("system_profiler")
    .arg("SPHardwareDataType")
    .arg("-json")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).to_string())
  }
}

/// Parses `system_profiler SPHardwareDataType` JSON and extracts best-effort
/// hardware overview details.
///
/// This parser intentionally tolerates missing fields and partial data.
/// If a field cannot be extracted reliably, it stays as `None`.
pub fn parse_sphardware_json(raw: &str) -> Result<SpHardwareDetails, String> {
  let value: Value = serde_json::from_str(raw)
    .map_err(|e| format!("Failed to parse system_profiler JSON: {e}"))?;

  let item = value
    .get("SPHardwareDataType")
    .and_then(|v| v.as_array())
    .and_then(|arr| arr.first())
    .and_then(|v| v.as_object())
    .ok_or_else(|| "system_profiler JSON missing SPHardwareDataType entry".to_string())?;

  let str_field = |key: &str| -> Option<String> {
    item
      .get(key)
      .and_then(|v| v.as_str())
      .map(str::trim)
      .filter(|s| !s.is_empty())
      .map(str::to_string)
  };

  Ok(SpHardwareDetails {
    machine_name: str_field("machine_name"),
    machine_model: str_field("machine_model"),
    model_number: str_field("model_number"),
    serial_number: str_field("serial_number"),
    boot_rom_version: str_field("boot_rom_version"),
  })
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  #[test]
  fn parse_sphardware_json_apple_silicon_shape_extracts_fields() {
    let raw = r#"{
      "SPHardwareDataType" : [
        {
          "_name" : "hardware_overview",
          "boot_rom_version" : "18000.120.36",
          "chip_type" : "Apple M4",
          "machine_model" : "Mac16,1",
          "machine_name" : "MacBook Pro",
          "model_number" : "Z1DS000MAJ/A",
          "physical_memory" : "24 GB",
          "serial_number" : "K2QJ212W6H"
        }
      ]
    }"#;

    let details = parse_sphardware_json(raw).expect("parse should succeed");
    assert_eq!(details.machine_name.as_deref(), Some("MacBook Pro"));
    assert_eq!(details.machine_model.as_deref(), Some("Mac16,1"));
    assert_eq!(details.model_number.as_deref(), Some("Z1DS000MAJ/A"));
    assert_eq!(details.serial_number.as_deref(), Some("K2QJ212W6H"));
    assert_eq!(details.boot_rom_version.as_deref(), Some("18000.120.36"));
  }

  #[test]
  fn parse_sphardware_json_missing_fields_stay_none() {
    let raw = r#"{
      "SPHardwareDataType" : [
        {
          "_name" : "hardware_overview",
          "machine_name" : "MacBook Pro",
          "serial_number" : "   "
        }
      ]
    }"#;

    let details = parse_sphardware_json(raw).expect("parse should succeed");
    assert_eq!(details.machine_name.as_deref(), Some("MacBook Pro"));
    assert_eq!(details.machine_model, None);
    assert_eq!(details.model_number, None);
    // Whitespace-only values are treated as missing.
    assert_eq!(details.serial_number, None);
    assert_eq!(details.boot_rom_version, None);
  }

  #[test]
  fn parse_sphardware_json_missing_array_errors() {
    let raw = r#"{ "SPSomethingElse" : [] }"#;
    assert!(parse_sphardware_json(raw).is_err());
  }

  #[test]
  fn get_raw_sphardware_json_runs_on_macos() {
    let raw = get_raw_sphardware_json().expect("system_profiler should run");
    assert!(raw.contains("SPHardwareDataType"));
  }
}
