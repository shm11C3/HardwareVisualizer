use crate::enums;
use crate::enums::error::BackendError;
use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform, Platform};
use crate::structs::hardware::{
  GraphicInfo, MemoryInfo, NetworkInfo, StorageInfo, SysInfo,
};
use crate::utils::formatter::SizeUnit;
use std::future::Future;
use std::pin::Pin;

pub mod cache;
pub mod gpu;
pub mod memory;
pub mod network;

/// Linux プラットフォーム実装（ダミー）
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
      dyn Future<Output = Result<crate::structs::hardware::MemoryInfo, String>>
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
      dyn Future<Output = Result<crate::structs::hardware::MemoryInfo, String>>
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
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
    Box::pin(gpu::get_gpu_usage())
  }

  fn get_gpu_temperature(
    &self,
    _temperature_unit: enums::settings::TemperatureUnit,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::structs::hardware::NameValue>, String>>
        + Send
        + '_,
    >,
  > {
    Box::pin(async { Err("Not implemented".to_string()) })
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(gpu::get_gpu_info())
  }
}

impl NetworkPlatform for LinuxPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, BackendError> {
    network::get_network_info()
  }
}

impl Platform for LinuxPlatform {
  fn get_system_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<SysInfo, String>> + Send + '_>> {
    Box::pin(async {
      // Linux ダミー実装
      let memory_info = MemoryInfo {
        size: "32 GB".to_string(),
        clock: 3600,
        clock_unit: "MHz".to_string(),
        memory_count: 4,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: true,
      };

      let gpu_info = vec![GraphicInfo {
        id: "GPU-11111111-1111-1111-1111-111111111111".to_string(),
        name: "Linux GPU (Dummy)".to_string(),
        vendor_name: "AMD".to_string(),
        clock: 1800,
        memory_size: "12 GB".to_string(),
        memory_size_dedicated: "12 GB".to_string(),
      }];

      let storage_info = vec![StorageInfo {
        name: "/dev/sda1".to_string(),
        size: 2000.0,
        size_unit: SizeUnit::GBytes,
        free: 1000.0,
        free_unit: SizeUnit::GBytes,
        storage_type: enums::hardware::DiskKind::Ssd,
        file_system: "ext4".to_string(),
      }];

      Ok(SysInfo {
        cpu: None, // CPU情報は別途実装が必要
        memory: Some(memory_info),
        gpus: Some(gpu_info),
        storage: storage_info,
      })
    })
  }
}
