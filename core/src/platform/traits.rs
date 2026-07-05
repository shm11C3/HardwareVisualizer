use crate::enums::error::BackendError;
use crate::models;
use std::future::Future;
use std::pin::Pin;

/// Return type for [`GpuPlatform::get_gpu_usage`]: `(percentage, source_name)`.
pub type GpuUsageRaw = (f32, String);

/// Trait that defines platform-specific memory operations
pub trait MemoryPlatform: Send + Sync {
  /// Get basic memory information
  fn get_memory_info(
    &self,
  ) -> Pin<
    Box<dyn Future<Output = Result<models::hardware::MemoryInfo, String>> + Send + '_>,
  >;

  /// Get detailed memory information (supported platforms only)
  fn get_memory_info_detail(
    &self,
  ) -> Pin<
    Box<dyn Future<Output = Result<models::hardware::MemoryInfo, String>> + Send + '_>,
  >;
}

/// Trait that defines platform-specific GPU operations
#[allow(clippy::type_complexity)]
pub trait GpuPlatform: Send + Sync {
  /// Get GPU usage together with the data-source name
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<GpuUsageRaw, String>> + Send + '_>>;

  /// Get GPU temperatures, always in raw degrees Celsius.
  ///
  /// Presentation conversion (Celsius/Fahrenheit) is the App's
  /// responsibility — Core never reads the user's preferred unit so the
  /// trait stays decoupled from UI preferences.
  fn get_gpu_temperature(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<models::hardware::NameValue>, String>> + Send + '_,
    >,
  >;

  /// Get GPU information
  fn get_gpu_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<models::hardware::GraphicInfo>, String>> + Send + '_,
    >,
  >;

  /// Get realtime GPU memory usage (best-effort)
  fn get_gpu_memory_usage(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Option<models::hardware::GpuMemoryUsage>, String>>
        + Send
        + '_,
    >,
  >;

  /// Collect per-GPU realtime metrics for the monitoring pipeline.
  fn sample_gpus(
    &self,
  ) -> Pin<Box<dyn Future<Output = Vec<models::GpuSample>> + Send + '_>>;
}

/// Trait that defines platform-specific network operations
pub trait NetworkPlatform: Send + Sync {
  /// Get network information
  #[allow(dead_code)]
  fn get_network_info(
    &self,
  ) -> Result<Vec<crate::models::hardware::NetworkInfo>, BackendError>;
}

/// Trait that defines platform-specific motherboard operations
pub trait MotherboardPlatform: Send + Sync {
  /// Get motherboard and BIOS information
  fn get_motherboard_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<models::hardware::MotherboardInfo, String>> + Send + '_,
    >,
  >;
}

/// Trait that defines platform-specific Super I/O chip-id diagnostics.
pub trait SuperIoPlatform: Send + Sync {
  /// Read raw Super I/O chip-id diagnostics.
  ///
  /// This is a blocking, read-only probe. Platforms without a Super I/O
  /// LpcIO path return a result with `platform_supported = false` rather
  /// than erroring, so the caller can branch on support uniformly.
  fn get_super_io_chip_id_diagnostics(
    &self,
  ) -> models::hardware::SuperIoChipIdDiagnostics;
}

/// Trait that defines live CPU / motherboard sensor sampling operations.
pub trait SensorPlatform: Send + Sync {
  /// Sample CPU and named temperature sensors, always in raw degrees Celsius.
  fn sample_temperatures(&self) -> models::TemperatureSample;

  /// Sample live motherboard temperature and fan readings.
  fn sample_motherboard_sensors(&self) -> models::MotherboardSensorCollection;
}

/// Trait that defines process elevation operations.
pub trait ProcessElevationPlatform: Send + Sync {
  /// Returns whether the current process is running with elevated privileges.
  fn is_process_elevated(&self) -> Result<bool, String>;

  /// Relaunch the current executable with elevated privileges.
  fn relaunch_current_process_elevated(&self) -> Result<(), String>;
}

/// Trait that integrates all platform functionality
pub trait Platform:
  MemoryPlatform
  + GpuPlatform
  + NetworkPlatform
  + MotherboardPlatform
  + SuperIoPlatform
  + SensorPlatform
  + ProcessElevationPlatform
{
}
