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
  pub avg: f32,
  pub max: f32,
  pub min: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GpuData {
  pub name: String,
  pub usage_avg: f32,
  pub usage_max: f32,
  pub usage_min: f32,
  pub temperature_avg: f32,
  pub temperature_max: i32,
  pub temperature_min: i32,
}
