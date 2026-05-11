use crate::infrastructure::providers::smartctl;
use crate::models::hardware::{SmartAttribute, SmartDiskInfo, SmartHealthStatus};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use wmi::WMIConnection;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsStorageDriverFailurePredictStatus {
  instance_name: String,
  predict_failure: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsStorageDriverFailurePredictData {
  instance_name: String,
  vendor_specific: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsStorageDriverFailurePredictThresholds {
  instance_name: String,
  vendor_specific: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32DiskDrive {
  #[serde(rename = "DeviceID")]
  device_id: Option<String>,
  #[serde(rename = "PNPDeviceID")]
  pnp_device_id: Option<String>,
  model: Option<String>,
  serial_number: Option<String>,
  firmware_revision: Option<String>,
  size: Option<u64>,
  interface_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsftPhysicalDisk {
  device_id: Option<String>,
  friendly_name: Option<String>,
  serial_number: Option<String>,
  firmware_version: Option<String>,
  size: Option<u64>,
  bus_type: Option<u16>,
  health_status: Option<u16>,
  media_type: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsftStorageReliabilityCounter {
  device_id: Option<String>,
  temperature: Option<u8>,
  temperature_max: Option<u8>,
  read_errors_total: Option<u64>,
  read_errors_corrected: Option<u64>,
  read_errors_uncorrected: Option<u64>,
  write_errors_total: Option<u64>,
  write_errors_corrected: Option<u64>,
  write_errors_uncorrected: Option<u64>,
  start_stop_cycle_count: Option<u32>,
  load_unload_cycle_count: Option<u32>,
  wear: Option<u8>,
  power_on_hours: Option<u16>,
  read_latency_max: Option<u64>,
  write_latency_max: Option<u64>,
  flush_latency_max: Option<u64>,
}

pub fn get_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  let mut errors = Vec::new();

  match query_wmi_smart_info() {
    Ok(disks) if !disks.is_empty() => return Ok(disks),
    Ok(_) => errors.push("legacy ROOT\\WMI returned no SMART devices".to_string()),
    Err(e) => errors.push(format!("legacy ROOT\\WMI failed: {e}")),
  }

  match query_storage_wmi_smart_info() {
    Ok(disks) if !disks.is_empty() => return Ok(disks),
    Ok(_) => errors
      .push("ROOT\\Microsoft\\Windows\\Storage returned no physical disks".to_string()),
    Err(e) => errors.push(format!("Storage WMI failed: {e}")),
  }

  smartctl::collect_smart_info_from_scan().map_err(|smartctl_error| {
    errors.push(format!("smartctl failed: {smartctl_error}"));
    format!(
      "Failed to collect SMART info from Windows fallbacks: {}",
      errors.join("; ")
    )
  })
}

fn query_wmi_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  let statuses: Vec<MsStorageDriverFailurePredictStatus> = wmi_query_in_namespace(
    "ROOT\\WMI",
    "SELECT InstanceName, PredictFailure FROM MSStorageDriver_FailurePredictStatus",
  )?;

  let data_by_instance: HashMap<String, Vec<u8>> =
    wmi_query_in_namespace::<MsStorageDriverFailurePredictData>(
      "ROOT\\WMI",
      "SELECT InstanceName, VendorSpecific FROM MSStorageDriver_FailurePredictData",
    )
    .unwrap_or_default()
    .into_iter()
    .filter_map(|item| item.vendor_specific.map(|data| (item.instance_name, data)))
    .collect();

  let thresholds_by_instance: HashMap<String, Vec<u8>> =
    wmi_query_in_namespace::<MsStorageDriverFailurePredictThresholds>(
      "ROOT\\WMI",
      "SELECT InstanceName, VendorSpecific FROM MSStorageDriver_FailurePredictThresholds",
    )
    .unwrap_or_default()
    .into_iter()
    .filter_map(|item| item.vendor_specific.map(|data| (item.instance_name, data)))
    .collect();

  let disk_drives: Vec<Win32DiskDrive> = wmi_query_in_namespace(
    "ROOT\\CIMV2",
    "SELECT DeviceID, PNPDeviceID, Model, SerialNumber, FirmwareRevision, Size, InterfaceType FROM Win32_DiskDrive",
  )
  .unwrap_or_default();

  Ok(
    statuses
      .into_iter()
      .map(|status| {
        let attributes = data_by_instance
          .get(&status.instance_name)
          .map(|data| {
            parse_vendor_specific_attributes(
              data,
              thresholds_by_instance
                .get(&status.instance_name)
                .map(Vec::as_slice),
            )
          })
          .unwrap_or_default();

        let disk_drive = find_disk_drive(&status.instance_name, &disk_drives);

        SmartDiskInfo {
          device_name: disk_drive
            .and_then(|disk| trimmed(disk.device_id.as_deref()))
            .unwrap_or_else(|| status.instance_name.clone()),
          device_type: disk_drive
            .and_then(|disk| trimmed(disk.interface_type.as_deref())),
          protocol: disk_drive.and_then(|disk| trimmed(disk.interface_type.as_deref())),
          model_name: disk_drive.and_then(|disk| trimmed(disk.model.as_deref())),
          serial_number: disk_drive
            .and_then(|disk| trimmed(disk.serial_number.as_deref())),
          firmware_version: disk_drive
            .and_then(|disk| trimmed(disk.firmware_revision.as_deref())),
          capacity_bytes: disk_drive.and_then(|disk| disk.size),
          health_status: if status.predict_failure {
            SmartHealthStatus::Failed
          } else {
            SmartHealthStatus::Passed
          },
          temperature_celsius: temperature_from_attributes(&attributes),
          power_on_hours: raw_attribute_value(&attributes, 9),
          power_cycle_count: raw_attribute_value(&attributes, 12),
          attributes,
        }
      })
      .collect(),
  )
}

fn query_storage_wmi_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  let physical_disks: Vec<MsftPhysicalDisk> = wmi_query_in_namespace(
    "ROOT\\Microsoft\\Windows\\Storage",
    "SELECT * FROM MSFT_PhysicalDisk",
  )?;

  let reliability_counters: Vec<MsftStorageReliabilityCounter> = wmi_query_in_namespace(
    "ROOT\\Microsoft\\Windows\\Storage",
    "SELECT * FROM MSFT_StorageReliabilityCounter",
  )
  .unwrap_or_default();

  let counters_by_device: HashMap<String, MsftStorageReliabilityCounter> =
    reliability_counters
      .into_iter()
      .filter_map(|counter| {
        let device_id = trimmed(counter.device_id.as_deref())?;
        Some((normalize_identifier(&device_id), counter))
      })
      .collect();

  Ok(
    physical_disks
      .iter()
      .map(|disk| {
        let counter_key =
          trimmed(disk.device_id.as_deref()).map(|id| normalize_identifier(&id));
        let counter = counter_key
          .as_deref()
          .and_then(|key| counters_by_device.get(key));

        storage_wmi_disk_to_smart_info(disk, counter)
      })
      .collect(),
  )
}

fn storage_wmi_disk_to_smart_info(
  disk: &MsftPhysicalDisk,
  counter: Option<&MsftStorageReliabilityCounter>,
) -> SmartDiskInfo {
  SmartDiskInfo {
    device_name: trimmed(disk.device_id.as_deref())
      .or_else(|| trimmed(disk.friendly_name.as_deref()))
      .unwrap_or_else(|| "unknown".to_string()),
    device_type: disk
      .media_type
      .and_then(storage_media_type_label)
      .map(ToOwned::to_owned),
    protocol: disk
      .bus_type
      .and_then(storage_bus_type_label)
      .map(ToOwned::to_owned),
    model_name: trimmed(disk.friendly_name.as_deref()),
    serial_number: trimmed(disk.serial_number.as_deref()),
    firmware_version: trimmed(disk.firmware_version.as_deref()),
    capacity_bytes: disk.size,
    health_status: storage_health_status(disk.health_status),
    temperature_celsius: counter.and_then(|c| c.temperature).map(i32::from),
    power_on_hours: counter.and_then(|c| c.power_on_hours).map(u64::from),
    power_cycle_count: None,
    attributes: storage_reliability_attributes(counter),
  }
}

fn storage_health_status(value: Option<u16>) -> SmartHealthStatus {
  match value {
    Some(0) => SmartHealthStatus::Passed,
    Some(1) => SmartHealthStatus::Warning,
    Some(2) => SmartHealthStatus::Failed,
    Some(5) | None => SmartHealthStatus::Unknown,
    _ => SmartHealthStatus::Unknown,
  }
}

fn storage_bus_type_label(value: u16) -> Option<&'static str> {
  Some(match value {
    0 => "Unknown",
    1 => "SCSI",
    2 => "ATAPI",
    3 => "ATA",
    4 => "IEEE 1394",
    5 => "SSA",
    6 => "Fibre Channel",
    7 => "USB",
    8 => "RAID",
    9 => "iSCSI",
    10 => "SAS",
    11 => "SATA",
    12 => "SD",
    13 => "MMC",
    15 => "File Backed Virtual",
    16 => "Storage Spaces",
    17 => "NVMe",
    _ => return None,
  })
}

fn storage_media_type_label(value: u16) -> Option<&'static str> {
  Some(match value {
    0 => "Unspecified",
    3 => "HDD",
    4 => "SSD",
    5 => "SCM",
    _ => return None,
  })
}

