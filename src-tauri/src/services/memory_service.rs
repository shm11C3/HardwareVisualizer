use crate::models::hardware::MemoryInfo;
use hardviz_core::enums::error::PlatformError;
use hardviz_core::platform::factory::PlatformFactory;

///
/// ## Get detailed memory information via Platform
/// Returns `MemoryInfo` on success, a [`PlatformError`] on failure
///
pub async fn fetch_memory_detail() -> Result<MemoryInfo, PlatformError> {
  let platform = PlatformFactory::shared()?;
  Ok(platform.get_memory_info_detail().await?.into())
}
