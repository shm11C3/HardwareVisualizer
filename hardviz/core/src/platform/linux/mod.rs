use crate::enums::error::PlatformError;
use crate::models::hardware::{
  GpuMemoryUsage, GraphicInfo, MemoryDetails, MemoryInfo, NetworkInfo,
  SuperIoChipIdDiagnostics,
};
use crate::platform::traits::{
  ElevatedProcessRun, ElevationAvailability, ExternalComponentSetupPlatform, GpuPlatform,
  GpuUsageRaw, MemoryPlatform, MotherboardPlatform, NetworkPlatform, Platform,
  ProcessElevationPlatform, ProcessExitWait, ProcessIdentity, SensorPlatform,
  SuperIoPlatform,
};
use async_trait::async_trait;

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

#[async_trait]
impl MemoryPlatform for LinuxPlatform {
  async fn get_memory_info(&self) -> Result<MemoryInfo, PlatformError> {
    memory::get_memory_info().await
  }

  async fn get_memory_info_detail(&self) -> Result<MemoryInfo, PlatformError> {
    memory::get_memory_info_detail().await
  }

  async fn get_memory_details(&self) -> Result<MemoryDetails, PlatformError> {
    memory::get_memory_details().await
  }
}

#[async_trait]
impl GpuPlatform for LinuxPlatform {
  async fn get_gpu_usage(&self) -> Result<GpuUsageRaw, PlatformError> {
    gpu::get_gpu_usage().await
  }

  async fn get_gpu_temperature(
    &self,
  ) -> Result<Vec<crate::models::hardware::NameValue>, PlatformError> {
    gpu::get_gpu_temperature().await
  }

  async fn get_gpu_info(&self) -> Result<Vec<GraphicInfo>, PlatformError> {
    gpu::get_gpu_info().await
  }

  async fn get_gpu_memory_usage(&self) -> Result<Option<GpuMemoryUsage>, PlatformError> {
    Ok(None)
  }

  async fn sample_gpus(&self) -> Vec<crate::models::GpuSample> {
    gpu::sample_gpus().await
  }
}

impl NetworkPlatform for LinuxPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, PlatformError> {
    network::get_network_info()
  }
}

#[async_trait]
impl MotherboardPlatform for LinuxPlatform {
  async fn get_motherboard_info(
    &self,
  ) -> Result<crate::models::hardware::MotherboardInfo, PlatformError> {
    Err(PlatformError::unsupported(
      "get_motherboard_info is not implemented for LinuxPlatform",
    ))
  }
}

#[async_trait]
impl SuperIoPlatform for LinuxPlatform {
  async fn get_super_io_chip_id_diagnostics(&self) -> SuperIoChipIdDiagnostics {
    SuperIoChipIdDiagnostics::unsupported_platform()
  }
}

#[async_trait]
impl SensorPlatform for LinuxPlatform {
  fn sample_temperatures(&self) -> crate::models::TemperatureSample {
    crate::models::TemperatureSample::unsupported(
      "CPU and named sensor temperature sampling is not implemented for LinuxPlatform",
    )
  }

  fn sample_motherboard_sensors(&self) -> crate::models::MotherboardSensorCollection {
    use crate::infrastructure::providers::{gigabyte_wmi_hwmon, nct6687_hwmon};
    first_recognizing_motherboard_provider(&[
      gigabyte_wmi_hwmon::sample,
      nct6687_hwmon::sample,
    ])
  }
}

/// Each Linux motherboard provider admits its own exact DMI board names, and
/// no board name belongs to two providers, so at most one provider recognizes
/// the board. The first provider that does not report Unsupported answers and
/// the remaining providers are not sampled. When none recognizes the board,
/// the last provider's Unsupported result is returned.
fn first_recognizing_motherboard_provider(
  providers: &[fn() -> crate::models::MotherboardSensorCollection],
) -> crate::models::MotherboardSensorCollection {
  let mut collection = crate::models::MotherboardSensorCollection::unsupported(
    "No Linux motherboard sensor provider is available",
  );
  for provider in providers {
    collection = provider();
    if !matches!(
      collection.availability,
      crate::models::SensorAvailability::Unsupported { .. }
    ) {
      break;
    }
  }
  collection
}

impl ProcessElevationPlatform for LinuxPlatform {
  fn is_process_elevated(&self) -> Result<bool, PlatformError> {
    Ok(false)
  }

