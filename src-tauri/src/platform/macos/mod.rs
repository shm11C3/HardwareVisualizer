use crate::enums::hardware::DiskKind;
use crate::models::hardware::{
  GraphicInfo, MemoryInfo, NetworkInfo, StorageInfo, SysInfo,
};
use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform, Platform};
use crate::utils::formatter::SizeUnit;
use std::future::Future;
use std::pin::Pin;

/// macOS プラットフォーム実装（ダミー）
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
      // macOS ダミー実装
      Ok(MemoryInfo {
        size: "64 GB".to_string(),
        clock: 4800,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 2,
        memory_type: "DDR5".to_string(),
        is_detailed: false,
      })
    })
  }

  fn get_memory_info_detail(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<MemoryInfo, String>> + Send + '_>> {
    Box::pin(async {
      // macOS ダミー実装
      Ok(MemoryInfo {
        size: "64 GB".to_string(),
        clock: 4800,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 2,
        memory_type: "DDR5".to_string(),
        is_detailed: true,
      })
    })
  }
}

impl GpuPlatform for MacOSPlatform {
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
    Box::pin(async {
      // macOS ダミー実装
      Ok(30.0) // 30% ダミー値
    })
  }

  fn get_gpu_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<Vec<GraphicInfo>, String>> + Send + '_>> {
    Box::pin(async {
      // macOS ダミー実装
      Ok(vec![GraphicInfo {
        id: "GPU-22222222-2222-2222-2222-222222222222".to_string(),
        name: "Apple M1 Pro (Dummy)".to_string(),
        vendor_name: "Apple".to_string(),
        clock: 1000,
        memory_size: "16 GB".to_string(),
        memory_size_dedicated: "16 GB".to_string(),
      }])
    })
  }
}

impl NetworkPlatform for MacOSPlatform {
  fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
    // macOS ダミー実装
    Ok(vec![NetworkInfo {
      description: Some("macOS Network Interface (Dummy)".to_string()),
      mac_address: Some("11:22:33:44:55:66".to_string()),
      ipv4: vec!["172.16.0.100".to_string()],
      ipv6: vec!["2001:db8:85a3::8a2e:370:7334".to_string()],
      link_local_ipv6: vec!["fe80::3%en0".to_string()],
      ip_subnet: vec!["172.16.0.0/16".to_string()],
      default_ipv4_gateway: vec!["172.16.0.1".to_string()],
      default_ipv6_gateway: vec!["2001:db8:85a3::1".to_string()],
    }])
  }
}

impl Platform for MacOSPlatform {
  fn get_system_info(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<SysInfo, String>> + Send + '_>> {
    Box::pin(async {
      // macOS ダミー実装
      let memory_info = MemoryInfo {
        size: "64 GB".to_string(),
        clock: 4800,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 2,
        memory_type: "DDR5".to_string(),
        is_detailed: true,
      };

      let gpu_info = vec![GraphicInfo {
        id: "GPU-22222222-2222-2222-2222-222222222222".to_string(),
        name: "Apple M1 Pro (Dummy)".to_string(),
        vendor_name: "Apple".to_string(),
        clock: 1000,
        memory_size: "16 GB".to_string(),
        memory_size_dedicated: "16 GB".to_string(),
      }];

      let storage_info = vec![StorageInfo {
        name: "Macintosh HD".to_string(),
        size: 1024.0,
        size_unit: SizeUnit::GB,
        free: 512.0,
        free_unit: SizeUnit::GB,
        storage_type: DiskKind::SSD,
        file_system: "APFS".to_string(),
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
