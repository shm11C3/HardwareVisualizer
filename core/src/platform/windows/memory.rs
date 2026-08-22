use crate::enums::error::PlatformError;
use crate::infrastructure::providers::wmi_provider;
use crate::models::hardware::MemoryInfo;
use std::future::Future;
use std::pin::Pin;

pub fn get_memory_info()
-> Pin<Box<dyn Future<Output = Result<MemoryInfo, PlatformError>> + Send + 'static>> {
  Box::pin(async {
    wmi_provider::query_memory_info()
      .await
      .map_err(PlatformError::fault)
  })
}

pub fn get_memory_info_detail()
-> Pin<Box<dyn Future<Output = Result<MemoryInfo, PlatformError>> + Send + 'static>> {
  Box::pin(async {
    Err(PlatformError::unsupported(
      "Detailed memory info is not implemented yet",
    ))
  })
}
