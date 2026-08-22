use crate::enums::error::PlatformError;
use crate::infrastructure::providers::wmi_provider;
use crate::models::hardware::MemoryInfo;

pub async fn get_memory_info() -> Result<MemoryInfo, PlatformError> {
  wmi_provider::query_memory_info()
    .await
    .map_err(PlatformError::fault)
}

pub async fn get_memory_info_detail() -> Result<MemoryInfo, PlatformError> {
  Err(PlatformError::unsupported(
    "Detailed memory info is not implemented yet",
  ))
}
