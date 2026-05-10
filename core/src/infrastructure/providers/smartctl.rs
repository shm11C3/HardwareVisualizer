use crate::models::hardware::{SmartAttribute, SmartDiskInfo, SmartHealthStatus};
use serde_json::Value;
use std::process::{Command, Output};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartctlDevice {
  pub name: String,
  pub device_type: Option<String>,
}

impl SmartctlDevice {
  pub fn new(name: impl Into<String>, device_type: Option<String>) -> Self {
    Self {
      name: name.into(),
      device_type,
    }
  }
}

pub fn scan_devices() -> Result<Vec<SmartctlDevice>, String> {
  let output = run_smartctl(["--scan-open", "-j"])?;
  parse_scan_output(&output)
}

pub fn collect_smart_info_from_scan() -> Result<Vec<SmartDiskInfo>, String> {
  let devices = scan_devices()?;
  collect_smart_info_from_devices(&devices)
}

pub fn collect_smart_info_from_devices(
  devices: &[SmartctlDevice],
) -> Result<Vec<SmartDiskInfo>, String> {
  let mut disks = Vec::new();
  let mut errors = Vec::new();

  for device in devices {
    match read_device_smart_info(device) {
      Ok(info) => disks.push(info),
      Err(e) => errors.push(format!("{}: {e}", device.name)),
    }
  }

  if disks.is_empty() {
    let detail = if errors.is_empty() {
      "no devices found".to_string()
    } else {
      errors.join("; ")
    };
    Err(format!("Failed to collect SMART info: {detail}"))
  } else {
    Ok(disks)
  }
}

pub fn parse_smartctl_json(raw: &str) -> Result<SmartDiskInfo, String> {
  let value: Value = serde_json::from_str(raw)
    .map_err(|e| format!("Failed to parse smartctl JSON: {e}"))?;
  parse_smartctl_value(&value)
}

pub(crate) fn ata_smart_attribute_name(id: u32) -> Option<&'static str> {
  Some(match id {
    1 => "Raw Read Error Rate",
    2 => "Throughput Performance",
    3 => "Spin-Up Time",
    4 => "Start/Stop Count",
    5 => "Reallocated Sector Count",
    7 => "Seek Error Rate",
    8 => "Seek Time Performance",
    9 => "Power-On Hours",
    10 => "Spin Retry Count",
    11 => "Calibration Retry Count",
    12 => "Power Cycle Count",
    13 => "Soft Read Error Rate",
    22 => "Current Helium Level",
    170 => "Available Reserved Space",
    171 => "Program Fail Count",
    172 => "Erase Fail Count",
    173 => "Wear Leveling Count",
    174 => "Unexpected Power Loss Count",
    175 => "Program Fail Count",
    176 => "Erase Fail Count",
    177 => "Wear Leveling Count",
    178 => "Used Reserved Block Count",
    179 => "Used Reserved Block Count Total",
    180 => "Unused Reserved Block Count Total",
    181 => "Program Fail Count Total",
    182 => "Erase Fail Count Total",
    183 => "Runtime Bad Block",
    184 => "End-to-End Error",
    187 => "Reported Uncorrectable Errors",
    188 => "Command Timeout",
    189 => "High Fly Writes",
    190 => "Airflow Temperature",
    191 => "G-Sense Error Rate",
    192 => "Power-Off Retract Count",
    193 => "Load Cycle Count",
    194 => "Temperature Celsius",
    195 => "Hardware ECC Recovered",
    196 => "Reallocation Event Count",
    197 => "Current Pending Sector Count",
    198 => "Offline Uncorrectable Sector Count",
    199 => "UDMA CRC Error Count",
    200 => "Multi-Zone Error Rate",
    201 => "Soft Read Error Rate",
    202 => "Data Address Mark Errors",
    203 => "Run Out Cancel",
    204 => "Soft ECC Correction",
    205 => "Thermal Asperity Rate",
    206 => "Flying Height",
    207 => "Spin High Current",
    208 => "Spin Buzz",
    209 => "Offline Seek Performance",
    210 => "Vibration During Write",
    211 => "Vibration During Write",
    212 => "Shock During Write",
    220 => "Disk Shift",
    221 => "G-Sense Error Rate",
    222 => "Loaded Hours",
    223 => "Load/Unload Retry Count",
    224 => "Load Friction",
    225 => "Load/Unload Cycle Count",
    226 => "Load-in Time",
    227 => "Torque Amplification Count",
    228 => "Power-Off Retract Count",
    230 => "Drive Life Protection Status",
    231 => "SSD Life Left",
    232 => "Available Reserved Space",
    233 => "Media Wearout Indicator",
    234 => "Average Erase Count",
    235 => "Good Block Count",
    240 => "Head Flying Hours",
    241 => "Total LBAs Written",
    242 => "Total LBAs Read",
    243 => "Total LBAs Written Expanded",
    244 => "Total LBAs Read Expanded",
    249 => "NAND Writes",
    250 => "Read Error Retry Rate",
    251 => "Minimum Spares Remaining",
    252 => "Newly Added Bad Flash Block",
    254 => "Free Fall Protection",
    _ => return None,
  })
}

