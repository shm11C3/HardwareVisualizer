use crate::enums;
use crate::structs;
use crate::utils;
use crate::utils::formatter::SizeUnit;

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
    core_count: sysinfo::System::physical_core_count().unwrap_or(0) as u32,
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
    let size = utils::formatter::format_size_with_unit(
      disk.total_space(),
      2,
      Some(SizeUnit::GBytes),
    );
    let free = utils::formatter::format_size_with_unit(
      disk.available_space(),
      2,
      Some(SizeUnit::GBytes),
    );
    let storage = structs::hardware::StorageInfo {
      name: disk.mount_point().to_string_lossy().into_owned(),
      size: size.value,
      size_unit: size.unit,
      free: free.value,
      free_unit: free.unit,
      storage_type: enums::hardware::DiskKind::from(disk.kind()),
      file_system: disk.file_system().to_string_lossy().into_owned(),
    };

    storage_info.push(storage);
  }

  Ok(storage_info)
}
