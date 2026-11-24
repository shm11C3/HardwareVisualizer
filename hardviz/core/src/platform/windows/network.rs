use crate::{
  enums::error::PlatformError,
  infrastructure,
  models::hardware::NetworkInfo,
  platform::link_speed::{LinkSpeedMatchKey, attach_link_speeds},
};

pub fn get_network_info() -> Result<Vec<NetworkInfo>, PlatformError> {
  let mut result = infrastructure::providers::wmi_provider::query_network_info()
    .map_err(PlatformError::fault)?;
  // WMI `description` is the adapter description, not sysinfo's interface
  // alias, so the MAC address is the shared identity.
  attach_link_speeds(
    &mut result,
    &infrastructure::providers::sysinfo_provider::read_interface_link_rates(),
    LinkSpeedMatchKey::Mac,
  );
  Ok(result)
}
