use crate::enums::error::BackendError;
use crate::enums::settings::TemperatureUnit;
use crate::models::hardware::{GraphicInfo, MemoryInfo, NetworkInfo};
use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform, Platform};
use std::future::Future;
use std::pin::Pin;
use tauri::async_runtime;
pub mod memory;
pub mod network;

pub struct MacOSPlatform;

impl MacOSPlatform {
  pub fn new() -> Result<Self, String> {
    Ok(Self)
  }
}

impl MemoryPlatform for MacOSPlatform {
  fn get_memory_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>> {
    Box::pin(async {
      async_runtime::spawn_blocking(memory::get_memory_info)
        .await
        .map_err(|e| format!("Failed to join memory task: {e}"))?
    })
  }

  fn get_memory_info_detail(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>> {
    Box::pin(async {
      // macOS is not supported yet (build-only stub)
      Err("get_memory_info_detail is not implemented for MacOSPlatform".to_string())
    })
  }
}

impl GpuPlatform for MacOSPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
    Box::pin(async {
      // macOS is not supported yet (build-only stub)
      Err("get_gpu_usage is not implemented for MacOSPlatform".to_string())
    })
  }

  fn get_gpu_temperature(
    &self,
    _temperature_unit: TemperatureUnit,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::models::hardware::NameValue>, String>>
        + Send
        + '_,
    >,
  > {
    Box::pin(async {
      // macOS is not supported yet (build-only stub)
      Err("get_gpu_temperature is not implemented for MacOSPlatform".to_string())
    })
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(async {
      // macOS is not supported yet (build-only stub)
      Err("get_gpu_info is not implemented for MacOSPlatform".to_string())
    })
  }
}

impl NetworkPlatform for MacOSPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError> {
    network::get_network_info()
  }
}

impl Platform for MacOSPlatform {}
