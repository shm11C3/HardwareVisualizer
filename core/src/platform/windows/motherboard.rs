use crate::enums::error::PlatformError;
use crate::infrastructure::providers::windows::wmi_provider;
use crate::models::hardware::MotherboardInfo;

pub async fn get_motherboard_info() -> Result<MotherboardInfo, PlatformError> {
  tokio::task::spawn_blocking(wmi_provider::query_motherboard_info)
    .await
    .map_err(|e| PlatformError::fault(format!("Join error: {e}")))?
    .map_err(PlatformError::fault)
}
