use crate::platform::traits::NetworkService;
use crate::platform::windows::wmi_service;
use crate::structs::hardware::NetworkInfo;
use async_trait::async_trait;

pub struct WindowsNetworkService;

#[async_trait]
impl NetworkService for WindowsNetworkService {
  async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
    wmi_service::get_network_info()
  }
}
