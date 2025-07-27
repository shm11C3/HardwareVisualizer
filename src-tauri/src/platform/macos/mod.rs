pub mod gpu_service;
pub mod memory_service;
pub mod network_service;

use super::traits::{GpuService, MemoryService, NetworkService, PlatformServices};

pub struct MacOSPlatform;

impl MacOSPlatform {
  #[allow(dead_code)]
  pub fn new() -> Self {
    Self
  }
}

impl PlatformServices for MacOSPlatform {
  fn memory_service(&self) -> Box<dyn MemoryService> {
    Box::new(memory_service::MacOSMemoryService)
  }

  fn gpu_service(&self) -> Box<dyn GpuService> {
    Box::new(gpu_service::MacOSGpuService)
  }

  fn network_service(&self) -> Box<dyn NetworkService> {
    Box::new(network_service::MacOSNetworkService)
  }
}
