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
  let clock = mac_memory::get_memory_speed().unwrap_or(0);
  let memory_count = mac_memory::get_memory_count().unwrap_or(0);

  Ok(MemoryInfo {
    size: formatter::format_size(total_memory, 1),
    clock,
    clock_unit: "MHz".to_string(),
    memory_count,
    total_slots: memory_count, // On macOS, accurate slot count retrieval is difficult, so we use the module count instead
    memory_type,
    is_detailed: true,
  })
}
