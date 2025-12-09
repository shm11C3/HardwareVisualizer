use crate::enums::error::BackendError;
use crate::infrastructure::providers::network;
use crate::models::hardware::NetworkInfo;

pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  let interfaces =
    network::get_network_interfaces().map_err(|_| BackendError::UnexpectedError)?;

  let network_infos = interfaces
    .iter()
    .map(|interface| network::get_interface_info(interface))
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| BackendError::UnexpectedError)?;

  Ok(network_infos)
}
