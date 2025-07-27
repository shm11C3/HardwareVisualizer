pub mod gpu_service;
pub mod memory_service;
pub mod network_service;

use super::traits::{GpuService, MemoryService, NetworkService, PlatformServices};

pub struct LinuxPlatform;

impl LinuxPlatform {
  #[allow(dead_code)]
  pub fn new() -> Self {
    Self
  }
}

impl PlatformServices for LinuxPlatform {
  fn memory_service(&self) -> Box<dyn MemoryService> {
    Box::new(memory_service::LinuxMemoryService)
  }

  fn gpu_service(&self) -> Box<dyn GpuService> {
    Box::new(gpu_service::LinuxGpuService)
  }

  fn network_service(&self) -> Box<dyn NetworkService> {
    Box::new(network_service::LinuxNetworkService)
  }
}
