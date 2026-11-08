use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
  Nvidia,
  Amd,
  Intel,
  Unknown,
}
#[derive(Debug, Clone)]
pub struct CardInfo {
  pub id: u32,
  pub vendor_id: String,
}

///
/// Enumerate all `card*` in `/sys/class/drm/` and return card IDs
///
pub async fn get_card_ids() -> Result<Vec<CardInfo>, String> {
  use tokio::task::JoinSet;

  let mut join_set = JoinSet::new();

  for card_id in 0..=9 {
    let vendor_path = format!("/sys/class/drm/card{card_id}/device/vendor");
    if std::path::Path::new(&vendor_path).exists() {
      join_set.spawn(async move {
        match tokio::fs::read_to_string(&vendor_path).await {
          Ok(content) => Ok(CardInfo {
            id: card_id,
            vendor_id: content.trim().to_string(),
          }),
          Err(e) => Err(format!("Failed to read vendor for card{card_id}: {e}")),
        }
      });
    }
  }

  let mut cards = Vec::new();
  while let Some(res) = join_set.join_next().await {
    match res {
      Ok(Ok(card)) => cards.push(card),
      Ok(Err(e)) => return Err(e),
      Err(e) => return Err(format!("Join error: {e}")),
    }
  }

  cards.sort_by_key(|c| c.id);
  Ok(cards)
}

pub async fn get_amd_gpu_usage(card_id: u32) -> Result<f64, String> {
  let path = format!("/sys/class/drm/card{card_id}/device/gpu_busy_percent");
  let content = fs::read_to_string(&path)
    .map_err(|e| format!("Failed to read AMD gpu_busy_percent: {e}"))?;

  let percent = content
    .trim()
    .parse::<f32>()
    .map_err(|e| format!("Failed to parse gpu_busy_percent: {e}"))?;

  Ok((percent / 100.0).into())
}

