use crate::enums::hardware::DiskKind;
use crate::enums::settings::TemperatureUnit;
use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform, Platform};
use crate::structs::hardware::{
  GraphicInfo, MemoryInfo, NetworkInfo, StorageInfo, SysInfo,
};
use crate::utils::formatter::SizeUnit;
use std::future::Future;
use std::pin::Pin;

pub mod gpu;
pub mod memory;

/// Windows プラットフォーム実装（ダミー）
pub struct WindowsPlatform;

impl WindowsPlatform {
  pub fn new() -> Result<Self, String> {
    Ok(Self)
  }
}

impl MemoryPlatform for WindowsPlatform {
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

impl GpuPlatform for WindowsPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
    Box::pin(gpu::get_gpu_usage())
  }

  fn get_gpu_temperature(
    &self,
    temperature_unit: TemperatureUnit,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<crate::structs::hardware::NameValue>, String>>
        + Send
        + '_,
    >,
  > {
    Box::pin(gpu::get_gpu_temperature(temperature_unit))
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(async {
      // Windows ダミー実装
      Ok(vec![GraphicInfo {
        id: "GPU-00000000-0000-0000-0000-000000000000".to_string(),
        name: "Windows GPU (Dummy)".to_string(),
        vendor_name: "NVIDIA".to_string(),
        clock: 1500,
        memory_size: "8 GB".to_string(),
        memory_size_dedicated: "8 GB".to_string(),
      }])
    })
  }
}

impl NetworkPlatform for WindowsPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
    // Windows ダミー実装
    Ok(vec![NetworkInfo {
      description: Some("Windows Network Adapter (Dummy)".to_string()),
      mac_address: Some("00:11:22:33:44:55".to_string()),
      ipv4: vec!["192.168.1.100".to_string()],
      ipv6: vec!["fe80::1%eth0".to_string()],
      link_local_ipv6: vec!["fe80::1%eth0".to_string()],
      ip_subnet: vec!["192.168.1.0/24".to_string()],
      default_ipv4_gateway: vec!["192.168.1.1".to_string()],
      default_ipv6_gateway: vec!["fe80::1%eth0".to_string()],
    }])
  }
}

impl Platform for WindowsPlatform {
  fn get_system_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<SysInfo, String>> + Send + '_>> {
    Box::pin(async {
      // Windows ダミー実装
      let memory_info = MemoryInfo {
        size: "16 GB".to_string(),
        clock: 3200,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: true,
      };

      let gpu_info = vec![GraphicInfo {
        id: "GPU-00000000-0000-0000-0000-000000000000".to_string(),
        name: "Windows GPU (Dummy)".to_string(),
        vendor_name: "NVIDIA".to_string(),
        clock: 1500,
        memory_size: "8 GB".to_string(),
        memory_size_dedicated: "8 GB".to_string(),
      }];

      let storage_info = vec![StorageInfo {
        name: "C:".to_string(),
        size: 1000.0,
        size_unit: SizeUnit::GBytes,
        free: 500.0,
        free_unit: SizeUnit::GBytes,
        storage_type: DiskKind::Ssd,
        file_system: "NTFS".to_string(),
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
