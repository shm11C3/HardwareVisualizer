use crate::infrastructure;
use crate::structs;
use crate::structs::hardware::MemoryInfo;
use crate::utils;
use crate::{log_internal, log_warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::future::Future;
use std::io;
use std::pin::Pin;

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
    let mem_kb = infrastructure::procfs::get_mem_total_kb()
      .map_err(|e| format!("Failed to read /proc/meminfo: {e}"))?;

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
    let raw = infrastructure::dmidecode::get_raw_dmidecode().await?;
    let parsed = infrastructure::dmidecode::parse_dmidecode_memory_info(&raw);

    if let Err(e) = save_memory_info_cache(&parsed) {
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

/// メモリ情報キャッシュを保存読み込む
fn save_memory_info_cache(info: &structs::hardware::MemoryInfo) -> io::Result<()> {
  let path = get_cache_path();

  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }

  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(io::Error::other)?
    .as_millis() as u64;

  let wrapper = MemoryInfoWithMeta {
    timestamp: now,
    data: info.clone(),
  };

  let json = serde_json::to_string_pretty(&wrapper)?;
  fs::write(path, json)
}

fn get_cache_path() -> std::path::PathBuf {
  dirs::cache_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
    .join("hardware_visualizer")
    .join("memory_info.json")
}
