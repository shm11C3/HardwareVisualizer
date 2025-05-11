use crate::structs;

use std::fs;
use std::io;

fn get_cache_path() -> std::path::PathBuf {
  dirs::cache_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
    .join("hardware_visualizer")
    .join("memory_info.json")
}

/// メモリ情報キャッシュを読み込む
pub fn get_memory_info_cached_detail() -> io::Result<structs::hardware::MemoryInfo> {
  let path = get_cache_path();
  let content = fs::read_to_string(path)?;
  let info = serde_json::from_str(&content)?;
  Ok(info)
}
