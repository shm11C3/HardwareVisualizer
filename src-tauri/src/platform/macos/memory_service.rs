use crate::platform::traits::MemoryService;
use crate::structs::hardware::MemoryInfo;
use async_trait::async_trait;

pub struct MacOSMemoryService;

#[async_trait]
impl MemoryService for MacOSMemoryService {
  async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
    Err("macOS memory info not implemented yet".to_string())
  }

  async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
    Err("macOS detailed memory info not implemented yet".to_string())
  }
}
