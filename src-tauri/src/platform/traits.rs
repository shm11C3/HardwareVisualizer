use crate::enums;
use crate::enums::error::BackendError;
use crate::models;
use std::future::Future;
use std::pin::Pin;

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
pub trait GpuPlatform: Send + Sync {
  /// Get GPU usage
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>>;

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
}

/// Trait that defines platform-specific network operations
pub trait NetworkPlatform: Send + Sync {
  /// Get network information
  #[allow(dead_code)]
  fn get_network_info(
    &self,
  ) -> Result<Vec<crate::models::hardware::NetworkInfo>, BackendError>;
}

/// Trait that integrates all platform functionality
pub trait Platform: MemoryPlatform + GpuPlatform + NetworkPlatform {}
