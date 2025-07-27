use crate::platform::common::memory;
use crate::structs;
use crate::utils;

pub async fn get_memory_info_linux() -> Result<structs::hardware::MemoryInfo, String> {
  if let Ok(cached) = memory::get_memory_info_cached_detail() {
    return Ok(cached);
  }

  // fallback: メモリ容量のみ取得
  let mem_kb = memory::get_memtotal_kb()
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
}
