use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpMemoryDetails {
  pub clock_mhz: Option<u32>,
  pub memory_count: Option<u32>,
  pub total_slots: Option<u32>,
  pub memory_type: Option<String>,
}

/// Reads the raw JSON output of `system_profiler SPMemoryDataType -json`.
///
/// This is a low-level macOS hardware access helper that relies on an external command.
/// The output shape can differ depending on the machine (e.g. Intel vs Apple Silicon),
/// system settings, and runtime environment.
pub fn get_raw_spmemory_json() -> Result<String, String> {
  let output = std::process::Command::new("system_profiler")
    .arg("SPMemoryDataType")
    .arg("-json")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).to_string())
  }
}

/// Parses `system_profiler` JSON output and extracts best-effort memory module details.
///
/// This parser intentionally tolerates missing fields and partial data.
/// If a field cannot be extracted reliably, it stays as `None`.
pub fn parse_spmemory_json(raw: &str) -> Result<SpMemoryDetails, String> {
  let value: Value = serde_json::from_str(raw)
    .map_err(|e| format!("Failed to parse system_profiler JSON: {e}"))?;

  let items = value
    .get("SPMemoryDataType")
    .and_then(|v| v.as_array())
    .ok_or_else(|| "system_profiler JSON missing SPMemoryDataType array".to_string())?;

  // system_profiler can return various shapes depending on the hardware (Intel vs Apple Silicon).
  // We treat each object in the array as a potential "memory module" entry and best-effort extract
  // dimm_type / dimm_speed / dimm_size if present.

  let mut memory_type: Option<String> = None;
  let mut clock_mhz: Option<u32> = None;

  // For module counting, we only count entries that have an explicit size and are not empty.
  let mut total_slots: u32 = 0;
  let mut installed_count: u32 = 0;

  for item in items {
    let Some(obj) = item.as_object() else {
      continue;
    };

    total_slots = total_slots.saturating_add(1);

    if memory_type.is_none()
      && let Some(typ) = obj.get("dimm_type").and_then(|v| v.as_str())
    {
      let typ = typ.trim();
      if !typ.is_empty() && typ != "Unknown" {
        memory_type = Some(typ.to_string());
      }
    }

    if clock_mhz.is_none()
      && let Some(speed) = obj.get("dimm_speed").and_then(|v| v.as_str())
    {
      clock_mhz = parse_speed_to_mhz(speed);
    }

    if let Some(size) = obj.get("dimm_size").and_then(|v| v.as_str()) {
      let size = size.trim();
      if !size.is_empty() && size != "Empty" && size != "Not Installed" {
        installed_count = installed_count.saturating_add(1);
      }
    }
  }

  let memory_count = if installed_count > 0 {
    Some(installed_count)
  } else {
    None
  };

  let total_slots = if installed_count > 0 && total_slots > 0 {
    Some(total_slots)
  } else {
    None
  };

  Ok(SpMemoryDetails {
    clock_mhz,
    memory_count,
    total_slots,
    memory_type,
  })
}

fn parse_speed_to_mhz(speed: &str) -> Option<u32> {
  // Examples:
  // - "2667 MHz"
  // - "3200 MT/s" (DDR effective rate)
  let speed = speed.trim();
  if speed.is_empty() {
    return None;
  }

  let parts: Vec<&str> = speed.split_whitespace().collect();
  if parts.is_empty() {
    return None;
  }

  let value: u32 = parts[0].parse().ok()?;
  let unit = parts.get(1).copied().unwrap_or("");

  match unit {
    "MHz" => Some(value),
    "MT/s" | "MT/s," | "MT/s)" => Some(value / 2),
    _ => None,
  }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  #[test]
  fn parse_spmemory_json_apple_silicon_shape_extracts_type() {
    let raw = r#"{
      "SPMemoryDataType" : [
        {
          "dimm_manufacturer" : "Hynix",
          "dimm_type" : "LPDDR5",
          "SPMemoryDataType" : "24 GB"
        }
      ]
    }"#;

    let details = parse_spmemory_json(raw).expect("parse should succeed");
    assert_eq!(details.memory_type.as_deref(), Some("LPDDR5"));
    assert_eq!(details.clock_mhz, None);
    assert_eq!(details.memory_count, None);
    assert_eq!(details.total_slots, None);
  }

  #[test]
  fn parse_spmemory_json_intel_shape_extracts_modules_and_speed() {
    let raw = r#"{
      "SPMemoryDataType" : [
        {
          "dimm_size" : "8 GB",
          "dimm_type" : "DDR4",
          "dimm_speed" : "2667 MHz"
        },
        {
          "dimm_size" : "Empty"
        },
        {
          "dimm_size" : "8 GB",
          "dimm_type" : "DDR4",
          "dimm_speed" : "2667 MHz"
        }
      ]
    }"#;

    let details = parse_spmemory_json(raw).expect("parse should succeed");
    assert_eq!(details.memory_type.as_deref(), Some("DDR4"));
    assert_eq!(details.clock_mhz, Some(2667));
    assert_eq!(details.memory_count, Some(2));
    assert_eq!(details.total_slots, Some(3));
  }

  #[test]
  fn get_raw_spmemory_json_runs_on_macos() {
    let raw = get_raw_spmemory_json().expect("system_profiler should run");
    assert!(raw.contains("SPMemoryDataType"));
  }
}
