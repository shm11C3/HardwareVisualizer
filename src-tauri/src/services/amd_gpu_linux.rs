use crate::services;
use crate::structs;
use std::fs;

const VENDOR_ID: &str = "1002";

pub async fn get_amd_gpu_usage(card_id: u8) -> Result<f32, String> {
  let path = format!("/sys/class/drm/card{}/device/gpu_busy_percent", card_id);
  let content = fs::read_to_string(&path)
    .map_err(|e| format!("Failed to read AMD gpu_busy_percent: {e}"))?;

  let percent = content
    .trim()
    .parse::<f32>()
    .map_err(|e| format!("Failed to parse gpu_busy_percent: {e}"))?;

  Ok(percent / 100.0)
}

pub async fn get_amd_graphic_info(
  card_id: u8,
) -> Result<structs::hardware::GraphicInfo, String> {
  let name = services::gpu_linux::get_gpu_name_from_lspci_by_vendor_id(VENDOR_ID)
    .unwrap_or_else(|| "Unknown AMD GPU".to_string());

  let clock = read_pm_info_sclk(card_id).unwrap_or(0);
  let memory_total = read_vram_total_bytes(card_id).unwrap_or(0);

  Ok(structs::hardware::GraphicInfo {
    id: format!("card{card_id}"),
    name,
    vendor_name: "AMD".into(),
    clock,
    memory_size: crate::utils::formatter::format_size(memory_total, 1),
    memory_size_dedicated: crate::utils::formatter::format_size(memory_total, 1),
  })
}

fn read_pm_info_sclk(card_id: u8) -> Option<u32> {
  use regex::Regex;

  let path = format!("/sys/kernel/debug/dri/{}/amdgpu_pm_info", card_id);
  let content = fs::read_to_string(path).ok()?;
  let re = Regex::new(r"SCLK.*?(\d+)\s+MHz").ok()?;
  re.captures(&content)
    .and_then(|cap| cap[1].parse::<u32>().ok())
}

fn read_vram_total_bytes(card_id: u8) -> Option<u64> {
  let path = format!("/sys/class/drm/card{}/device/mem_info_vram_total", card_id);
  fs::read_to_string(path).ok()?.trim().parse::<u64>().ok()
}
