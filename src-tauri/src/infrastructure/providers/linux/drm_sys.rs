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
