use crate::utils;

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::MutexGuard;
use sysinfo::System;

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
