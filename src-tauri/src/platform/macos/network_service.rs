use crate::platform::traits::NetworkService;
use crate::structs::hardware::NetworkInfo;
use async_trait::async_trait;

pub struct MacOSNetworkService;

#[async_trait]
impl NetworkService for MacOSNetworkService {
  async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
    Err("macOS network info not implemented yet".to_string())
  }
}