fn storage_reliability_attributes(
  counter: Option<&MsftStorageReliabilityCounter>,
) -> Vec<SmartAttribute> {
  let Some(counter) = counter else {
    return Vec::new();
  };

  let mut attributes = Vec::new();
  push_storage_attribute(
    &mut attributes,
    None,
    "Temperature",
    counter.temperature.map(u64::from),
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Temperature Max",
    counter.temperature_max.map(u64::from),
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Percentage Used",
    counter.wear.map(u64::from),
  );
  push_storage_attribute(
    &mut attributes,
    Some(9),
    "Power-On Hours",
    counter.power_on_hours.map(u64::from),
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Read Errors Total",
    counter.read_errors_total,
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Read Errors Corrected",
    counter.read_errors_corrected,
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Read Errors Uncorrected",
    counter.read_errors_uncorrected,
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Write Errors Total",
    counter.write_errors_total,
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Write Errors Corrected",
    counter.write_errors_corrected,
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Write Errors Uncorrected",
    counter.write_errors_uncorrected,
  );

  if counter.read_errors_uncorrected.is_some()
    || counter.write_errors_uncorrected.is_some()
  {
    let media_errors = counter
      .read_errors_uncorrected
      .unwrap_or(0)
      .saturating_add(counter.write_errors_uncorrected.unwrap_or(0));
    push_storage_attribute(&mut attributes, None, "Media Errors", Some(media_errors));
  }

  push_storage_attribute(
    &mut attributes,
    Some(4),
    "Start/Stop Count",
    counter.start_stop_cycle_count.map(u64::from),
  );
  push_storage_attribute(
    &mut attributes,
    Some(193),
    "Load/Unload Cycle Count",
    counter.load_unload_cycle_count.map(u64::from),
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Read Latency Max",
    counter.read_latency_max,
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Write Latency Max",
    counter.write_latency_max,
  );
  push_storage_attribute(
    &mut attributes,
    None,
    "Flush Latency Max",
    counter.flush_latency_max,
  );

  attributes
}

