use crate::enums::error::BackendError;
use crate::enums::settings::TemperatureUnit;
use crate::models::hardware::{
  GpuMemoryUsage, GraphicInfo, MotherboardInfo, NetworkInfo,
};
use crate::platform::traits::{
  GpuPlatform, MemoryPlatform, MotherboardPlatform, NetworkPlatform, Platform,
};

use std::future::Future;
use std::pin::Pin;

pub mod gpu;
pub mod memory;
pub mod motherboard;
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
      dyn Future<Output = Result<crate::models::hardware::MemoryInfo, String>>
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
      dyn Future<Output = Result<crate::models::hardware::MemoryInfo, String>>
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
  ) -> Pin<Box<dyn Future<Output = Result<super::traits::GpuUsageRaw, String>> + Send + '_>>
  {
    Box::pin(gpu::get_gpu_usage())
  }

  fn get_gpu_temperature(
    &self,
    temperature_unit: TemperatureUnit,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::models::hardware::NameValue>, String>>
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

  fn get_gpu_memory_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Option<GpuMemoryUsage>, String>> + Send + '_>>
  {
    Box::pin(async { Ok(None) })
  }
}

impl NetworkPlatform for WindowsPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError> {
    network::get_network_info()
  }
}

impl MotherboardPlatform for WindowsPlatform {
  fn get_motherboard_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<MotherboardInfo, String>> + Send + '_>> {
    motherboard::get_motherboard_info()
  }
}

impl Platform for WindowsPlatform {}
