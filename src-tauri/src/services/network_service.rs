use crate::enums;
use crate::models::hardware::NetworkInfo;
use hardviz_core::platform::factory::PlatformFactory;

///
/// Get network interface information.
///
/// Returns `BackendError::UnexpectedError` if the Platform layer can't be
/// instantiated. Provider-side failures are propagated through the
/// `From<core::enums::error::BackendError>` conversion so frontend callers
/// keep seeing the specific variants
/// (`NetworkInfoNotAvailable` / `NetworkUsageNotAvailable`).
///
pub fn fetch_network_info() -> Result<Vec<NetworkInfo>, enums::error::BackendError> {
  let platform =
    PlatformFactory::create().map_err(|_| enums::error::BackendError::UnexpectedError)?;
  let core_list = platform
    .get_network_info()
    .map_err(enums::error::BackendError::from)?;
  Ok(core_list.into_iter().map(NetworkInfo::from).collect())
}
