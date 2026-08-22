use crate::enums::error::PlatformError;
use crate::models::hardware::{
  GpuMemoryUsage, GraphicInfo, NetworkInfo, SuperIoChipIdDiagnostics,
};
use crate::platform::traits::{
  GpuPlatform, MemoryPlatform, MotherboardPlatform, NetworkPlatform, Platform,
  ProcessElevationPlatform, SensorPlatform, SuperIoPlatform,
};
use std::future::Future;
use std::pin::Pin;

pub mod cache;
pub mod gpu;
pub mod memory;
pub mod network;

pub struct LinuxPlatform;

impl LinuxPlatform {
  pub fn new() -> Result<Self, PlatformError> {
    Ok(Self)
  }
}

impl MemoryPlatform for LinuxPlatform {
  fn get_memory_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::models::hardware::MemoryInfo, PlatformError>>
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
      dyn Future<Output = Result<crate::models::hardware::MemoryInfo, PlatformError>>
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
  ) -> Pin<
    Box<
      dyn Future<Output = Result<super::traits::GpuUsageRaw, PlatformError>> + Send + '_,
    >,
  > {
    Box::pin(gpu::get_gpu_usage())
  }

  fn get_gpu_temperature(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::models::hardware::NameValue>, PlatformError>>
        + Send
        + '_,
    >,
  > {
    Box::pin(gpu::get_gpu_temperature())
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, PlatformError>> + Send + '_>>
  {
    Box::pin(gpu::get_gpu_info())
  }

  fn get_gpu_memory_usage(
    &self,
  ) -> Pin<
    Box<dyn Future<Output = Result<Option<GpuMemoryUsage>, PlatformError>> + Send + '_>,
  > {
    Box::pin(async { Ok(None) })
  }

  fn sample_gpus(
    &self,
  ) -> Pin<Box<dyn Future<Output = Vec<crate::models::GpuSample>> + Send + '_>> {
    Box::pin(gpu::sample_gpus())
  }
}

impl NetworkPlatform for LinuxPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, PlatformError> {
    network::get_network_info()
  }
}

impl MotherboardPlatform for LinuxPlatform {
  fn get_motherboard_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::models::hardware::MotherboardInfo, PlatformError>>
        + Send
        + '_,
    >,
  > {
    Box::pin(async {
      Err(PlatformError::unsupported(
        "get_motherboard_info is not implemented for LinuxPlatform",
      ))
    })
  }
}

impl SuperIoPlatform for LinuxPlatform {
  fn get_super_io_chip_id_diagnostics(&self) -> SuperIoChipIdDiagnostics {
    SuperIoChipIdDiagnostics::unsupported_platform()
  }
}

impl SensorPlatform for LinuxPlatform {
  fn sample_temperatures(&self) -> crate::models::TemperatureSample {
    crate::models::TemperatureSample::unsupported(
      "CPU and named sensor temperature sampling is not implemented for LinuxPlatform",
    )
  }

  fn sample_motherboard_sensors(&self) -> crate::models::MotherboardSensorCollection {
    crate::models::MotherboardSensorCollection::unsupported(
      "Motherboard sensor sampling is available on Windows only",
    )
  }
}

impl ProcessElevationPlatform for LinuxPlatform {
  fn is_process_elevated(&self) -> Result<bool, PlatformError> {
    Ok(false)
  }

  fn relaunch_current_process_elevated(&self) -> Result<(), PlatformError> {
    Err(PlatformError::unsupported(
      "Elevated Startup Mode is only supported on Windows.",
    ))
  }
}

impl Platform for LinuxPlatform {}