fn push_storage_attribute(
  attributes: &mut Vec<SmartAttribute>,
  id: Option<u32>,
  name: &str,
  value: Option<u64>,
) {
  if let Some(value) = value {
    attributes.push(SmartAttribute {
      id,
      name: name.to_string(),
      current: Some(value),
      worst: None,
      threshold: None,
      raw_value: Some(value.to_string()),
      when_failed: None,
    });
  }
}

fn parse_vendor_specific_attributes(
  data: &[u8],
  thresholds: Option<&[u8]>,
) -> Vec<SmartAttribute> {
  let threshold_map = thresholds.map(parse_threshold_map).unwrap_or_default();
  let mut attributes = Vec::new();

  for index in 0..30 {
    let offset = 2 + index * 12;
    if offset + 12 > data.len() {
      break;
    }

    let id = data[offset] as u32;
    if id == 0 {
      continue;
    }

    let raw = raw_u48_le(&data[offset + 5..offset + 11]);

    attributes.push(SmartAttribute {
      id: Some(id),
      name: smartctl::ata_smart_attribute_name(id)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("SMART Attribute {id}")),
      current: Some(data[offset + 3] as u64),
      worst: Some(data[offset + 4] as u64),
      threshold: threshold_map.get(&id).copied(),
      raw_value: Some(raw.to_string()),
      when_failed: None,
    });
  }

  attributes
}

