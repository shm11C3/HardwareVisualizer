use crate::services;
use crate::{enums::hardware::DiskKind, utils::formatter::SizeUnit};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use sysinfo;

pub struct HardwareMonitorState {
  pub system: Arc<Mutex<sysinfo::System>>,
  pub cpu_history: Arc<Mutex<VecDeque<f32>>>,
  pub memory_history: Arc<Mutex<VecDeque<f32>>>,
  pub gpu_history: Arc<Mutex<VecDeque<f32>>>,
  pub process_cpu_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  pub process_memory_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  #[allow(dead_code)]
  pub nv_gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  #[allow(dead_code)]
  pub nv_gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
}

#[derive(Serialize, Deserialize, Type, Clone)]
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

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphicInfo {
  pub id: String,
  pub name: String,
  pub vendor_name: String,
  pub clock: u32,
  pub memory_size: String,
  pub memory_size_dedicated: String,
}

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NameValue {
  pub name: String,
  pub value: i32, // 摂氏温度
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
  pub name: String,
  pub size: f32,
  pub size_unit: SizeUnit,
  pub free: f32,
  pub free_unit: SizeUnit,
  pub storage_type: DiskKind,
  pub file_system: String,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
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

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsage {
  pub ip: String,
  pub sent: f32,
  pub sent_unit: SizeUnit,
  pub received: f32,
  pub received_unit: SizeUnit,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
  pub pid: i32,
  pub name: String,
  #[serde(serialize_with = "serialize_usage")]
  pub cpu_usage: f32,
  #[serde(serialize_with = "serialize_usage")]
  pub memory_usage: f32,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SysInfo {
  pub cpu: Option<services::system_info_service::CpuInfo>,
  pub memory: Option<MemoryInfo>,
  pub gpus: Option<Vec<GraphicInfo>>,
  pub storage: Vec<StorageInfo>,
}

fn serialize_usage<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  if x.fract() == 0.0 {
    s.serialize_str(&format!("{x:.0}")) // 整数のみ
  } else {
    s.serialize_str(&format!("{x:.1}")) // 小数点以下1桁まで
  }
}
