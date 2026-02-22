use crate::enums;
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

  /// Get GPU temperature
  fn get_gpu_temperature(
    &self,
    temperature_unit: enums::settings::TemperatureUnit,
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

/// Trait that integrates all platform functionality
pub trait Platform:
  MemoryPlatform + GpuPlatform + NetworkPlatform + MotherboardPlatform
{
}