fn read_device_smart_info(device: &SmartctlDevice) -> Result<SmartDiskInfo, String> {
  let mut args = vec!["-a".to_string(), "-j".to_string()];

  if let Some(device_type) = device.device_type.as_deref()
    && !device_type.trim().is_empty()
  {
    args.push("-d".to_string());
    args.push(device_type.to_string());
  }

  args.push(device.name.clone());

  let output = run_smartctl(args)?;
  let stdout = String::from_utf8_lossy(&output.stdout);
  if stdout.trim().is_empty() {
    return Err(command_error(&output));
  }

  parse_smartctl_json(&stdout).map_err(|parse_err| {
    if output.status.success() {
      parse_err
    } else {
      format!("{parse_err}; {}", command_error(&output))
    }
  })
}

fn run_smartctl<I, S>(args: I) -> Result<Output, String>
where
  I: IntoIterator<Item = S>,
  S: AsRef<std::ffi::OsStr>,
{
  Command::new("smartctl")
    .args(args)
    .output()
    .map_err(|e| format!("Failed to execute smartctl: {e}"))
}

fn parse_scan_output(output: &Output) -> Result<Vec<SmartctlDevice>, String> {
  let stdout = String::from_utf8_lossy(&output.stdout);
  let command_err = command_error(output);

  if stdout.trim().is_empty() {
    return Err(command_err);
  }

  if !output.status.success() {
    let devices = parse_scan_json(&stdout)
      .map_err(|parse_err| format!("{parse_err}; {command_err}"))?;

    return if devices.is_empty() {
      Err(command_err)
    } else {
      Ok(devices)
    };
  }

  parse_scan_json(&stdout)
}

fn parse_scan_json(raw: &str) -> Result<Vec<SmartctlDevice>, String> {
  let value: Value = serde_json::from_str(raw)
    .map_err(|e| format!("Failed to parse smartctl scan JSON: {e}"))?;
  let devices = value
    .get("devices")
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(|item| {
          let name = item.get("name").and_then(Value::as_str)?;
          let device_type = item
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);

          Some(SmartctlDevice::new(name, device_type))
        })
        .collect()
    })
    .unwrap_or_default();

  Ok(devices)
}

fn parse_smartctl_value(value: &Value) -> Result<SmartDiskInfo, String> {
  let device = value.get("device");
  let device_name = string_at(device, &["name"])
    .or_else(|| string_at(device, &["info_name"]))
    .or_else(|| last_smartctl_arg(value))
    .unwrap_or_else(|| "unknown".to_string());

  let attributes = parse_ata_attributes(value)
    .into_iter()
    .chain(parse_nvme_health_attributes(value))
    .collect();

  Ok(SmartDiskInfo {
    device_name,
    device_type: string_at(device, &["type"]),
    protocol: string_at(device, &["protocol"]),
    model_name: first_string(value, &["model_name", "device_model", "model_family"]),
    serial_number: first_string(value, &["serial_number"]),
    firmware_version: first_string(value, &["firmware_version"]),
    capacity_bytes: value
      .pointer("/user_capacity/bytes")
      .and_then(Value::as_u64),
    health_status: parse_health_status(value),
    temperature_celsius: parse_temperature_celsius(value),
    power_on_hours: value
      .pointer("/power_on_time/hours")
      .and_then(Value::as_u64)
      .or_else(|| {
        value
          .pointer("/nvme_smart_health_information_log/power_on_hours")
          .and_then(Value::as_u64)
      }),
    power_cycle_count: value
      .get("power_cycle_count")
      .and_then(Value::as_u64)
      .or_else(|| {
        value
          .pointer("/nvme_smart_health_information_log/power_cycles")
          .and_then(Value::as_u64)
      }),
    attributes,
  })
}

fn parse_health_status(value: &Value) -> SmartHealthStatus {
  match value
    .pointer("/smart_status/passed")
    .and_then(Value::as_bool)
  {
    Some(true) => SmartHealthStatus::Passed,
    Some(false) => SmartHealthStatus::Failed,
    None => SmartHealthStatus::Unknown,
  }
}

