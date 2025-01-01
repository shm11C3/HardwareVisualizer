use crate::{enums::hardware::DiskKind, utils::formatter::SizeUnit};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
  pub size: String,
  pub clock: u32,
  pub clock_unit: String,
  pub memory_count: u32,
  pub total_slots: u32,
  pub memory_type: String,
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
