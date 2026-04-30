use hwviz_core::settings::HardwareArchiveSettings as CoreHardwareArchiveSettings;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
  collections::{HashMap, VecDeque},
  sync::{Arc, Mutex},
};

pub struct MonitorResources {
  pub cpu_history: Arc<Mutex<VecDeque<f32>>>,
  pub memory_history: Arc<Mutex<VecDeque<f32>>>,
  pub process_cpu_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  pub process_memory_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  pub gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  pub gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  pub gpu_dedicated_memory_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  /// Maps gpu_id → gpu_name for archive persistence
  pub gpu_name_map: Arc<Mutex<HashMap<String, String>>>,
}

/// Wire-format mirror of [`hwviz_core::settings::HardwareArchiveSettings`].
///
/// The canonical definition lives in `hwviz_core::settings` so the
/// archive worker (and any future Core consumer) doesn't need to know
/// about Tauri or specta. This App-side struct exists only because the
/// frontend wire format requires `specta::Type`.
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(default, rename_all = "camelCase")]
pub struct HardwareArchiveSettings {
  pub enabled: bool,
  pub scheduled_data_deletion: bool,
  pub refresh_interval_days: u32,
}

impl Default for HardwareArchiveSettings {
  fn default() -> Self {
    CoreHardwareArchiveSettings::default().into()
  }
}

impl From<CoreHardwareArchiveSettings> for HardwareArchiveSettings {
  fn from(value: CoreHardwareArchiveSettings) -> Self {
    Self {
      enabled: value.enabled,
      scheduled_data_deletion: value.scheduled_data_deletion,
      refresh_interval_days: value.refresh_interval_days,
    }
  }
}

impl From<HardwareArchiveSettings> for CoreHardwareArchiveSettings {
  fn from(value: HardwareArchiveSettings) -> Self {
    Self {
      enabled: value.enabled,
      scheduled_data_deletion: value.scheduled_data_deletion,
      refresh_interval_days: value.refresh_interval_days,
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HardwareData {
  pub avg: Option<f32>,
  pub max: Option<f32>,
  pub min: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Debug, Clone)]
pub struct ProcessStatData {
  pub pid: i32,
  pub process_name: String,
  pub cpu_usage: f32,
  pub memory_usage: i32,
  pub execution_sec: i32,
}
