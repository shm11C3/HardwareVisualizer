use crate::infrastructure::providers::wmi_provider;
use crate::models::hardware::MemoryInfo;
use std::future::Future;
use std::pin::Pin;

pub fn get_memory_info()
-> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + 'static>> {
  Box::pin(async {
    // Use actual WMI implementation
    match wmi_provider::query_memory_info().await {
      Ok(info) => Ok(info),
      Err(e) => Err(e),
    }
  })
}

pub fn get_memory_info_detail()
-> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + 'static>> {
  Box::pin(async { Err("Detailed memory info is not implemented yet".to_string()) })
}
