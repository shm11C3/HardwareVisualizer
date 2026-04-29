use crate::enums;
use crate::models::hardware::NetworkInfo;
use hwviz_core::platform::factory::PlatformFactory;

///
/// Get network interface information
/// Returns `BackendError::UnexpectedError` if Platform is unsupported / fails
///
pub fn fetch_network_info() -> Result<Vec<NetworkInfo>, enums::error::BackendError> {
  let platform =
    PlatformFactory::create().map_err(|_| enums::error::BackendError::UnexpectedError)?;
  let core_list = platform
    .get_network_info()
    .map_err(|_| enums::error::BackendError::UnexpectedError)?;
  Ok(core_list.into_iter().map(NetworkInfo::from).collect())
}
