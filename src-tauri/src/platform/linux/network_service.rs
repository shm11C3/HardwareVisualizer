#[cfg(target_os = "linux")]
use crate::platform::linux::ip_linux;
use crate::platform::traits::NetworkService;
use crate::structs::hardware::NetworkInfo;
use async_trait::async_trait;

pub struct LinuxNetworkService;

#[async_trait]
impl NetworkService for LinuxNetworkService {
  async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
    #[cfg(target_os = "linux")]
    {
      ip_linux::get_network_info()
    }
    #[cfg(not(target_os = "linux"))]
    {
      Err("Linux network service not available on this OS".to_string())
    }
  }
}
