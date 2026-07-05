use crate::enums::error::BackendError;
use crate::models::hardware::{
  GpuMemoryUsage, GraphicInfo, MemoryInfo, NetworkInfo, SuperIoChipIdDiagnostics,
};
use crate::platform::traits::{
  GpuPlatform, MemoryPlatform, MotherboardPlatform, NetworkPlatform, Platform,
  ProcessElevationPlatform, SensorPlatform, SuperIoPlatform,
};
use std::future::Future;
use std::pin::Pin;
use tokio::task;

mod gpu;
pub mod memory;
pub mod motherboard;
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
      task::spawn_blocking(memory::get_memory_info)
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
  ) -> Pin<Box<dyn Future<Output = Result<super::traits::GpuUsageRaw, String>> + Send + '_>>
  {
    Box::pin(async { gpu::get_gpu_usage().await })
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
    Box::pin(async {
      // macOS is not supported yet (build-only stub)
      Err("get_gpu_temperature is not implemented for MacOSPlatform".to_string())
    })
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(async { gpu::get_gpu_info().await })
  }

  fn get_gpu_memory_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Option<GpuMemoryUsage>, String>> + Send + '_>>
  {
    Box::pin(async { gpu::get_gpu_memory_usage().await })
  }

  fn sample_gpus(
    &self,
  ) -> Pin<Box<dyn Future<Output = Vec<crate::models::GpuSample>> + Send + '_>> {
    Box::pin(gpu::sample_gpus())
  }
}

impl NetworkPlatform for MacOSPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError> {
    network::get_network_info()
  }
}

impl MotherboardPlatform for MacOSPlatform {
  fn get_motherboard_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::models::hardware::MotherboardInfo, String>>
        + Send
        + '_,
    >,
  > {
    motherboard::get_motherboard_info()
  }
}

impl SuperIoPlatform for MacOSPlatform {
  fn get_super_io_chip_id_diagnostics(&self) -> SuperIoChipIdDiagnostics {
    SuperIoChipIdDiagnostics::unsupported_platform()
  }
}

impl SensorPlatform for MacOSPlatform {
  fn sample_temperatures(&self) -> crate::models::TemperatureSample {
    crate::models::TemperatureSample::unsupported(
      "CPU and named sensor temperature sampling is not implemented for MacOSPlatform",
    )
  }

  fn sample_motherboard_sensors(&self) -> crate::models::MotherboardSensorCollection {
    crate::models::MotherboardSensorCollection::unsupported(
      "Motherboard sensor sampling is available on Windows only",
    )
  }
}

impl ProcessElevationPlatform for MacOSPlatform {
  fn is_process_elevated(&self) -> Result<bool, String> {
    Ok(false)
  }

  fn relaunch_current_process_elevated(&self) -> Result<(), String> {
    Err("Elevated Startup Mode is only supported on Windows.".to_string())
  }
}

impl Platform for MacOSPlatform {}
