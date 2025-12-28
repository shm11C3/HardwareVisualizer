use crate::enums::error::BackendError;
use crate::infrastructure::providers::network;
use crate::models::hardware::NetworkInfo;

pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  let interfaces =
    network::get_network_interfaces().map_err(|_| BackendError::UnexpectedError)?;

  // Get default gateways once for all interfaces (system-wide information)
  let (default_ipv4_gateway, default_ipv6_gateway) =
    network::get_default_gateways().map_err(|_| BackendError::UnexpectedError)?;

  let network_infos = interfaces
    .iter()
    .map(|interface| {
      network::get_interface_info(interface, &default_ipv4_gateway, &default_ipv6_gateway)
    })
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| BackendError::UnexpectedError)?;

  Ok(network_infos)
}
