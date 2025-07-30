use crate::infrastructure::wmi_provider;
use crate::services::memory;
use crate::structs;
use crate::structs;
use crate::structs::hardware::MemoryInfo;
use crate::utils;
use crate::{log_internal, log_warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::future::Future;
use std::io;
use std::pin::Pin;

use std::process::Command;

#[derive(Serialize, Deserialize)]
struct MemoryInfoWithMeta {
  pub timestamp: u64, // UNIX time millis
  pub data: structs::hardware::MemoryInfo,
}

const MAX_AGE_SECS: u64 = 60 * 60 * 24;

pub fn get_memory_info()
-> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + 'static>> {
  Box::pin(async {
    if let Ok(cached) = get_memory_info_cached_detail() {
      return Ok(cached);
    }

    // fallback: メモリ容量のみ取得
    let mem_kb =
      get_mem_total_kb().map_err(|e| format!("Failed to read /proc/meminfo: {e}"))?;

    Ok(structs::hardware::MemoryInfo {
      size: utils::formatter::format_size(mem_kb * 1024, 1),
      clock: 0,
      clock_unit: "MHz".into(),
      memory_count: 0,
      total_slots: 0,
      memory_type: "Unknown".into(),
      is_detailed: false,
    })
  })
}

pub fn get_memory_info_detail()
-> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + 'static>> {
  Box::pin(async {
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
  })
}

/// メモリ情報キャッシュを読み込む
fn get_memory_info_cached_detail() -> io::Result<structs::hardware::MemoryInfo> {
  let path = get_cache_path();

  let content = fs::read_to_string(path)?;
  let wrapper: MemoryInfoWithMeta = serde_json::from_str(&content)?;

  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(io::Error::other)?
    .as_secs();

  let cache_time = wrapper.timestamp / 1000;

  if now - cache_time <= MAX_AGE_SECS {
    Ok(wrapper.data)
  } else {
    Err(io::Error::other("Cache expired"))
  }
}

fn get_mem_total_kb() -> std::io::Result<u64> {
  use std::fs;
  use std::io;

  let content = fs::read_to_string("/proc/meminfo")?;
  for line in content.lines() {
    if let Some(mem_kb_str) = line.strip_prefix("MemTotal:") {
      let kb = mem_kb_str
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No value found"))?;
      return kb
        .parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
    }
  }
  Err(io::Error::new(
    io::ErrorKind::NotFound,
    "MemTotal entry not found",
  ))
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

fn get_cache_path() -> std::path::PathBuf {
  dirs::cache_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
    .join("hardware_visualizer")
    .join("memory_info.json")
}
