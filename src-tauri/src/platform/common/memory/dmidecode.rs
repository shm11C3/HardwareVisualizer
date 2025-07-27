use crate::platform::common::memory;
use crate::structs;
use crate::utils;
use crate::{log_internal, log_warn};

use std::process::Command;

pub async fn get_memory_info_dmidecode() -> Result<structs::hardware::MemoryInfo, String>
{
  let raw = get_raw_dmidecode().await?;
  let parsed = parse_dmidecode_memory_info(&raw);

  if let Err(e) = memory::save_memory_info_cache(&parsed) {
    log_warn!(
      "Failed to cache memory info",
      "get_memory_info_dmidecode",
      Some(e.to_string())
    );
  }

  Ok(parsed)
}

async fn get_raw_dmidecode() -> Result<String, String> {
  let output = Command::new("pkexec")
    .arg("dmidecode")
    .arg("--type")
    .arg("memory")
    .output()
    .map_err(|e| format!("Failed to execute pkexec dmidecode: {e}"))?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).to_string())
  }
}

fn parse_dmidecode_memory_info(raw: &str) -> structs::hardware::MemoryInfo {
  use regex::Regex;

  let mut total_bytes: u64 = 0;
  let mut count = 0;
  let mut total_slots = 0;
  let mut memory_type = String::from("Unknown");
  let mut clock_mts = 0;

  let re_device = Regex::new(r"^\s*Memory Device").unwrap();
  let re_size = Regex::new(r"^\s*Size:\s+(\d+)\s+(GB|MB)").unwrap();
  let re_type = Regex::new(r"^\s*Type:\s+([A-Za-z0-9]+)").unwrap();
  let re_clock = Regex::new(r"^\s*Configured Memory Speed:\s+(\d+)\s+MT/s").unwrap();

  for line in raw.lines() {
    if re_device.is_match(line) {
      total_slots += 1;
    }

    if let Some(cap) = re_size.captures(line) {
      let value: u64 = cap[1].parse().unwrap_or(0);
      let unit = &cap[2];

      let bytes = match unit {
        "GB" => value * 1024 * 1024 * 1024,
        "MB" => value * 1024 * 1024,
        _ => 0,
      };

      if bytes > 0 {
        total_bytes += bytes;
        count += 1;
      }
    }

    if memory_type == "Unknown" {
      if let Some(cap) = re_type.captures(line) {
        let typ = &cap[1];
        if typ != "Unknown" && typ != "RAM" {
          memory_type = typ.to_string();
        }
      }
    }

    if clock_mts == 0 {
      if let Some(cap) = re_clock.captures(line) {
        clock_mts = cap[1].parse().unwrap_or(0);
      }
    }
  }

  let size_with_unit: utils::formatter::SizeWithUnit =
    utils::formatter::format_size_with_unit(total_bytes, 1, None);

  structs::hardware::MemoryInfo {
    is_detailed: true,
    size: format!(
      "{value:.1} {unit}",
      value = size_with_unit.value,
      unit = size_with_unit.unit
    ),
    clock: clock_mts / 2, // DDR系メモリ：MT/s => MHz
    clock_unit: "MHz".into(),
    memory_count: count,
    total_slots,
    memory_type,
  }
}
