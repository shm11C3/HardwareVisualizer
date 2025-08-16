use crate::{enums::error::BackendError, infrastructure, structs::hardware::NetworkInfo};

pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  infrastructure::wmi_provider::query_network_info()
    .map_err(|_| BackendError::UnexpectedError)
}
