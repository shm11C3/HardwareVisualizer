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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperIoChipIdDiagnostics {
  pub platform_supported: bool,
  pub pawnio: Option<PawnIoRuntimeDiagnostics>,
  pub slots: Vec<SuperIoChipIdSlotProbe>,
  pub error: Option<String>,
}

impl SuperIoChipIdDiagnostics {
  /// Result for platforms that do not have a Super I/O `LpcIO` path.
  ///
  /// Reports `platform_supported = false` with an explanatory message
  /// instead of an error, so callers can branch on support uniformly.
  pub fn unsupported_platform() -> Self {
    Self {
      platform_supported: false,
      pawnio: None,
      slots: Vec::new(),
      error: Some(
        "Super I/O LpcIO diagnostics are available on Windows only".to_string(),
      ),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PawnIoRuntimeDiagnostics {
  pub install_location: Option<String>,
  pub dll_path: Option<String>,
  pub module_path: Option<String>,
  pub pawnio_available: bool,
  pub library_loadable: bool,
  pub driver_openable: bool,
  pub module_loadable: bool,
  pub version: Option<u32>,
  pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperIoChipIdSlotProbe {
  pub slot: u8,
  pub index_port: u16,
  pub data_port: u16,
  pub attempts: Vec<SuperIoChipIdAttempt>,
  pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperIoChipIdAttempt {
  pub vendor: SuperIoVendor,
  pub id_high: Option<u8>,
  pub id_low: Option<u8>,
  pub chip_id: Option<u16>,
  pub absent: bool,
  pub error: Option<String>,
  pub exit_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuperIoVendor {
  Nuvoton,
  Ite,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SmartHealthStatus {
  Passed,
  Failed,
  Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartAttribute {
  pub id: Option<u32>,
  pub name: String,
  pub current: Option<u64>,
  pub worst: Option<u64>,
  pub threshold: Option<u64>,
  pub raw_value: Option<String>,
  pub when_failed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartDiskInfo {
  pub device_name: String,
  pub device_type: Option<String>,
  pub protocol: Option<String>,
  pub model_name: Option<String>,
  pub serial_number: Option<String>,
  pub firmware_version: Option<String>,
  pub capacity_bytes: Option<u64>,
  pub health_status: SmartHealthStatus,
  pub temperature_celsius: Option<i32>,
  pub power_on_hours: Option<u64>,
  pub power_cycle_count: Option<u64>,
  pub attributes: Vec<SmartAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageHealthStatus {
  Good,
  Warning,
  Critical,
  Unknown,
}

impl StorageHealthStatus {
  pub fn as_str(&self) -> &'static str {
    match self {
      StorageHealthStatus::Good => "good",
      StorageHealthStatus::Warning => "warning",
      StorageHealthStatus::Critical => "critical",
      StorageHealthStatus::Unknown => "unknown",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageWarningLevel {
  None,
  Warning,
  Critical,
  Unknown,
}

impl StorageWarningLevel {
  pub fn as_str(&self) -> &'static str {
    match self {
      StorageWarningLevel::None => "none",
      StorageWarningLevel::Warning => "warning",
      StorageWarningLevel::Critical => "critical",
      StorageWarningLevel::Unknown => "unknown",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageHealthRecordDraft {
  pub device_id: String,
  pub date: String,
  pub health_status: StorageHealthStatus,
  pub warning_level: StorageWarningLevel,
  pub temperature_celsius: Option<f32>,
  pub power_on_hours: Option<u64>,
  pub percentage_used: Option<f32>,
  pub available_spare_percent: Option<f32>,
  pub reallocated_sector_count: Option<u64>,
  pub current_pending_sector_count: Option<u64>,
  pub offline_uncorrectable_count: Option<u64>,
  pub media_errors: Option<u64>,
  pub error_log_entries: Option<u64>,
  pub unsafe_shutdown_count: Option<u64>,
  pub warning_reasons: Vec<String>,
  pub collected_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageHealthRecord {
  pub device_id: String,
  pub display_name: String,
  pub model: Option<String>,
  pub protocol: Option<String>,
  pub capacity_bytes: Option<u64>,
  pub date: String,
  pub health_status: StorageHealthStatus,
  pub warning_level: StorageWarningLevel,
  pub temperature_celsius: Option<f32>,
  pub power_on_hours: Option<u64>,
  pub percentage_used: Option<f32>,
  pub available_spare_percent: Option<f32>,
  pub reallocated_sector_count: Option<u64>,
  pub current_pending_sector_count: Option<u64>,
  pub offline_uncorrectable_count: Option<u64>,
  pub media_errors: Option<u64>,
  pub error_log_entries: Option<u64>,
  pub unsafe_shutdown_count: Option<u64>,
  pub warning_reasons: Vec<String>,
  pub collected_at: String,
}

/// One device's Live Storage Health signals (ADR 0006).
///
/// Collected on demand for immediate display and never persisted,
/// unlike the daily [`StorageHealthRecord`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStorageHealth {
  /// Same HMAC-based identity as the daily Storage Health Record, so
  /// displays can join live signals to persisted device rows.
  pub device_id: String,
  pub display_name: String,
  pub temperature_celsius: Option<f32>,
  pub available_spare_percent: Option<f32>,
  pub percentage_used: Option<f32>,
  pub media_errors: Option<u64>,
  pub unsafe_shutdown_count: Option<u64>,
  pub power_on_hours: Option<u64>,
  pub power_cycle_count: Option<u64>,
  pub collected_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageDeviceRecord {
  pub id: String,
  pub display_name: String,
  pub model: Option<String>,
  pub serial_hash: Option<String>,
  pub protocol: Option<String>,
  pub capacity_bytes: Option<u64>,
  pub first_seen_at: String,
  pub last_seen_at: String,
}
