use crate::{enums::hardware::DiskKind, utils::formatter::SizeUnit};
use specta::Type;

mod generated {
  include!(concat!(env!("OUT_DIR"), "/hardware_models.rs"));
}

pub use generated::*;

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
  /// Headline CPU temperature in the user's preferred unit. Currently Windows only.
  pub cpu_temperature: Option<f32>,
  /// Verification confidence for the headline CPU temperature.
  pub cpu_temperature_verification: Option<SensorVerification>,
  /// All named temperature sensors (thermal zones) in the user's preferred unit.
  pub sensor_temperatures: Vec<SensorTemperatureValue>,
  /// Motherboard temperature sensors in the user's preferred unit.
  pub motherboard_temperatures: Vec<MotherboardTemperatureValue>,
  /// Motherboard fan speeds in RPM.
  pub motherboard_fan_speeds: Vec<MotherboardFanSpeedValue>,
}

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SensorTemperatureValue {
  pub name: String,
  pub value: i32,
  pub verification: SensorVerification,
}

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MotherboardTemperatureValue {
  pub name: String,
  pub value: i32,
  pub source: String,
  pub verification: SensorVerification,
}

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MotherboardFanSpeedValue {
  pub name: String,
  pub rpm: Option<u32>,
  pub status: FanSpeedStatus,
  pub source: String,
  pub verification: SensorVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SensorVerification {
  Verified,
  Experimental,
}

impl From<hardviz_core::models::SensorVerification> for SensorVerification {
  fn from(src: hardviz_core::models::SensorVerification) -> Self {
    match src {
      hardviz_core::models::SensorVerification::Verified => Self::Verified,
      hardviz_core::models::SensorVerification::Experimental => Self::Experimental,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FanSpeedStatus {
  Active,
  Inactive,
  Invalid,
}

impl From<hardviz_core::models::FanSpeedStatus> for FanSpeedStatus {
  fn from(src: hardviz_core::models::FanSpeedStatus) -> Self {
    match src {
      hardviz_core::models::FanSpeedStatus::Active => Self::Active,
      hardviz_core::models::FanSpeedStatus::Inactive => Self::Inactive,
      hardviz_core::models::FanSpeedStatus::Invalid => Self::Invalid,
    }
  }
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
      cpu_temperature: None,
      cpu_temperature_verification: None,
      sensor_temperatures: vec![],
      motherboard_temperatures: vec![],
      motherboard_fan_speeds: vec![],
    };
    let json = serde_json::to_value(&update).unwrap();
    assert_eq!(json["gpus"].as_array().unwrap().len(), 0);
    assert!(json["cpuTemperature"].is_null());
    assert_eq!(json["sensorTemperatures"].as_array().unwrap().len(), 0);
  }

  #[test]
  fn hardware_monitor_update_single_gpu() {
    let update = HardwareMonitorUpdate {
      cpu_usage: 50.0,
      memory_usage: 60.0,
      gpus: vec![make_gpu_monitor_data("gpu:0", "RTX 4090")],
      processors_usage: vec![],
      cpu_temperature: None,
      cpu_temperature_verification: None,
      sensor_temperatures: vec![],
      motherboard_temperatures: vec![],
      motherboard_fan_speeds: vec![],
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
      cpu_temperature: None,
      cpu_temperature_verification: None,
      sensor_temperatures: vec![],
      motherboard_temperatures: vec![],
      motherboard_fan_speeds: vec![],
    };
    let json = serde_json::to_value(&update).unwrap();
    let gpus = json["gpus"].as_array().unwrap();
    assert_eq!(gpus.len(), 2);
    assert_eq!(gpus[0]["gpuId"], "pci:0:2.0");
    assert_eq!(gpus[1]["gpuId"], "pci:0:3.0");
  }

  #[test]
  fn hardware_monitor_update_with_cpu_and_sensor_temperatures() {
    let update = HardwareMonitorUpdate {
      cpu_usage: 50.0,
      memory_usage: 60.0,
      gpus: vec![],
      processors_usage: vec![],
      cpu_temperature: Some(48.0),
      cpu_temperature_verification: Some(SensorVerification::Experimental),
      sensor_temperatures: vec![
        SensorTemperatureValue {
          name: "CPUZ".to_string(),
          value: 48,
          verification: SensorVerification::Experimental,
        },
        SensorTemperatureValue {
          name: "TZ01".to_string(),
          value: 40,
          verification: SensorVerification::Verified,
        },
      ],
      motherboard_temperatures: vec![],
      motherboard_fan_speeds: vec![],
    };
    let json = serde_json::to_value(&update).unwrap();
    assert_eq!(json["cpuTemperature"], 48.0);
    assert_eq!(json["cpuTemperatureVerification"], "experimental");
    let sensors = json["sensorTemperatures"].as_array().unwrap();
    assert_eq!(sensors.len(), 2);
    assert_eq!(sensors[0]["name"], "CPUZ");
    assert_eq!(sensors[0]["value"], 48);
    assert_eq!(sensors[0]["verification"], "experimental");
    assert_eq!(sensors[1]["name"], "TZ01");
  }

  #[test]
  fn hardware_monitor_update_with_motherboard_sensors() {
    let update = HardwareMonitorUpdate {
      cpu_usage: 50.0,
      memory_usage: 60.0,
      gpus: vec![],
      processors_usage: vec![],
      cpu_temperature: None,
      cpu_temperature_verification: None,
      sensor_temperatures: vec![],
      motherboard_temperatures: vec![MotherboardTemperatureValue {
        name: "SYSTIN".to_string(),
        value: 41,
        source: "NCT6799D / Super I/O".to_string(),
        verification: SensorVerification::Verified,
      }],
      motherboard_fan_speeds: vec![MotherboardFanSpeedValue {
        name: "Fan 1".to_string(),
        rpm: Some(0),
        status: FanSpeedStatus::Inactive,
        source: "NCT6799D / Super I/O".to_string(),
        verification: SensorVerification::Verified,
      }],
    };

    let json = serde_json::to_value(&update).unwrap();
    assert_eq!(json["motherboardTemperatures"][0]["name"], "SYSTIN");
    assert_eq!(json["motherboardTemperatures"][0]["value"], 41);
    assert_eq!(
      json["motherboardTemperatures"][0]["source"],
      "NCT6799D / Super I/O"
    );
    assert_eq!(json["motherboardFanSpeeds"][0]["rpm"], 0);
    assert_eq!(json["motherboardFanSpeeds"][0]["status"], "inactive");
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

  #[test]
  fn generated_conversion_preserves_nested_sys_info_fields() {
    use hardviz_core::{
      enums::hardware as core_enums, models::hardware as core_hw,
      utils::formatter as core_formatter,
    };

    let info: SysInfo = core_hw::SysInfo {
      cpu: Some(core_hw::CpuInfo {
        name: "CPU".to_string(),
        vendor: "Vendor".to_string(),
        core_count: 8,
        clock: 3200,
        clock_unit: "MHz".to_string(),
        cpu_name: "CPU Vendor".to_string(),
      }),
      memory: Some(core_hw::MemoryInfo {
        size: "32 GB".to_string(),
        clock: 5600,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 4,
        memory_type: "DDR5".to_string(),
        is_detailed: true,
      }),
      gpus: Some(vec![core_hw::GraphicInfo {
        id: "gpu-0".to_string(),
        name: "GPU".to_string(),
        vendor_name: "Vendor".to_string(),
        clock: 1800,
        memory_size: "8 GB".to_string(),
        memory_size_dedicated: "8 GB".to_string(),
        core_count: Some("128".to_string()),
      }]),
      storage: vec![core_hw::StorageInfo {
        name: "Disk".to_string(),
        size: 1.0,
        size_unit: core_formatter::SizeUnit::GBytes,
        free: 0.5,
        free_unit: core_formatter::SizeUnit::GBytes,
        storage_type: core_enums::DiskKind::Ssd,
        file_system: "apfs".to_string(),
      }],
      motherboard: Some(core_hw::MotherboardInfo {
        manufacturer: "Board Maker".to_string(),
        product: "Board".to_string(),
        version: "1.0".to_string(),
        serial_number: "serial".to_string(),
        bios_vendor: "BIOS Vendor".to_string(),
        bios_version: "BIOS 1".to_string(),
        bios_release_date: "2026-01-01".to_string(),
      }),
    }
    .into();

    assert_eq!(info.cpu.as_ref().unwrap().core_count, 8);
    assert_eq!(info.memory.as_ref().unwrap().memory_type, "DDR5");
    assert_eq!(info.gpus.as_ref().unwrap()[0].id, "gpu-0");
    assert_eq!(info.storage[0].storage_type, DiskKind::Ssd);
    assert_eq!(info.storage[0].size_unit, SizeUnit::GBytes);
    assert_eq!(
      info.motherboard.as_ref().unwrap().manufacturer,
      "Board Maker"
    );
  }
}