fn parse_threshold_map(data: &[u8]) -> HashMap<u32, u64> {
  let mut thresholds = HashMap::new();

  for index in 0..30 {
    let offset = 2 + index * 12;
    if offset + 2 > data.len() {
      break;
    }

    let id = data[offset] as u32;
    if id == 0 {
      continue;
    }

    thresholds.insert(id, data[offset + 1] as u64);
  }

  thresholds
}

fn raw_u48_le(bytes: &[u8]) -> u64 {
  bytes
    .iter()
    .take(6)
    .enumerate()
    .fold(0, |acc, (index, byte)| {
      acc | ((*byte as u64) << (index * 8))
    })
}

fn temperature_from_attributes(attributes: &[SmartAttribute]) -> Option<i32> {
  [194, 190].into_iter().find_map(|id| {
    let raw = raw_attribute_value(attributes, id)?;
    let celsius = (raw & 0xff) as i32;
    if (1..=255).contains(&celsius) {
      Some(celsius)
    } else {
      None
    }
  })
}

fn raw_attribute_value(attributes: &[SmartAttribute], id: u32) -> Option<u64> {
  attributes
    .iter()
    .find(|attr| attr.id == Some(id))
    .and_then(|attr| attr.raw_value.as_deref())
    .and_then(|raw| raw.parse::<u64>().ok())
}

fn find_disk_drive<'a>(
  instance_name: &str,
  disk_drives: &'a [Win32DiskDrive],
) -> Option<&'a Win32DiskDrive> {
  if disk_drives.len() == 1 {
    return disk_drives.first();
  }

  let normalized_instance = normalize_identifier(instance_name);

  disk_drives.iter().find(|disk| {
    disk
      .pnp_device_id
      .as_deref()
      .map(normalize_identifier)
      .filter(|id| !id.is_empty())
      .map(|id| normalized_instance.contains(&id) || id.contains(&normalized_instance))
      .unwrap_or(false)
      || disk
        .serial_number
        .as_deref()
        .map(normalize_identifier)
        .filter(|serial| !serial.is_empty())
        .map(|serial| normalized_instance.contains(&serial))
        .unwrap_or(false)
  })
}

fn normalize_identifier(value: &str) -> String {
  value
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .flat_map(char::to_uppercase)
    .collect()
}

fn trimmed(value: Option<&str>) -> Option<String> {
  value
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToOwned::to_owned)
}

