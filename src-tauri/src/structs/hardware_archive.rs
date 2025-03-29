use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct HardwareArchiveSettings {
  pub enabled: bool,
  pub scheduled_data_deletion: bool,
  pub refresh_interval_days: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HardwareData {
  pub avg: Option<f32>,
  pub max: Option<f32>,
  pub min: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GpuData {
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
