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

pub fn get_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  match query_wmi_smart_info() {
    Ok(disks) if !disks.is_empty() => Ok(disks),
    Ok(_) => smartctl::collect_smart_info_from_scan()
      .map_err(|smartctl_error| {
        format!("Windows WMI returned no SMART devices; smartctl fallback failed: {smartctl_error}")
      }),
    Err(wmi_error) => smartctl::collect_smart_info_from_scan().map_err(|smartctl_error| {
      format!(
        "Failed to collect SMART info from Windows WMI ({wmi_error}) and smartctl ({smartctl_error})"
      )
    }),
  }
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
}