fn parse_temperature_celsius(value: &Value) -> Option<i32> {
  value
    .pointer("/temperature/current")
    .and_then(Value::as_i64)
    .or_else(|| {
      value
        .pointer("/nvme_smart_health_information_log/temperature")
        .and_then(Value::as_i64)
    })
    .and_then(|temp| i32::try_from(temp).ok())
}

fn parse_ata_attributes(value: &Value) -> Vec<SmartAttribute> {
  value
    .pointer("/ata_smart_attributes/table")
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(|item| {
          let id = item
            .get("id")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok());
          let name = item
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| id.and_then(ata_smart_attribute_name).map(ToOwned::to_owned))?;

          Some(SmartAttribute {
            id,
            name,
            current: item.get("value").and_then(Value::as_u64),
            worst: item.get("worst").and_then(Value::as_u64),
            threshold: item.get("thresh").and_then(Value::as_u64),
            raw_value: smartctl_raw_value(item.get("raw")),
            when_failed: item
              .get("when_failed")
              .and_then(Value::as_str)
              .map(str::trim)
              .filter(|v| !v.is_empty() && *v != "-")
              .map(ToOwned::to_owned),
          })
        })
        .collect()
    })
    .unwrap_or_default()
}

fn parse_nvme_health_attributes(value: &Value) -> Vec<SmartAttribute> {
  let Some(log) = value
    .get("nvme_smart_health_information_log")
    .and_then(Value::as_object)
  else {
    return Vec::new();
  };

  [
    ("critical_warning", "Critical Warning"),
    ("temperature", "Temperature"),
    ("available_spare", "Available Spare"),
    ("available_spare_threshold", "Available Spare Threshold"),
    ("percentage_used", "Percentage Used"),
    ("data_units_read", "Data Units Read"),
    ("data_units_written", "Data Units Written"),
    ("host_reads", "Host Reads"),
    ("host_writes", "Host Writes"),
    ("controller_busy_time", "Controller Busy Time"),
    ("power_cycles", "Power Cycles"),
    ("power_on_hours", "Power-On Hours"),
    ("unsafe_shutdowns", "Unsafe Shutdowns"),
    ("media_errors", "Media Errors"),
    ("num_err_log_entries", "Error Information Log Entries"),
    ("warning_temp_time", "Warning Temperature Time"),
    ("critical_comp_time", "Critical Composite Temperature Time"),
  ]
  .into_iter()
  .filter_map(|(key, name)| {
    let value = log.get(key)?;
    let raw_value = value
      .as_u64()
      .map(|v| v.to_string())
      .or_else(|| value.as_i64().map(|v| v.to_string()))
      .or_else(|| value.as_str().map(ToOwned::to_owned))?;

    Some(SmartAttribute {
      id: None,
      name: name.to_string(),
      current: value.as_u64(),
      worst: None,
      threshold: None,
      raw_value: Some(raw_value),
      when_failed: None,
    })
  })
  .collect()
}

fn smartctl_raw_value(raw: Option<&Value>) -> Option<String> {
  let raw = raw?;
  raw
    .get("string")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      raw
        .get("value")
        .and_then(Value::as_u64)
        .map(|v| v.to_string())
    })
    .or_else(|| raw.as_str().map(ToOwned::to_owned))
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
  keys.iter().find_map(|key| {
    value
      .get(*key)
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|v| !v.is_empty())
      .map(ToOwned::to_owned)
  })
}

fn last_smartctl_arg(value: &Value) -> Option<String> {
  value
    .pointer("/smartctl/argv")
    .and_then(Value::as_array)
    .and_then(|argv| argv.last())
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|v| !v.is_empty())
    .map(ToOwned::to_owned)
}

fn string_at(value: Option<&Value>, path: &[&str]) -> Option<String> {
  let mut current = value?;
  for key in path {
    current = if let Ok(index) = key.parse::<usize>() {
      current.get(index)?
    } else {
      current.get(*key)?
    };
  }

  current
    .as_str()
    .map(str::trim)
    .filter(|v| !v.is_empty())
    .map(ToOwned::to_owned)
}

