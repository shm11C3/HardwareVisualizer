use crate::models::hardware::MotherboardInfo;
use hardviz_core::platform::factory::PlatformFactory;

///
/// Fetch motherboard and BIOS information
///
pub async fn fetch_motherboard_info() -> Result<MotherboardInfo, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;

  Ok(platform.get_motherboard_info().await?.into())
}
