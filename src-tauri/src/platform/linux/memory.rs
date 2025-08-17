use crate::infrastructure;
use crate::platform::linux;
use crate::structs;
use crate::structs::hardware::MemoryInfo;
use crate::utils;
use crate::{log_internal, log_warn};
use serde::{Deserialize, Serialize};
use std;

#[derive(Serialize, Deserialize)]
struct MemoryInfoWithMeta {
  pub timestamp: u64, // UNIX time millis
  pub data: structs::hardware::MemoryInfo,
}

pub fn get_memory_info() -> std::pin::Pin<
  Box<dyn std::future::Future<Output = Result<MemoryInfo, String>> + Send + 'static>,
> {
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

pub fn get_memory_info_detail() -> std::pin::Pin<
  Box<dyn std::future::Future<Output = Result<MemoryInfo, String>> + Send + 'static>,
> {
  Box::pin(async {
    let raw = infrastructure::dmidecode::get_raw_dmidecode().await?;
    let parsed = infrastructure::dmidecode::parse_dmidecode_memory_info(&raw);

    if let Err(e) =
      linux::cache::write_cache(&parsed, &linux::cache::get_memory_cache_path())
    {
      log_warn!(
        "Failed to cache memory info",
        "get_memory_info_detail",
        Some(e.to_string())
      );
    }

    Ok(parsed)
  })
}

fn get_memory_info_cached_detail() -> std::io::Result<MemoryInfo> {
  let cache_path = linux::cache::get_memory_cache_path();
  linux::cache::read_cache(&cache_path)
}
