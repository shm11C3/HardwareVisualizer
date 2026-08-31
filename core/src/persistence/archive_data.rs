//! On-disk shapes consumed by `core::infrastructure::database` writers.
//!
//! These structs are intentionally Tauri-independent and free of
//! `specta::Type` / `serde` derives — the wire format used by the
//! frontend lives App-side in `commands::settings`.

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareData {
  pub avg: Option<f32>,
  pub max: Option<f32>,
  pub min: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareArchiveRow {
  pub cpu: HardwareData,
  pub memory: HardwareData,
  pub cpu_temperature: HardwareData,
  pub cpu_power: HardwareData,
  pub gpu_power: HardwareData,
  pub ane_power: HardwareData,
  pub package_power: HardwareData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuData {
  pub gpu_id: Option<String>,
  pub gpu_name: String,
  pub usage_avg: Option<f32>,
  pub usage_max: Option<f32>,
  pub usage_min: Option<f32>,
  pub temperature_avg: Option<f32>,
  pub temperature_max: Option<i32>,
  pub temperature_min: Option<i32>,
  pub dedicated_memory_avg: Option<i32>,
  pub dedicated_memory_max: Option<i32>,
  pub dedicated_memory_min: Option<i32>,
}

/// One ambient source's contribution to a single archive minute. The
/// minute's timestamp is supplied by the writer, not carried per row -
/// see [`crate::infrastructure::database::ambient_archive::insert_from_pool`].
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientData {
  /// Sensor Source Label the reading is attributed to.
  pub source: String,
  pub temperature: f32,
  pub humidity: Option<f32>,
}

/// One archived one-minute fan-speed summary for a single fan (#2022).
///
/// Row-per-fan rather than fixed columns: how many fans a machine exposes
/// is configuration-dependent, so a fixed column set would either truncate
/// or pad. `source` is the fan's stable channel-derived identifier (the
/// live [`crate::models::MotherboardFanSpeed::name`], e.g. `Fan 1`).
///
/// `rpm` is the minute's average of the readings that were actually
/// archivable. A minute with no such reading writes no row at all, so an
/// unreadable or absent fan stays absent rather than becoming 0 RPM - which
/// is a real Inactive Fan Reading, not a missing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanArchiveRow {
  pub source: String,
  pub rpm: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessStatData {
  pub pid: i32,
  pub process_name: String,
  pub cpu_usage: f32,
  pub memory_usage: i32,
  pub execution_sec: i32,
}
