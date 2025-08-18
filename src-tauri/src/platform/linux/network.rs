use crate::{enums::error::BackendError, infrastructure, models::hardware::NetworkInfo};

pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  infrastructure::providers::net_sys::get_network_info()
    .map_err(|_| BackendError::UnexpectedError)
}
