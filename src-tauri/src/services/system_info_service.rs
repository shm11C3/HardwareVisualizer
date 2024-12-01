use crate::structs;
use crate::utils;

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::MutexGuard;
use sysinfo::{Disks, System};

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
  name: String,
  vendor: String,
  core_count: u32,
  clock: u32,
  clock_unit: String,
  cpu_name: String,
}

///
/// ## CPU情報を取得
///
pub fn get_cpu_info(system: MutexGuard<'_, System>) -> Result<CpuInfo, String> {
  let cpus = system.cpus();

  if cpus.is_empty() {
    return Err("CPU information not available".to_string());
  }

  // CPU情報を収集
  let cpu_info = CpuInfo {
    name: cpus[0].brand().to_string(),
    vendor: utils::formatter::format_vendor_name(cpus[0].vendor_id()),
    core_count: cpus.len() as u32,
    clock: cpus[0].frequency() as u32,
    clock_unit: "MHz".to_string(),
    cpu_name: cpus[0].name().to_string(),
  };

  Ok(cpu_info)
}

pub fn get_storage_info() -> Result<Vec<structs::hardware::StorageInfo>, String> {
  let mut storage_info: Vec<structs::hardware::StorageInfo> = Vec::new();

  let disks = Disks::new_with_refreshed_list();

  for disk in &disks {
    let size = utils::formatter::format_size(disk.total_space(), 2);
    let storage = structs::hardware::StorageInfo {
      name: disk.mount_point().to_string_lossy().into_owned(),
      size: size,
      storage_type: disk.kind().to_string(),
      file_system: disk.file_system().to_string_lossy().into_owned(),
    };

    storage_info.push(storage);
  }

  Ok(storage_info)
}