fn command_error(output: &Output) -> String {
  let stderr = String::from_utf8_lossy(&output.stderr);
  let stdout = String::from_utf8_lossy(&output.stdout);
  let detail = if !stderr.trim().is_empty() {
    stderr.trim()
  } else {
    stdout.trim()
  };

  if detail.is_empty() {
    format!("smartctl exited with status {}", output.status)
  } else {
    format!("smartctl exited with status {}: {detail}", output.status)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(unix)]
  fn test_output(status_code: i32, stdout: &str, stderr: &str) -> Output {
    use std::os::unix::process::ExitStatusExt;

    Output {
      status: std::process::ExitStatus::from_raw(status_code << 8),
      stdout: stdout.as_bytes().to_vec(),
      stderr: stderr.as_bytes().to_vec(),
    }
  }

  #[cfg(windows)]
  fn test_output(status_code: u32, stdout: &str, stderr: &str) -> Output {
    use std::os::windows::process::ExitStatusExt;

    Output {
      status: std::process::ExitStatus::from_raw(status_code),
      stdout: stdout.as_bytes().to_vec(),
      stderr: stderr.as_bytes().to_vec(),
    }
  }

  #[test]
  fn parse_scan_json_extracts_devices() {
    let raw = r#"{
      "devices": [
        {"name": "/dev/sda", "type": "sat"},
        {"name": "/dev/nvme0", "type": "nvme"}
      ]
    }"#;

    let devices = parse_scan_json(raw).expect("scan JSON should parse");

    assert_eq!(
      devices,
      vec![
        SmartctlDevice::new("/dev/sda", Some("sat".to_string())),
        SmartctlDevice::new("/dev/nvme0", Some("nvme".to_string())),
      ]
    );
  }

  #[test]
  fn parse_scan_output_preserves_command_error_on_invalid_failure_output() {
    let output = test_output(2, "not json", "smartctl permission denied");

    let err = parse_scan_output(&output).expect_err("scan should fail");

    assert!(err.contains("Failed to parse smartctl scan JSON"));
    assert!(err.contains("smartctl permission denied"));
  }

  #[test]
  fn parse_smartctl_json_uses_last_argv_as_device_name_fallback() {
    let raw = r#"{
      "smartctl": {"argv": ["smartctl", "-a", "-j", "-d", "sat", "/dev/sda"]},
      "smart_status": {"passed": true}
    }"#;

    let disk = parse_smartctl_json(raw).expect("SMART JSON should parse");

    assert_eq!(disk.device_name, "/dev/sda");
  }

  #[test]
  fn parse_smartctl_json_extracts_ata_fields() {
    let raw = r#"{
      "device": {"name": "/dev/sda", "type": "sat", "protocol": "ATA"},
      "model_name": "Example SSD",
      "serial_number": "ABC123",
      "firmware_version": "1.0",
      "user_capacity": {"bytes": 512110190592},
      "smart_status": {"passed": true},
      "temperature": {"current": 33},
      "power_on_time": {"hours": 1234},
      "power_cycle_count": 56,
      "ata_smart_attributes": {
        "table": [
          {
            "id": 9,
            "name": "Power_On_Hours",
            "value": 99,
            "worst": 99,
            "thresh": 0,
            "raw": {"value": 1234, "string": "1234"}
          }
        ]
      }
    }"#;

    let disk = parse_smartctl_json(raw).expect("SMART JSON should parse");

    assert_eq!(disk.device_name, "/dev/sda");
    assert_eq!(disk.health_status, SmartHealthStatus::Passed);
    assert_eq!(disk.temperature_celsius, Some(33));
    assert_eq!(disk.power_on_hours, Some(1234));
    assert_eq!(disk.power_cycle_count, Some(56));
    assert_eq!(disk.attributes.len(), 1);
    assert_eq!(disk.attributes[0].id, Some(9));
    assert_eq!(disk.attributes[0].raw_value.as_deref(), Some("1234"));
  }

  #[test]
  fn parse_smartctl_json_extracts_nvme_health_log() {
    let raw = r#"{
      "device": {"name": "/dev/nvme0", "type": "nvme", "protocol": "NVMe"},
      "model_name": "Example NVMe",
      "smart_status": {"passed": false},
      "nvme_smart_health_information_log": {
        "temperature": 42,
        "percentage_used": 7,
        "power_on_hours": 100,
        "power_cycles": 11,
        "media_errors": 0
      }
    }"#;

    let disk = parse_smartctl_json(raw).expect("NVMe SMART JSON should parse");

    assert_eq!(disk.health_status, SmartHealthStatus::Failed);
    assert_eq!(disk.temperature_celsius, Some(42));
    assert_eq!(disk.power_on_hours, Some(100));
    assert_eq!(disk.power_cycle_count, Some(11));
    assert!(
      disk
        .attributes
        .iter()
        .any(|attr| attr.name == "Percentage Used" && attr.current == Some(7))
    );
  }
}
