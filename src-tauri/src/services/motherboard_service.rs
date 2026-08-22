use crate::models::hardware::MotherboardInfo;
use hardviz_core::enums::error::PlatformError;
use hardviz_core::platform::factory::PlatformFactory;

///
/// Fetch motherboard and BIOS information
///
pub async fn fetch_motherboard_info() -> Result<MotherboardInfo, PlatformError> {
  let platform = PlatformFactory::create()?;

  Ok(platform.get_motherboard_info().await?.into())
}
