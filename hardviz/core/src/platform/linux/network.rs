use crate::{
  enums::error::PlatformError,
  infrastructure,
  models::hardware::NetworkInfo,
  platform::link_speed::{LinkSpeedMatchKey, attach_link_speeds},
};

pub fn get_network_info() -> Result<Vec<NetworkInfo>, PlatformError> {
  let mut result = infrastructure::providers::net_sys::get_network_info()
    .map_err(PlatformError::fault)?;
  // `description` is the kernel interface name, the same key sysinfo uses.
  attach_link_speeds(
    &mut result,
    &infrastructure::providers::sysinfo_provider::read_interface_link_rates(),
    LinkSpeedMatchKey::Name,
  );
  Ok(result)
}
