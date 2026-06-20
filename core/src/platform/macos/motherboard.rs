use crate::infrastructure::providers::macos::system_profiler_hardware::{
  self, SpHardwareDetails,
};
use crate::models::hardware::MotherboardInfo;
use std::future::Future;
use std::pin::Pin;

/// macOS does not expose a baseboard / BIOS pair, so both the board and
/// firmware vendor are reported as Apple.
const APPLE_MANUFACTURER: &str = "Apple Inc.";

/// Returns motherboard / firmware information on macOS.
///
/// macOS has no Windows-style `Win32_BaseBoard` + `Win32_BIOS` data. We map the
/// `system_profiler SPHardwareDataType` hardware overview onto the shared
/// [`MotherboardInfo`] shape on a best-effort basis:
///
/// - `manufacturer` / `bios_vendor` are always `"Apple Inc."`.
/// - `product` is the marketing model name (e.g. "MacBook Pro"), falling back
///   to the model identifier when the name is unavailable.
/// - `version` is the model identifier (e.g. "Mac16,1").
/// - `serial_number` is the system serial number.
/// - `bios_version` is the system firmware (boot ROM) version.
/// - `bios_release_date` is left empty (macOS does not expose it).
pub fn get_motherboard_info()
-> Pin<Box<dyn Future<Output = Result<MotherboardInfo, String>> + Send>> {
  Box::pin(async {
    tokio::task::spawn_blocking(collect_motherboard_info)
      .await
      .map_err(|e| format!("Join error: {e}"))?
  })
}

fn collect_motherboard_info() -> Result<MotherboardInfo, String> {
  let raw = system_profiler_hardware::get_raw_sphardware_json()?;
  let details = system_profiler_hardware::parse_sphardware_json(&raw)?;
  Ok(map_motherboard_info(details))
}

/// Maps best-effort `system_profiler` hardware details onto [`MotherboardInfo`].
///
/// `product` prefers the human-readable model name and `version` carries the
/// model identifier. When only the model identifier is available it is used as
/// the product, and `version` is left empty to avoid showing the same value
/// twice (the UI hides empty versions).
fn map_motherboard_info(details: SpHardwareDetails) -> MotherboardInfo {
  let (product, version) = match (details.machine_name, details.machine_model) {
    (Some(name), Some(model)) => (name, model),
    (Some(name), None) => (name, String::new()),
    (None, Some(model)) => (model, String::new()),
    (None, None) => (String::new(), String::new()),
  };

  MotherboardInfo {
    manufacturer: APPLE_MANUFACTURER.to_string(),
    product,
    version,
    serial_number: details.serial_number.unwrap_or_default(),
    bios_vendor: APPLE_MANUFACTURER.to_string(),
    bios_version: details.boot_rom_version.unwrap_or_default(),
    bios_release_date: String::new(),
  }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  #[test]
  fn map_motherboard_info_full_details() {
    let details = SpHardwareDetails {
      machine_name: Some("MacBook Pro".to_string()),
      machine_model: Some("Mac16,1".to_string()),
      model_number: Some("Z1DS000MAJ/A".to_string()),
      serial_number: Some("K2QJ212W6H".to_string()),
      boot_rom_version: Some("18000.120.36".to_string()),
    };

    let info = map_motherboard_info(details);
    assert_eq!(info.manufacturer, "Apple Inc.");
    assert_eq!(info.product, "MacBook Pro");
    assert_eq!(info.version, "Mac16,1");
    assert_eq!(info.serial_number, "K2QJ212W6H");
    assert_eq!(info.bios_vendor, "Apple Inc.");
    assert_eq!(info.bios_version, "18000.120.36");
    assert_eq!(info.bios_release_date, "");
  }

  #[test]
  fn map_motherboard_info_model_only_avoids_duplicate_version() {
    let details = SpHardwareDetails {
      machine_name: None,
      machine_model: Some("Mac16,1".to_string()),
      ..SpHardwareDetails::default()
    };

    let info = map_motherboard_info(details);
    assert_eq!(info.product, "Mac16,1");
    assert_eq!(info.version, "");
  }

  #[test]
  fn map_motherboard_info_missing_details_default_to_empty() {
    let info = map_motherboard_info(SpHardwareDetails::default());
    assert_eq!(info.manufacturer, "Apple Inc.");
    assert_eq!(info.product, "");
    assert_eq!(info.version, "");
    assert_eq!(info.serial_number, "");
    assert_eq!(info.bios_version, "");
  }

  #[tokio::test]
  async fn get_motherboard_info_returns_apple_manufacturer() {
    let info = get_motherboard_info()
      .await
      .expect("get_motherboard_info should succeed on macOS");
    assert_eq!(info.manufacturer, "Apple Inc.");
    assert_eq!(info.bios_vendor, "Apple Inc.");
    assert!(!info.serial_number.is_empty());
  }
}
