use crate::structs;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

const MAX_AGE_SECS: u64 = 60 * 60 * 24;

#[derive(Serialize, Deserialize)]
pub struct MemoryInfoWithMeta {
  pub timestamp: u64, // UNIX time millis
  pub data: structs::hardware::MemoryInfo,
}

/// メモリ情報キャッシュを読み込む
pub fn get_memory_info_cached_detail() -> io::Result<structs::hardware::MemoryInfo> {
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
pub fn save_memory_info_cache(info: &structs::hardware::MemoryInfo) -> io::Result<()> {
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
