use crate::infrastructure::providers::mac_memory;
use crate::models::hardware::MemoryInfo;
use crate::utils::formatter;

pub fn get_memory_info() -> std::pin::Pin<
  Box<dyn std::future::Future<Output = Result<MemoryInfo, String>> + Send + 'static>,
> {
  Box::pin(async { get_memory_info_impl().await })
}

pub fn get_memory_info_detail() -> std::pin::Pin<
  Box<dyn std::future::Future<Output = Result<MemoryInfo, String>> + Send + 'static>,
> {
  Box::pin(async { get_memory_info_impl().await })
}

async fn get_memory_info_impl() -> Result<MemoryInfo, String> {
  let total_memory = mac_memory::get_hw_memsize()?;
  let memory_type =
    mac_memory::get_memory_type().unwrap_or_else(|_| "Unknown".to_string());
  let (clock, clock_unit) =
    mac_memory::get_memory_speed_with_unit().unwrap_or((0, "MHz".to_string()));
  let memory_count = mac_memory::get_memory_count().unwrap_or(0);

  Ok(MemoryInfo {
    size: formatter::format_size(total_memory, 1),
    clock,
    clock_unit,
    memory_count,
    // On macOS (especially with unified/soldered memory), reliable physical slot counts are not available.
    // Use 0 to indicate "not applicable/unknown" rather than assuming all modules correspond to distinct slots.
    total_slots: 0,
    memory_type,
    is_detailed: true,
  })
}