  fn relaunch_current_process_elevated(
    &self,
    _args: &[String],
  ) -> Result<(), PlatformError> {
    Err(PlatformError::unsupported(
      "Elevated Startup Mode is only supported on Windows.",
    ))
  }

  fn elevation_availability(&self) -> ElevationAvailability {
    ElevationAvailability::Unsupported
  }

  fn current_process_identity(&self) -> Result<ProcessIdentity, PlatformError> {
    Err(PlatformError::unsupported(
      "Process identity is only supported on Windows.",
    ))
  }

  fn wait_for_process_exit(
    &self,
    _identity: &ProcessIdentity,
  ) -> Result<ProcessExitWait, PlatformError> {
    Err(PlatformError::unsupported(
      "Waiting for another process is only supported on Windows.",
    ))
  }
}

impl ExternalComponentSetupPlatform for LinuxPlatform {
  fn external_component_setup_status(
    &self,
    plan: &crate::external_component_setup::ExternalComponentSetupPlan,
  ) -> crate::external_component_setup::ExternalComponentSetupStatus {
    crate::external_component_setup::ExternalComponentSetupStatus::unsupported_platform(
      plan,
    )
  }

  fn run_external_component_setup(
    &self,
    plan: &crate::external_component_setup::ExternalComponentSetupPlan,
  ) -> crate::external_component_setup::ExternalComponentSetupResult {
    crate::external_component_setup::ExternalComponentSetupResult::failed(
      plan.component,
      crate::external_component_setup::SetupFailureStage::UnsupportedPlatform,
      "External Component Setup is available on Windows only.",
    )
  }

  fn refresh_external_component_files(
    &self,
    plan: &crate::external_component_setup::ExternalComponentSetupPlan,
  ) -> crate::external_component_setup::ExternalComponentSetupResult {
    self.run_external_component_setup(plan)
  }

  fn run_current_executable_elevated(
    &self,
    _args: &[String],
  ) -> Result<ElevatedProcessRun, PlatformError> {
    Err(PlatformError::unsupported(
      "Elevated process launch is only supported on Windows.",
    ))
  }
}

impl Platform for LinuxPlatform {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{
    MotherboardSensorCollection, MotherboardTemperature, SensorAvailability,
  };
  use std::cell::Cell;

  thread_local! {
    static CALLS: Cell<u8> = const { Cell::new(0) };
  }

  fn counted(collection: MotherboardSensorCollection) -> MotherboardSensorCollection {
    CALLS.set(CALLS.get() + 1);
    collection
  }

  fn other_board() -> MotherboardSensorCollection {
    counted(MotherboardSensorCollection::unsupported("other board"))
  }

  fn recognized_unavailable() -> MotherboardSensorCollection {
    counted(MotherboardSensorCollection::unavailable("endpoint missing"))
  }

  fn recognized_available() -> MotherboardSensorCollection {
    let mut collection = MotherboardSensorCollection::default();
    collection.sample.temperatures.push(MotherboardTemperature {
      name: "Temperature 1".to_owned(),
      temperature: 21.0,
      source: "test".to_owned(),
    });
    counted(collection)
  }

  fn run(
    providers: &[fn() -> MotherboardSensorCollection],
  ) -> (MotherboardSensorCollection, u8) {
    CALLS.set(0);
    let collection = first_recognizing_motherboard_provider(providers);
    (collection, CALLS.get())
  }

  #[test]
  fn the_first_provider_that_recognizes_the_board_answers() {
    let (collection, calls) = run(&[recognized_available, recognized_unavailable]);
    assert_eq!(collection, recognized_available());
    assert_eq!(calls, 1);

    // An Unavailable result on the recognized board is kept, not replaced
    // by a later provider.
    let (collection, calls) = run(&[recognized_unavailable, recognized_available]);
    assert_eq!(collection, recognized_unavailable());
    assert_eq!(calls, 1);

    let (collection, calls) = run(&[other_board, recognized_available]);
    assert_eq!(collection, recognized_available());
    assert_eq!(calls, 2);
  }

  #[test]
  fn an_unrecognized_board_keeps_the_last_unsupported_result() {
    let (collection, calls) = run(&[other_board, other_board]);
    assert_eq!(collection, other_board());
    assert_eq!(calls, 2);
    let (collection, _) = run(&[]);
    assert!(matches!(
      collection.availability,
      SensorAvailability::Unsupported { .. }
    ));
  }
}
