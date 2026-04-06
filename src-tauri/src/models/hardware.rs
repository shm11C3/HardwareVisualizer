use crate::{enums::hardware::DiskKind, utils::formatter::SizeUnit};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use sysinfo;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GpuMonitorData {
  pub gpu_id: String,
  pub gpu_name: String,
  pub gpu_usage: Option<f32>,
  pub gpu_temperature: Option<f32>,
  pub gpu_source: String,
  pub gpu_dedicated_memory_usage_kb: Option<f32>,
  pub gpu_cooler_level: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct HardwareMonitorUpdate {
  pub cpu_usage: f32,
  pub memory_usage: f32,
  pub gpus: Vec<GpuMonitorData>,
  pub processors_usage: Vec<f32>,
}

pub struct HardwareMonitorState {
  pub system: Arc<Mutex<sysinfo::System>>,
  pub cpu_history: Arc<Mutex<VecDeque<f32>>>,
  pub memory_history: Arc<Mutex<VecDeque<f32>>>,
  pub process_cpu_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  pub process_memory_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  pub gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  #[allow(dead_code)]
  pub gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
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
  pub core_count: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpuMemoryUsage {
  pub in_use_bytes: Option<String>,
  pub alloc_bytes: Option<String>,
}

/// GPU usage percentage together with the data-source identifier
/// (e.g. "NVAPI", "ADL", "WMI", "DRM (AMD)", "IOKit")
#[derive(Serialize, Deserialize, Type, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpuUsageResult {
  pub usage: i32,
  pub source: String,
}

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NameValue {
  pub name: String,
  pub value: i32, // Celsius temperature
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
pub struct ProcessInfo {
  /// Process ID
  pub pid: i32,

  /// Process name
  pub name: String,

  /// CPU usage
  #[serde(serialize_with = "serialize_usage")]
  pub cpu_usage: f32,

  /// Memory usage
  #[serde(serialize_with = "serialize_usage")]
  pub memory_usage: f32,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MotherboardInfo {
  pub manufacturer: String,
  pub product: String,
  pub version: String,
  pub serial_number: String,
  pub bios_vendor: String,
  pub bios_version: String,
  pub bios_release_date: String,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SysInfo {
  pub cpu: Option<CpuInfo>,
  pub memory: Option<MemoryInfo>,
  pub gpus: Option<Vec<GraphicInfo>>,
  pub storage: Vec<StorageInfo>,
  pub motherboard: Option<MotherboardInfo>,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
  pub name: String,
  pub vendor: String,
  pub core_count: u32,
  pub clock: u32,
  pub clock_unit: String,
  pub cpu_name: String,
}

fn serialize_usage<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  if x.fract() == 0.0 {
    s.serialize_str(&format!("{x:.0}")) // Integer only
  } else {
    s.serialize_str(&format!("{x:.1}")) // Up to 1 decimal place
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_process_info(cpu: f32, mem: f32) -> ProcessInfo {
    ProcessInfo {
      pid: 1,
      name: "test".to_string(),
      cpu_usage: cpu,
      memory_usage: mem,
    }
  }

  fn make_gpu_monitor_data(gpu_id: &str, name: &str) -> GpuMonitorData {
    GpuMonitorData {
      gpu_id: gpu_id.to_string(),
      gpu_name: name.to_string(),
      gpu_usage: Some(75.0),
      gpu_temperature: Some(65.0),
      gpu_source: "NVAPI".to_string(),
      gpu_dedicated_memory_usage_kb: Some(4096.0),
      gpu_cooler_level: Some(60),
    }
  }

  // ── GpuMonitorData serialization ──

  #[test]
  fn gpu_monitor_data_serialization_camel_case() {
    let data = make_gpu_monitor_data("pci:0:2.0", "RTX 4090");
    let json = serde_json::to_value(&data).unwrap();
    assert!(json.get("gpuId").is_some());
    assert!(json.get("gpuName").is_some());
    assert!(json.get("gpuUsage").is_some());
    assert!(json.get("gpuTemperature").is_some());
    assert!(json.get("gpuSource").is_some());
    assert!(json.get("gpuDedicatedMemoryUsageKb").is_some());
    assert!(json.get("gpuCoolerLevel").is_some());
  }

  #[test]
  fn gpu_monitor_data_with_all_fields() {
    let data = make_gpu_monitor_data("pci:0:2.0", "RTX 4090");
    let json = serde_json::to_value(&data).unwrap();
    assert_eq!(json["gpuId"], "pci:0:2.0");
    assert_eq!(json["gpuName"], "RTX 4090");
    assert_eq!(json["gpuUsage"], 75.0);
    assert_eq!(json["gpuTemperature"], 65.0);
    assert_eq!(json["gpuSource"], "NVAPI");
    assert_eq!(json["gpuDedicatedMemoryUsageKb"], 4096.0);
    assert_eq!(json["gpuCoolerLevel"], 60);
  }

  #[test]
  fn gpu_monitor_data_with_none_optionals() {
    let data = GpuMonitorData {
      gpu_id: "gpu:0".to_string(),
      gpu_name: "Intel GPU".to_string(),
      gpu_usage: None,
      gpu_temperature: None,
      gpu_source: "PDH".to_string(),
      gpu_dedicated_memory_usage_kb: None,
      gpu_cooler_level: None,
    };
    let json = serde_json::to_value(&data).unwrap();
    assert!(json["gpuUsage"].is_null());
    assert!(json["gpuTemperature"].is_null());
    assert!(json["gpuDedicatedMemoryUsageKb"].is_null());
    assert!(json["gpuCoolerLevel"].is_null());
  }

  #[test]
  fn gpu_monitor_data_clone() {
    let data = make_gpu_monitor_data("gpu:0", "GPU");
    let cloned = data.clone();
    assert_eq!(cloned.gpu_id, data.gpu_id);
    assert_eq!(cloned.gpu_name, data.gpu_name);
  }

  // ── HardwareMonitorUpdate serialization ──

  #[test]
  fn hardware_monitor_update_empty_gpus() {
    let update = HardwareMonitorUpdate {
      cpu_usage: 50.0,
      memory_usage: 60.0,
      gpus: vec![],
      processors_usage: vec![25.0, 75.0],
    };
    let json = serde_json::to_value(&update).unwrap();
    assert_eq!(json["gpus"].as_array().unwrap().len(), 0);
  }

  #[test]
  fn hardware_monitor_update_single_gpu() {
    let update = HardwareMonitorUpdate {
      cpu_usage: 50.0,
      memory_usage: 60.0,
      gpus: vec![make_gpu_monitor_data("gpu:0", "RTX 4090")],
      processors_usage: vec![],
    };
    let json = serde_json::to_value(&update).unwrap();
    assert_eq!(json["gpus"].as_array().unwrap().len(), 1);
    assert_eq!(json["gpus"][0]["gpuName"], "RTX 4090");
  }

  #[test]
  fn hardware_monitor_update_multiple_gpus() {
    let update = HardwareMonitorUpdate {
      cpu_usage: 50.0,
      memory_usage: 60.0,
      gpus: vec![
        make_gpu_monitor_data("pci:0:2.0", "RTX 4090"),
        make_gpu_monitor_data("pci:0:3.0", "RX 7900 XTX"),
      ],
      processors_usage: vec![],
    };
    let json = serde_json::to_value(&update).unwrap();
    let gpus = json["gpus"].as_array().unwrap();
    assert_eq!(gpus.len(), 2);
    assert_eq!(gpus[0]["gpuId"], "pci:0:2.0");
    assert_eq!(gpus[1]["gpuId"], "pci:0:3.0");
  }

  // ── serialize_usage via ProcessInfo JSON serialization ──

  #[test]
  fn serialize_usage_integer_values() {
    let info = make_process_info(50.0, 100.0);
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["cpuUsage"], "50");
    assert_eq!(json["memoryUsage"], "100");
  }

  #[test]
  fn serialize_usage_fractional_values() {
    let info = make_process_info(12.5, 33.3);
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["cpuUsage"], "12.5");
    assert_eq!(json["memoryUsage"], "33.3");
  }

  #[test]
  fn serialize_usage_zero() {
    let info = make_process_info(0.0, 0.0);
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["cpuUsage"], "0");
    assert_eq!(json["memoryUsage"], "0");
  }

  #[test]
  fn serialize_usage_small_fraction() {
    let info = make_process_info(0.1, 0.9);
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["cpuUsage"], "0.1");
    assert_eq!(json["memoryUsage"], "0.9");
  }

  // ── ProcessInfo camelCase serialization ──

  #[test]
  fn process_info_camel_case_keys() {
    let info = make_process_info(1.0, 2.0);
    let json = serde_json::to_value(&info).unwrap();
    assert!(json.get("cpuUsage").is_some());
    assert!(json.get("memoryUsage").is_some());
    assert!(json.get("pid").is_some());
    assert!(json.get("name").is_some());
  }

  // ── NameValue serialization ──

  #[test]
  fn name_value_serialization() {
    let nv = NameValue {
      name: "GPU0".to_string(),
      value: 72,
    };
    let json = serde_json::to_value(&nv).unwrap();
    assert_eq!(json["name"], "GPU0");
    assert_eq!(json["value"], 72);
  }

  // ── GpuUsageResult serialization ──

  #[test]
  fn gpu_usage_result_serialization() {
    let result = GpuUsageResult {
      usage: 85,
      source: "NVAPI".to_string(),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["usage"], 85);
    assert_eq!(json["source"], "NVAPI");
  }

  // ── GpuMemoryUsage serialization ──

  #[test]
  fn gpu_memory_usage_with_none_fields() {
    let mem = GpuMemoryUsage {
      in_use_bytes: None,
      alloc_bytes: None,
    };
    let json = serde_json::to_value(&mem).unwrap();
    assert!(json["inUseBytes"].is_null());
    assert!(json["allocBytes"].is_null());
  }

  #[test]
  fn gpu_memory_usage_with_values() {
    let mem = GpuMemoryUsage {
      in_use_bytes: Some("1048576".to_string()),
      alloc_bytes: Some("2097152".to_string()),
    };
    let json = serde_json::to_value(&mem).unwrap();
    assert_eq!(json["inUseBytes"], "1048576");
    assert_eq!(json["allocBytes"], "2097152");
  }
}
