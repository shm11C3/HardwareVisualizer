use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
  Nvidia,
  Amd,
  Intel,
  Unknown,
}

/// `/sys/class/drm/` に存在する `card*` をすべて列挙して card ID を返す
pub fn get_all_card_ids() -> Vec<u8> {
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

pub fn get_gpu_name_from_lspci_by_vendor_id(vendor_id: &str) -> Option<String> {
  use std::process::Command;

  let output = Command::new("lspci").arg("-nn").output().ok()?;

  if !output.status.success() {
    return None;
  }

  let stdout = String::from_utf8_lossy(&output.stdout);

  for line in stdout.lines() {
    if line.contains("VGA") && line.contains(vendor_id) {
      // 例: "03:00.0 VGA compatible controller [0300]: AMD/ATI Renoir [1002:1636]"
      return Some(line.trim().to_string());
    }
  }

  None
}
