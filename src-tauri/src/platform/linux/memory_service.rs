#[cfg(target_os = "linux")]
use crate::platform::common::memory;
use crate::platform::traits::MemoryService;
use crate::structs::hardware::MemoryInfo;
use async_trait::async_trait;

pub struct LinuxMemoryService;

#[async_trait]
impl MemoryService for LinuxMemoryService {
  async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
    #[cfg(target_os = "linux")]
    {
      memory::get_memory_info_linux().await
    }
    #[cfg(not(target_os = "linux"))]
    {
      Err("Linux memory service not available on this OS".to_string())
    }
  }

  async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
    #[cfg(target_os = "linux")]
    {
      memory::get_memory_info_dmidecode().await
    }
    #[cfg(not(target_os = "linux"))]
    {
      Err("Linux detailed memory info not available on this OS".to_string())
    }
  }
}
