use crate::enums::error::BackendError;
use crate::models::hardware::{GpuMemoryUsage, GraphicInfo, NetworkInfo};
use crate::platform::traits::{
  GpuPlatform, MemoryPlatform, MotherboardPlatform, NetworkPlatform, Platform,
  ProcessElevationPlatform,
};
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

impl GpuPlatform for LinuxPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<super::traits::GpuUsageRaw, String>> + Send + '_>>
  {
    Box::pin(gpu::get_gpu_usage())
  }

  fn get_gpu_temperature(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::models::hardware::NameValue>, String>>
        + Send
        + '_,
    >,
  > {
    Box::pin(gpu::get_gpu_temperature())
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

impl NetworkPlatform for LinuxPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError> {
    network::get_network_info()
  }
}

impl MotherboardPlatform for LinuxPlatform {
  fn get_motherboard_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::models::hardware::MotherboardInfo, String>>
        + Send
        + '_,
    >,
  > {
    Box::pin(async {
      Err("get_motherboard_info is not implemented for LinuxPlatform".to_string())
    })
  }
}

impl ProcessElevationPlatform for LinuxPlatform {
  fn is_process_elevated(&self) -> Result<bool, String> {
    Ok(false)
  }

  fn relaunch_current_process_elevated(&self) -> Result<(), String> {
    Err("Elevated Startup Mode is only supported on Windows.".to_string())
  }
}

impl Platform for LinuxPlatform {}
