use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpDisplayGpu {
  pub name: Option<String>,
  pub vendor_name: Option<String>,
  pub vram: Option<String>,
  pub vram_shared: Option<String>,
}

/// Reads the raw JSON output of `system_profiler SPDisplaysDataType -json`.
pub fn get_raw_spdisplays_json() -> Result<String, String> {
  let output = std::process::Command::new("system_profiler")
    .arg("SPDisplaysDataType")
    .arg("-json")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).to_string())
  }
}

/// Parses `system_profiler` JSON output and extracts best-effort GPU/display adapter details.
pub fn parse_spdisplays_json(raw: &str) -> Result<Vec<SpDisplayGpu>, String> {
  let value: Value = serde_json::from_str(raw)
    .map_err(|e| format!("Failed to parse system_profiler JSON: {e}"))?;

  let items = value
    .get("SPDisplaysDataType")
    .and_then(|v| v.as_array())
    .ok_or_else(|| "system_profiler JSON missing SPDisplaysDataType array".to_string())?;

  let mut out: Vec<SpDisplayGpu> = Vec::new();

  for item in items {
    let Some(obj) = item.as_object() else {
      continue;
    };

    if let Some(ndrvs) = obj.get("spdisplays_ndrvs").and_then(|v| v.as_array()) {
      for drv in ndrvs {
        let Some(drv_obj) = drv.as_object() else {
          continue;
        };

        // Prefer explicit model keys. `_name` can be a display name on some machines.
        let name = drv_obj
          .get("sppci_model")
          .or_else(|| drv_obj.get("spdisplays_model"))
          .or_else(|| drv_obj.get("spdisplays_chipset_model"))
          .or_else(|| drv_obj.get("spdisplays_chipset"))
          .or_else(|| drv_obj.get("spdisplays_renderer"))
          .or_else(|| drv_obj.get("_name"))
          .and_then(|v| v.as_str())
          .map(|s| s.trim().to_string())
          .filter(|s| !s.is_empty());

        let vendor_name = drv_obj
          .get("spdisplays_vendor")
          .and_then(|v| v.as_str())
          .map(|s| s.trim().to_string())
          .filter(|s| !s.is_empty());

        let vram = drv_obj
          .get("spdisplays_vram")
          .and_then(|v| v.as_str())
          .map(|s| s.trim().to_string())
          .filter(|s| !s.is_empty());

        let vram_shared = drv_obj
          .get("spdisplays_vram_shared")
          .and_then(|v| v.as_str())
          .map(|s| s.trim().to_string())
          .filter(|s| !s.is_empty());

        // Filter out display-only entries (e.g. "Color LCD") when no GPU-relevant fields exist.
        let is_displayish = name
          .as_deref()
          .map(|s| {
            s.eq_ignore_ascii_case("Color LCD")
              || s.to_ascii_lowercase().contains("retina")
          })
          .unwrap_or(false);

        // Some Apple Silicon machines may not provide vendor/VRAM strings here.
        // Treat chipset/metal fields as GPU-relevant signals.
        let has_any_gpu_field = vendor_name.is_some()
          || vram.is_some()
          || vram_shared.is_some()
          || drv_obj
            .get("spdisplays_chipset_model")
            .and_then(|v| v.as_str())
            .is_some()
          || drv_obj.get("spdisplays_metal").is_some();
        if is_displayish && !has_any_gpu_field {
          continue;
        }

        out.push(SpDisplayGpu {
          name,
          vendor_name,
          vram,
          vram_shared,
        });
      }
    }
  }

  Ok(out)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  #[test]
  fn parse_spdisplays_json_extracts_basic_fields() {
    let raw = r#"{
      "SPDisplaysDataType": [
        {
          "spdisplays_ndrvs": [
            {
              "_name": "Apple M1",
              "spdisplays_vendor": "Apple",
              "spdisplays_vram": "N/A",
              "spdisplays_vram_shared": "System Memory"
            }
          ]
        }
      ]
    }"#;

    let gpus = parse_spdisplays_json(raw).expect("parse should succeed");
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].name.as_deref(), Some("Apple M1"));
    assert_eq!(gpus[0].vendor_name.as_deref(), Some("Apple"));
  }
}
