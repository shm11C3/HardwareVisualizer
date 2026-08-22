use crate::enums::error::PlatformError;
use crate::infrastructure::providers;
use crate::log_warn;
use crate::models;
use crate::models::hardware::MemoryInfo;
use crate::platform::linux;
use crate::utils;
use std;

pub async fn get_memory_info() -> Result<MemoryInfo, PlatformError> {
  if let Ok(cached) = get_memory_info_cached_detail() {
    return Ok(cached);
  }

  // fallback: Only get memory capacity
  let mem_kb = providers::procfs::get_mem_total_kb()
    .map_err(|e| PlatformError::fault(format!("Failed to read /proc/meminfo: {e}")))?;

  Ok(models::hardware::MemoryInfo {
    size: utils::formatter::format_size(mem_kb * 1024, 1),
    clock: 0,
    clock_unit: "MHz".into(),
    memory_count: 0,
    total_slots: 0,
    memory_type: "Unknown".into(),
    is_detailed: false,
  })
}

pub async fn get_memory_info_detail() -> Result<MemoryInfo, PlatformError> {
  let raw = providers::dmidecode::get_raw_dmidecode()
    .await
    .map_err(PlatformError::fault)?;
  let parsed = providers::dmidecode::parse_dmidecode_memory_info(&raw);

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
}

fn get_memory_info_cached_detail() -> std::io::Result<MemoryInfo> {
  let cache_path = linux::cache::get_memory_cache_path();
  linux::cache::read_cache(&cache_path)
}
