use crate::platform::traits::MemoryService;
use crate::platform::windows::wmi_service;
use crate::structs::hardware::MemoryInfo;
use async_trait::async_trait;

pub struct WindowsMemoryService;

#[async_trait]
impl MemoryService for WindowsMemoryService {
  async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
    wmi_service::get_memory_info().await
  }

  async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
    match wmi_service::get_memory_info().await {
      Ok(info) => Ok(vec![info]),
      Err(e) => Err(e),
    }
  }
}
