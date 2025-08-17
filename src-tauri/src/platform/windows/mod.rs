use crate::enums::error::BackendError;
use crate::enums::settings::TemperatureUnit;
use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform, Platform};
use crate::structs::hardware::{GraphicInfo, NetworkInfo};

use std::future::Future;
use std::pin::Pin;

pub mod gpu;
pub mod memory;
pub mod network;

pub struct WindowsPlatform;

impl WindowsPlatform {
  pub fn new() -> Result<Self, String> {
    Ok(Self)
  }
}

impl MemoryPlatform for WindowsPlatform {
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

impl GpuPlatform for WindowsPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
    Box::pin(gpu::get_gpu_usage())
  }

  fn get_gpu_temperature(
    &self,
    temperature_unit: TemperatureUnit,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::structs::hardware::NameValue>, String>>
        + Send
        + '_,
    >,
  > {
    Box::pin(gpu::get_gpu_temperature(temperature_unit))
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(gpu::get_gpu_info())
  }
}

impl NetworkPlatform for WindowsPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError> {
    network::get_network_info()
  }
}

impl Platform for WindowsPlatform {}
