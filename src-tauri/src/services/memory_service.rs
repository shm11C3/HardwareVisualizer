use crate::models::hardware::MemoryInfo;
use hardviz_core::platform::factory::PlatformFactory;

///
/// ## Get detailed memory information via Platform
/// Returns `MemoryInfo` on success, error message on failure
///
pub async fn fetch_memory_detail() -> Result<MemoryInfo, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  Ok(platform.get_memory_info_detail().await?.into())
}