fn wmi_query_in_namespace<T>(
  namespace: &'static str,
  query: &'static str,
) -> Result<Vec<T>, String>
where
  T: DeserializeOwned + std::fmt::Debug + Send + 'static,
{
  type ResultChannel<T> = Result<Vec<T>, String>;
  type SenderChannel<T> = Sender<ResultChannel<T>>;
  type ReceiverChannel<T> = Receiver<ResultChannel<T>>;

  let (tx, rx): (SenderChannel<T>, ReceiverChannel<T>) = channel();

  thread::spawn(move || {
    let result = (|| {
      let wmi_con = WMIConnection::with_namespace_path(namespace)
        .map_err(|e| format!("Failed to create WMI connection for {namespace}: {e:?}"))?;

      wmi_con
        .raw_query(query)
        .map_err(|e| format!("Failed to execute WMI query in {namespace}: {e:?}"))
    })();

    let _ = tx.send(result);
  });

  rx.recv()
    .map_err(|_| "Failed to receive WMI query result".to_string())?
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_vendor_specific_attributes() {
    let mut data = vec![0_u8; 362];
    let offset = 2;
    data[offset] = 9;
    data[offset + 3] = 99;
    data[offset + 4] = 98;
    data[offset + 5] = 0xd2;
    data[offset + 6] = 0x04;

    let mut thresholds = vec![0_u8; 362];
    thresholds[offset] = 9;
    thresholds[offset + 1] = 10;

    let attributes = parse_vendor_specific_attributes(&data, Some(&thresholds));

    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0].id, Some(9));
    assert_eq!(attributes[0].name, "Power-On Hours");
    assert_eq!(attributes[0].current, Some(99));
    assert_eq!(attributes[0].worst, Some(98));
    assert_eq!(attributes[0].threshold, Some(10));
    assert_eq!(attributes[0].raw_value.as_deref(), Some("1234"));
  }

  #[test]
  fn extracts_temperature_low_byte() {
    let attributes = vec![SmartAttribute {
      id: Some(194),
      name: "Temperature Celsius".to_string(),
      current: Some(70),
      worst: Some(60),
      threshold: Some(0),
      raw_value: Some("33".to_string()),
      when_failed: None,
    }];

    assert_eq!(temperature_from_attributes(&attributes), Some(33));
  }

  #[test]
  fn storage_wmi_disk_maps_physical_disk_and_reliability_counter() {
    let disk = MsftPhysicalDisk {
      device_id: Some("0".to_string()),
      friendly_name: Some("Example NVMe".to_string()),
      serial_number: Some(" SERIAL123 ".to_string()),
      firmware_version: Some("1.2.3".to_string()),
      size: Some(1_000_000_000),
      bus_type: Some(17),
      health_status: Some(1),
      media_type: Some(4),
    };
    let counter = MsftStorageReliabilityCounter {
      device_id: Some("0".to_string()),
      temperature: Some(42),
      temperature_max: Some(80),
      read_errors_total: Some(10),
      read_errors_corrected: Some(9),
      read_errors_uncorrected: Some(1),
      write_errors_total: Some(0),
      write_errors_corrected: Some(0),
      write_errors_uncorrected: Some(2),
      start_stop_cycle_count: Some(3),
      load_unload_cycle_count: Some(4),
      wear: Some(12),
      power_on_hours: Some(345),
      read_latency_max: Some(6),
      write_latency_max: Some(7),
      flush_latency_max: Some(8),
    };

    let smart = storage_wmi_disk_to_smart_info(&disk, Some(&counter));

    assert_eq!(smart.device_name, "0");
    assert_eq!(smart.device_type.as_deref(), Some("SSD"));
    assert_eq!(smart.protocol.as_deref(), Some("NVMe"));
    assert_eq!(smart.model_name.as_deref(), Some("Example NVMe"));
    assert_eq!(smart.serial_number.as_deref(), Some("SERIAL123"));
    assert_eq!(smart.health_status, SmartHealthStatus::Warning);
    assert_eq!(smart.temperature_celsius, Some(42));
    assert_eq!(smart.power_on_hours, Some(345));
    assert_eq!(raw_attribute_value(&smart.attributes, 9), Some(345));
    assert_eq!(
      smart
        .attributes
        .iter()
        .find(|attr| attr.name == "Percentage Used")
        .and_then(|attr| attr.raw_value.as_deref()),
      Some("12")
    );
    assert_eq!(
      smart
        .attributes
        .iter()
        .find(|attr| attr.name == "Media Errors")
        .and_then(|attr| attr.raw_value.as_deref()),
      Some("3")
    );
  }

  #[test]
  fn storage_health_status_maps_known_values() {
    assert_eq!(storage_health_status(Some(0)), SmartHealthStatus::Passed);
    assert_eq!(storage_health_status(Some(1)), SmartHealthStatus::Warning);
    assert_eq!(storage_health_status(Some(2)), SmartHealthStatus::Failed);
    assert_eq!(storage_health_status(Some(5)), SmartHealthStatus::Unknown);
    assert_eq!(storage_health_status(Some(99)), SmartHealthStatus::Unknown);
    assert_eq!(storage_health_status(None), SmartHealthStatus::Unknown);
  }
}
