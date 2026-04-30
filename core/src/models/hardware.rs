//! POJO mirrors of the wire-format hardware types defined in
//! `src-tauri/src/models/hardware.rs`.
//!
//! These types are returned by the Core platform sensors. The wire types
//! (with `specta::Type` for tauri-specta TypeScript bindings) live in
//! `src-tauri`; commands and services convert between the two via `From`
//! impls at the boundary.
//!
//! Most types derive `Serialize` / `Deserialize` because the Linux
//! platform layer caches some of them to disk via
//! `crate::platform::linux::cache` (JSON round-trip).

use serde::{Deserialize, Serialize};

use crate::enums::hardware::DiskKind;
use crate::utils::formatter::SizeUnit;

/// `#[serde(rename_all = "camelCase")]` matches the format the Linux
/// platform layer wrote to its on-disk cache before this type moved
/// into Core, so existing cache files stay readable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
  pub size: String,
  pub clock: u32,
  pub clock_unit: String,
  pub memory_count: u32,
  pub total_slots: u32,
  pub memory_type: String,
  pub is_detailed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicInfo {
  pub id: String,
  pub name: String,
  pub vendor_name: String,
  pub clock: u32,
  pub memory_size: String,
  pub memory_size_dedicated: String,
  pub core_count: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuMemoryUsage {
  pub in_use_bytes: Option<String>,
  pub alloc_bytes: Option<String>,
}

/// GPU usage percentage together with the data-source identifier
/// (e.g. "NVAPI", "ADL", "WMI", "DRM (AMD)", "IOKit").
#[derive(Debug, Clone, PartialEq)]
pub struct GpuUsageResult {
  pub usage: i32,
  pub source: String,
}

/// Generic `(name, integer)` pair. Used by the GPU temperature API,
/// where `value` is always in raw degrees Celsius — presentation
/// conversion (Celsius/Fahrenheit) lives at the App-side boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct NameValue {
  pub name: String,
  pub value: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StorageInfo {
  pub name: String,
  pub size: f32,
  pub size_unit: SizeUnit,
  pub free: f32,
  pub free_unit: SizeUnit,
  pub storage_type: DiskKind,
  pub file_system: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkInfo {
  pub description: Option<String>,
  pub mac_address: Option<String>,
  pub ipv4: Vec<String>,
  pub ipv6: Vec<String>,
  pub link_local_ipv6: Vec<String>,
  pub ip_subnet: Vec<String>,
  pub default_ipv4_gateway: Vec<String>,
  pub default_ipv6_gateway: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
  pub pid: i32,
  pub name: String,
  pub cpu_usage: f32,
  pub memory_usage: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotherboardInfo {
  pub manufacturer: String,
  pub product: String,
  pub version: String,
  pub serial_number: String,
  pub bios_vendor: String,
  pub bios_version: String,
  pub bios_release_date: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuInfo {
  pub name: String,
  pub vendor: String,
  pub core_count: u32,
  pub clock: u32,
  pub clock_unit: String,
  pub cpu_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SysInfo {
  pub cpu: Option<CpuInfo>,
  pub memory: Option<MemoryInfo>,
  pub gpus: Option<Vec<GraphicInfo>>,
  pub storage: Vec<StorageInfo>,
  pub motherboard: Option<MotherboardInfo>,
}
