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

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessStatData {
  pub pid: i32,
  pub process_name: String,
  pub cpu_usage: f32,
  pub memory_usage: i32,
  pub execution_sec: i32,
}
