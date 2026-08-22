use crate::enums::error::PlatformError;
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
  pub fn new() -> Result<Self, PlatformError> {
    Ok(Self)
  }
}

impl MemoryPlatform for MacOSPlatform {
  fn get_memory_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, PlatformError>> + Send + '_>> {
    Box::pin(async {
      task::spawn_blocking(memory::get_memory_info)
        .await
        .map_err(|e| PlatformError::fault(format!("Failed to join memory task: {e}")))?
    })
  }

  fn get_memory_info_detail(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, PlatformError>> + Send + '_>> {
    Box::pin(async {
      // macOS is not supported yet (build-only stub)
      Err(PlatformError::unsupported(
        "get_memory_info_detail is not implemented for MacOSPlatform",
      ))
    })
  }
}

impl GpuPlatform for MacOSPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<super::traits::GpuUsageRaw, PlatformError>> + Send + '_,
    >,
  > {
    Box::pin(async { gpu::get_gpu_usage().await })
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
    Box::pin(async {
      // macOS is not supported yet (build-only stub)
      Err(PlatformError::unsupported(
        "get_gpu_temperature is not implemented for MacOSPlatform",
      ))
    })
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, PlatformError>> + Send + '_>>
  {
    Box::pin(async { gpu::get_gpu_info().await })
  }

  fn get_gpu_memory_usage(
    &self,
  ) -> Pin<
    Box<dyn Future<Output = Result<Option<GpuMemoryUsage>, PlatformError>> + Send + '_>,
  > {
    Box::pin(async { gpu::get_gpu_memory_usage().await })
  }

  fn sample_gpus(
    &self,
  ) -> Pin<Box<dyn Future<Output = Vec<crate::models::GpuSample>> + Send + '_>> {
    Box::pin(gpu::sample_gpus())
  }

  fn sample_power_draw(&self) -> crate::models::PowerDraw {
    gpu::sample_power_draw()
  }
}

impl NetworkPlatform for MacOSPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, PlatformError> {
    network::get_network_info()
  }
}

impl MotherboardPlatform for MacOSPlatform {
  fn get_motherboard_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::models::hardware::MotherboardInfo, PlatformError>>
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
  fn is_process_elevated(&self) -> Result<bool, PlatformError> {
    Ok(false)
  }

  fn relaunch_current_process_elevated(&self) -> Result<(), PlatformError> {
    Err(PlatformError::unsupported(
      "Elevated Startup Mode is only supported on Windows.",
    ))
  }
}

impl Platform for MacOSPlatform {}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  #[tokio::test]
  async fn build_only_stubs_classify_as_unsupported() {
    let platform = MacOSPlatform::new().expect("MacOSPlatform::new never fails");

    assert!(matches!(
      platform.get_memory_info_detail().await,
      Err(PlatformError::Unsupported { .. })
    ));
    assert!(matches!(
      platform.get_gpu_temperature().await,
      Err(PlatformError::Unsupported { .. })
    ));
  }

  #[test]
  fn elevation_relaunch_classifies_as_unsupported() {
    let platform = MacOSPlatform::new().expect("MacOSPlatform::new never fails");

    assert!(matches!(
      platform.relaunch_current_process_elevated(),
      Err(PlatformError::Unsupported { .. })
    ));
    assert_eq!(platform.is_process_elevated(), Ok(false));
  }
}
