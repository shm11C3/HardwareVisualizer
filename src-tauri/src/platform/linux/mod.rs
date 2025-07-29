use crate::enums::hardware::DiskKind;
use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform, Platform};
use crate::structs::hardware::{
  GraphicInfo, MemoryInfo, NetworkInfo, StorageInfo, SysInfo,
};
use crate::utils::formatter::SizeUnit;
use std::future::Future;
use std::pin::Pin;

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
  ) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>> {
    Box::pin(async {
      // Linux ダミー実装
      Ok(MemoryInfo {
        size: "32 GB".to_string(),
        clock: 3600,
        clock_unit: "MHz".to_string(),
        memory_count: 4,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: false,
      })
    })
  }

  fn get_memory_info_detail(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>> {
    Box::pin(async {
      // Linux ダミー実装
      Ok(MemoryInfo {
        size: "32 GB".to_string(),
        clock: 3600,
        clock_unit: "MHz".to_string(),
        memory_count: 4,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: true,
      })
    })
  }
}

impl GpuPlatform for LinuxPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
    Box::pin(async {
      // Linux ダミー実装
      Ok(75.0) // 75% ダミー値
    })
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(async {
      // Linux ダミー実装
      Ok(vec![GraphicInfo {
        id: "GPU-11111111-1111-1111-1111-111111111111".to_string(),
        name: "Linux GPU (Dummy)".to_string(),
        vendor_name: "AMD".to_string(),
        clock: 1800,
        memory_size: "12 GB".to_string(),
        memory_size_dedicated: "12 GB".to_string(),
      }])
    })
  }
}

impl NetworkPlatform for LinuxPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
    // Linux ダミー実装
    Ok(vec![NetworkInfo {
      description: Some("Linux Network Interface (Dummy)".to_string()),
      mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
      ipv4: vec!["10.0.0.100".to_string()],
      ipv6: vec!["2001:db8::1".to_string()],
      link_local_ipv6: vec!["fe80::2%eth0".to_string()],
      ip_subnet: vec!["10.0.0.0/24".to_string()],
      default_ipv4_gateway: vec!["10.0.0.1".to_string()],
      default_ipv6_gateway: vec!["2001:db8::1".to_string()],
    }])
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
        size_unit: SizeUnit::GB,
        free: 1000.0,
        free_unit: SizeUnit::GB,
        storage_type: DiskKind::NVMe,
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
