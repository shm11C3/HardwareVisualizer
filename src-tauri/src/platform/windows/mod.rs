pub mod directx_gpu_service;
pub mod gpu_service;
pub mod memory_service;
pub mod network_service;
pub mod wmi_service;

use super::traits::{GpuService, MemoryService, NetworkService, PlatformServices};

pub struct WindowsPlatform;

impl WindowsPlatform {
  pub fn new() -> Self {
    Self
  }
}

impl PlatformServices for WindowsPlatform {
  fn memory_service(&self) -> Box<dyn MemoryService> {
    Box::new(memory_service::WindowsMemoryService)
  }

  fn gpu_service(&self) -> Box<dyn GpuService> {
    Box::new(gpu_service::WindowsGpuService)
  }

  fn network_service(&self) -> Box<dyn NetworkService> {
    Box::new(network_service::WindowsNetworkService)
  }
}
