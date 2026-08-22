use crate::{enums::error::PlatformError, infrastructure, models::hardware::NetworkInfo};

pub fn get_network_info() -> Result<Vec<NetworkInfo>, PlatformError> {
  infrastructure::providers::net_sys::get_network_info().map_err(PlatformError::fault)
}