pub async fn get_intel_gpu_usage() -> Result<f64, String> {
  let output = Command::new("intel_gpu_top")
    .args(["-J", "-s", "1000"]) // JSON output, get only 1 second
    .output()
    .map_err(|e| format!("Failed to run intel_gpu_top: {e}"))?;

  if !output.status.success() {
    return Err("intel_gpu_top failed".to_string());
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  for line in stdout.lines() {
    if line.contains("\"render busy\"")
      && let Some(value_str) = line.split(':').nth(1)
      && let Ok(value) = value_str.trim().trim_end_matches(',').parse::<f32>()
    {
      return Ok((value / 100.0).into());
    }
  }

  Err("Could not parse intel_gpu_top output".to_string())
}

/// Enumerate all `card*` in `/sys/class/drm/` and return card IDs
pub fn get_all_card_ids() -> Vec<u8> {
  use regex::Regex;
  use std::path::Path;

  let path = Path::new("/sys/class/drm");

  let re = Regex::new(r"^card(\d+)$").unwrap();

  let mut ids = vec![];

  if let Ok(entries) = fs::read_dir(path) {
    for entry in entries.flatten() {
      if let Some(name) = entry.file_name().to_str()
        && let Some(cap) = re.captures(name)
        && let Ok(id) = cap[1].parse::<u8>()
      {
        ids.push(id);
      }
    }
  }

  ids.sort();
  ids
}

pub fn read_vram_total_bytes(card_id: u8) -> Option<u64> {
  let path = format!("/sys/class/drm/card{card_id}/device/mem_info_vram_total");
  fs::read_to_string(path).ok()?.trim().parse::<u64>().ok()
}

pub fn detect_gpu_vendor(card_id: u8) -> GpuVendor {
  let path = format!("/sys/class/drm/card{card_id}/device/vendor");

  match fs::read_to_string(&path) {
    Ok(vendor_id) => match vendor_id.trim() {
      "0x10de" => GpuVendor::Nvidia, // NVIDIA
      "0x1002" => GpuVendor::Amd,    // AMD
      "0x8086" => GpuVendor::Intel,  // Intel
      _ => GpuVendor::Unknown,
    },
    Err(_) => GpuVendor::Unknown,
  }
}

/// Read the PCI BDF (bus:device.function) for a DRM card by following the
/// `/sys/class/drm/card{N}/device` symlink.
///
/// Returns e.g. `"03:00.0"`.
pub fn get_card_bdf(card_id: u8) -> Option<String> {
  let address = get_card_pci_address(card_id)?;
  Some(domainless_bdf(&address).to_string())
}

/// Read a DRM card's full PCI address for joining against a vendor API.
pub(crate) fn get_card_pci_address(card_id: u8) -> Option<String> {
  let link = fs::read_link(format!("/sys/class/drm/card{card_id}/device")).ok()?;
  parse_pci_address_from_device_link(link.to_str()?).map(str::to_ascii_lowercase)
}

/// Query the OS runtime state for a validated full PCI address.
pub(crate) fn pci_runtime_suspended(address: &str) -> bool {
  fs::read_to_string(format!(
    "/sys/bus/pci/devices/{address}/power/runtime_status"
  ))
  .is_ok_and(|status| runtime_suspended(&status))
}

fn runtime_suspended(status: &str) -> bool {
  matches!(status.trim(), "suspended" | "suspending")
}

pub(crate) fn domainless_bdf(address: &str) -> &str {
  // Callers hold an address already validated by get_card_pci_address.
  address.split_once(':').map_or(address, |(_, bdf)| bdf)
}

fn parse_pci_address_from_device_link(symlink_target: &str) -> Option<&str> {
  let segment = symlink_target.rsplit('/').next()?;
  let (domain, bdf) = segment.split_once(':')?;
  let (bus, devfn) = bdf.split_once(':')?;
  let (device, function) = devfn.split_once('.')?;
  if !(4..=8).contains(&domain.len())
    || bus.len() != 2
    || device.len() != 2
    || function.len() != 1
    || ![domain, bus, device, function]
      .iter()
      .all(|field| field.bytes().all(|byte| byte.is_ascii_hexdigit()))
    || u8::from_str_radix(device, 16).ok()? > 31
    || u8::from_str_radix(function, 16).ok()? > 7
  {
    return None;
  }

  Some(segment)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn runtime_status_only_skips_confirmed_suspend_states() {
    for status in ["suspended\n", "suspending"] {
      assert!(runtime_suspended(status));
    }
    for status in ["active", "resuming", "unsupported", "error", ""] {
      assert!(!runtime_suspended(status));
    }
  }

  fn parse_bdf_from_device_link(symlink_target: &str) -> Option<String> {
    parse_pci_address_from_device_link(symlink_target)
      .map(domainless_bdf)
      .map(str::to_string)
  }

  #[test]
  fn parse_bdf_standard_pci_address() {
    assert_eq!(
      parse_bdf_from_device_link("../../0000:03:00.0"),
      Some("03:00.0".to_string())
    );
  }

  #[test]
  fn parse_bdf_different_domain() {
    assert_eq!(
      parse_bdf_from_device_link("../../0001:0a:00.0"),
      Some("0a:00.0".to_string())
    );
  }

  #[test]
  fn parse_bdf_non_zero_function() {
    assert_eq!(
      parse_bdf_from_device_link("../../0000:06:00.1"),
      Some("06:00.1".to_string())
    );
  }

  #[test]
  fn parse_bdf_absolute_path() {
    assert_eq!(
      parse_bdf_from_device_link("/sys/devices/pci0000:00/0000:00:08.1/0000:0a:00.0"),
      Some("0a:00.0".to_string())
    );
  }

  #[test]
  fn parse_bdf_empty_string() {
    assert!(parse_bdf_from_device_link("").is_none());
  }

  #[test]
  fn parse_bdf_invalid_format() {
    assert!(parse_bdf_from_device_link("../../invalid").is_none());
  }

  #[test]
  fn vendor_lookup_keeps_domain_and_function_and_checks_slot_bounds() {
    assert_eq!(
      parse_pci_address_from_device_link("../../ffff:ff:1f.7"),
      Some("ffff:ff:1f.7")
    );
    assert_eq!(
      parse_pci_address_from_device_link("../../0001:03:00.1"),
      Some("0001:03:00.1")
    );
    for link in [
      "../../0000:01:20.0",
      "../../0000:01:00.8",
      "../../0000:gg:00.0",
      "../../01:00.0",
      "../../+000:01:00.0",
      "../../000000000:01:00.0",
      "../../0000:01:00.00",
      "../../0000:01:00.0:other",
    ] {
      assert_eq!(parse_pci_address_from_device_link(link), None, "{link}");
    }
  }
}
