use crate::enums;
use crate::enums::error::BackendError;
use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform, Platform};
use crate::structs::hardware::{GraphicInfo, NetworkInfo};
use std::future::Future;
use std::pin::Pin;

pub mod cache;
pub mod gpu;
pub mod memory;
pub mod network;

pub struct LinuxPlatform;

impl LinuxPlatform {
  pub fn new() -> Result<Self, String> {
    Ok(Self)
  }
}

impl MemoryPlatform for LinuxPlatform {
  fn get_memory_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::structs::hardware::MemoryInfo, String>>
        + Send
        + '_,
    >,
  > {
    memory::get_memory_info()
  }

  fn get_memory_info_detail(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::structs::hardware::MemoryInfo, String>>
        + Send
        + '_,
    >,
  > {
    memory::get_memory_info_detail()
  }
}

impl GpuPlatform for LinuxPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
    Box::pin(gpu::get_gpu_usage())
  }

  fn get_gpu_temperature(
    &self,
    _temperature_unit: enums::settings::TemperatureUnit,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::structs::hardware::NameValue>, String>>
        + Send
        + '_,
    >,
  > {
    Box::pin(async { Err("Not implemented".to_string()) })
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(gpu::get_gpu_info())
  }
}

impl NetworkPlatform for LinuxPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError> {
    network::get_network_info()
  }
}

impl Platform for LinuxPlatform {}
