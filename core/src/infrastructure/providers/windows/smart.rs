use crate::infrastructure::providers::smartctl;
use crate::log_warn;
use crate::models::external_component_guidance::{
  SIGNAL_AVAILABLE_SPARE, SIGNAL_CURRENT_PENDING_SECTORS, SIGNAL_MEDIA_ERRORS,
  SIGNAL_OFFLINE_UNCORRECTABLE_SECTORS, SIGNAL_PERCENTAGE_USED,
  SIGNAL_REALLOCATED_SECTORS, SIGNAL_SMART_OVERALL_HEALTH, SIGNAL_TEMPERATURE,
  all_storage_health_signal_names,
};
use crate::models::hardware::{SmartAttribute, SmartDiskInfo, SmartHealthStatus};
use crate::models::{ExternalComponentGuidanceCandidate, SmartInfoCollectionOutcome};
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
  health_status: Option<u16>,
  bus_type: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsftStorageReliabilityCounter {
  device_id: Option<String>,
  temperature: Option<i64>,
  wear: Option<u64>,
  read_errors_total: Option<u64>,
  write_errors_total: Option<u64>,
}

pub fn get_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  get_smart_info_with_guidance().disks
}

pub fn get_smart_info_with_guidance() -> SmartInfoCollectionOutcome {
  collect_with_fallbacks_with_guidance(
    super::device_io::get_smart_info,
    query_wmi_smart_info,
    smartctl::collect_smart_info_from_scan,
    query_storage_management_cim_info,
  )
}

#[cfg(test)]
fn collect_with_fallbacks<D, W, S, C>(
  mut device_io: D,
  mut wmi: W,
  mut smartctl: S,
  mut cim: C,
) -> Result<Vec<SmartDiskInfo>, String>
where
  D: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
  W: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
  S: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
  C: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
{
  collect_with_fallbacks_with_guidance(&mut device_io, &mut wmi, &mut smartctl, &mut cim)
    .disks
}

fn collect_with_fallbacks_with_guidance<D, W, S, C>(
  mut device_io: D,
  mut wmi: W,
  mut smartctl: S,
  mut cim: C,
) -> SmartInfoCollectionOutcome
where
  D: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
  W: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
  S: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
  C: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
{
  let mut failures = Vec::new();

  if let Some(disks) =
    collect_from_source("DeviceIoControl", &mut device_io, &mut failures)
  {
    return SmartInfoCollectionOutcome {
      disks: Ok(disks),
      guidance_candidates: Vec::new(),
    };
  }
  if let Some(disks) = collect_from_source("Windows WMI", &mut wmi, &mut failures) {
    return SmartInfoCollectionOutcome {
      disks: Ok(disks),
      guidance_candidates: Vec::new(),
    };
  }

  let smartctl_failure_index = failures.len();
  if let Some(disks) = collect_from_source("smartctl", &mut smartctl, &mut failures) {
    return SmartInfoCollectionOutcome {
      disks: Ok(disks),
      guidance_candidates: Vec::new(),
    };
  }
  let smartctl_failure = failures.get(smartctl_failure_index).cloned();

  if let Some(disks) =
    collect_from_source("Storage Management CIM", &mut cim, &mut failures)
  {
    let guidance_candidates = smartctl_failure
      .as_deref()
      .and_then(|detail| smartctl_guidance_for_disks(&disks, detail))
      .into_iter()
      .collect();
    return SmartInfoCollectionOutcome {
      disks: Ok(disks),
      guidance_candidates,
    };
  }

  let guidance_candidates = smartctl_failure
    .as_deref()
    .map(|detail| {
      ExternalComponentGuidanceCandidate::smartctl_storage_health(
        all_storage_health_signal_names(),
        None,
        detail.to_string(),
      )
    })
    .into_iter()
    .collect();

  SmartInfoCollectionOutcome {
    disks: Err(format!(
      "Failed to collect Windows storage health info: {}",
      failures.join("; ")
    )),
    guidance_candidates,
  }
}

fn collect_from_source<F>(
  label: &'static str,
  source: &mut F,
  failures: &mut Vec<String>,
) -> Option<Vec<SmartDiskInfo>>
where
  F: FnMut() -> Result<Vec<SmartDiskInfo>, String>,
{
  match source() {
    Ok(disks) if !disks.is_empty() => Some(disks),
    Ok(_) => {
      let detail = format!("{label} returned no SMART devices");
      log_warn!(
        "Windows Storage Health collection source returned no devices",
        "infrastructure::providers::windows::smart::collect_from_source",
        Some(detail.clone())
      );
      failures.push(detail);
      None
    }
    Err(e) => {
      let detail = format!("{label} failed: {e}");
      log_warn!(
        "Windows Storage Health collection source failed",
        "infrastructure::providers::windows::smart::collect_from_source",
        Some(detail.clone())
      );
      failures.push(detail);
      None
    }
  }
}

fn smartctl_guidance_for_disks(
  disks: &[SmartDiskInfo],
  diagnostic_detail: &str,
) -> Option<ExternalComponentGuidanceCandidate> {
  let mut missing_signals = Vec::new();
  let mut affected_device_count = 0_u32;

  for disk in disks {
    let disk_missing = missing_storage_health_signals(disk);
    if !disk_missing.is_empty() {
      affected_device_count += 1;
    }
    for signal in disk_missing {
      if !missing_signals.contains(&signal) {
        missing_signals.push(signal);
      }
    }
  }

  (!missing_signals.is_empty()).then(|| {
    ExternalComponentGuidanceCandidate::smartctl_storage_health(
      missing_signals,
      Some(affected_device_count),
      diagnostic_detail.to_string(),
    )
  })
}

fn missing_storage_health_signals(disk: &SmartDiskInfo) -> Vec<String> {
  let mut missing = Vec::new();

  if matches!(disk.health_status, SmartHealthStatus::Unknown) {
    missing.push(SIGNAL_SMART_OVERALL_HEALTH.to_string());
  }
  if disk.temperature_celsius.is_none() {
    missing.push(SIGNAL_TEMPERATURE.to_string());
  }

  match storage_protocol_kind(disk) {
    StorageProtocolKind::Nvme => {
      push_if_attr_missing(
        disk,
        None,
        "Percentage Used",
        SIGNAL_PERCENTAGE_USED,
        &mut missing,
      );
      push_if_attr_missing(
        disk,
        None,
        "Available Spare",
        SIGNAL_AVAILABLE_SPARE,
        &mut missing,
      );
      push_if_attr_missing(
        disk,
        None,
        "Media Errors",
        SIGNAL_MEDIA_ERRORS,
        &mut missing,
      );
    }
    StorageProtocolKind::Ata => {
      push_if_attr_missing(
        disk,
        Some(5),
        "Reallocated Sector Count",
        SIGNAL_REALLOCATED_SECTORS,
        &mut missing,
      );
      push_if_attr_missing(
        disk,
        Some(197),
        "Current Pending Sector Count",
        SIGNAL_CURRENT_PENDING_SECTORS,
        &mut missing,
      );
      push_if_attr_missing(
        disk,
        Some(198),
        "Offline Uncorrectable Sector Count",
        SIGNAL_OFFLINE_UNCORRECTABLE_SECTORS,
        &mut missing,
      );
    }
    StorageProtocolKind::Unknown => {}
  }

  missing
}

enum StorageProtocolKind {
  Ata,
  Nvme,
  Unknown,
}

fn storage_protocol_kind(disk: &SmartDiskInfo) -> StorageProtocolKind {
  let value = disk
    .protocol
    .as_deref()
    .or(disk.device_type.as_deref())
    .unwrap_or_default()
    .to_ascii_lowercase();

  if value.contains("nvme") {
    StorageProtocolKind::Nvme
  } else if value.contains("ata") || value.contains("sata") || value.contains("sat") {
    StorageProtocolKind::Ata
  } else {
    StorageProtocolKind::Unknown
  }
}

fn push_if_attr_missing(
  disk: &SmartDiskInfo,
  id: Option<u32>,
  name: &str,
  signal: &str,
  missing: &mut Vec<String>,
) {
  if !has_attr_value(disk, id, name) {
    missing.push(signal.to_string());
  }
}

fn has_attr_value(disk: &SmartDiskInfo, id: Option<u32>, name: &str) -> bool {
  disk.attributes.iter().any(|attr| {
    let matches = match id {
      Some(id) => attr.id == Some(id),
      None => attr.name.eq_ignore_ascii_case(name),
    };
    matches && (attr.raw_value.is_some() || attr.current.is_some())
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

fn query_storage_management_cim_info() -> Result<Vec<SmartDiskInfo>, String> {
  let physical_disks: Vec<MsftPhysicalDisk> = wmi_query_in_namespace(
    "ROOT\\Microsoft\\Windows\\Storage",
    "SELECT * FROM MSFT_PhysicalDisk",
  )?;
  // MSFT_StorageReliabilityCounter often returns no rows without elevation;
  // continue with identity and health data only in that case.
  let reliability_counters: Vec<MsftStorageReliabilityCounter> = wmi_query_in_namespace(
    "ROOT\\Microsoft\\Windows\\Storage",
    "SELECT * FROM MSFT_StorageReliabilityCounter",
  )
  .unwrap_or_else(|e| {
    log_warn!(
      "Failed to query MSFT_StorageReliabilityCounter; continuing without reliability data",
      "infrastructure::providers::windows::smart::query_storage_management_cim_info",
      Some(e)
    );
    Vec::new()
  });
  let counters_by_device_id: HashMap<String, MsftStorageReliabilityCounter> =
    reliability_counters
      .into_iter()
      .filter_map(|counter| {
        let device_id = trimmed(counter.device_id.as_deref())?;
        Some((device_id, counter))
      })
      .collect();

  Ok(
    physical_disks
      .into_iter()
      .enumerate()
      .map(|(index, disk)| {
        let device_id = trimmed(disk.device_id.as_deref());
        let reliability = device_id
          .as_deref()
          .and_then(|id| counters_by_device_id.get(id));
        let temperature_celsius = reliability
          .and_then(|counter| counter.temperature)
          .and_then(|value| i32::try_from(value).ok());
        let wear = reliability.and_then(|counter| counter.wear);
        let media_errors = reliability.and_then(|counter| {
          match (counter.read_errors_total, counter.write_errors_total) {
            (Some(read), Some(write)) => read.checked_add(write),
            (Some(read), None) => Some(read),
            (None, Some(write)) => Some(write),
            (None, None) => None,
          }
        });
        let protocol = disk.bus_type.map(cim_bus_type_name);

        SmartDiskInfo {
          device_name: trimmed(disk.friendly_name.as_deref())
            .or_else(|| device_id.clone())
            .unwrap_or_else(|| format!("MSFT_PhysicalDisk#{index}")),
          device_type: protocol.map(str::to_ascii_lowercase),
          protocol: protocol.map(ToOwned::to_owned),
          model_name: trimmed(disk.friendly_name.as_deref()),
          serial_number: trimmed(disk.serial_number.as_deref()),
          firmware_version: trimmed(disk.firmware_version.as_deref()),
          capacity_bytes: disk.size,
          health_status: cim_health_status(disk.health_status),
          temperature_celsius,
          power_on_hours: None,
          power_cycle_count: None,
          attributes: [
            wear.map(|value| cim_attr("Percentage Used", value)),
            media_errors.map(|value| cim_attr("Media Errors", value)),
          ]
          .into_iter()
          .flatten()
          .collect(),
        }
      })
      .collect(),
  )
}

fn cim_attr(name: &str, value: u64) -> SmartAttribute {
  SmartAttribute {
    id: None,
    name: name.to_string(),
    current: Some(value),
    worst: None,
    threshold: None,
    raw_value: Some(value.to_string()),
    when_failed: None,
  }
}

fn cim_health_status(value: Option<u16>) -> SmartHealthStatus {
  // MSFT_PhysicalDisk HealthStatus: 0 = Healthy, 1 = Warning, 2 = Unhealthy.
  match value {
    Some(0) => SmartHealthStatus::Passed,
    Some(1 | 2) => SmartHealthStatus::Failed,
    _ => SmartHealthStatus::Unknown,
  }
}

fn cim_bus_type_name(value: u16) -> &'static str {
  match value {
    1 => "SCSI",
    2 => "ATAPI",
    3 => "ATA",
    4 => "IEEE1394",
    5 => "SSA",
    6 => "Fibre",
    7 => "USB",
    8 => "RAID",
    9 => "iSCSI",
    10 => "SAS",
    11 => "SATA",
    12 => "SD",
    13 => "MMC",
    14 => "Virtual",
    15 => "FileBackedVirtual",
    16 => "StorageSpaces",
    17 => "NVMe",
    18 => "SCM",
    19 => "UFS",
    _ => "Unknown",
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
  use std::cell::RefCell;

  fn smart_disk(device_name: &str) -> SmartDiskInfo {
    SmartDiskInfo {
      device_name: device_name.to_string(),
      device_type: None,
      protocol: None,
      model_name: None,
      serial_number: None,
      firmware_version: None,
      capacity_bytes: None,
      health_status: SmartHealthStatus::Passed,
      temperature_celsius: None,
      power_on_hours: None,
      power_cycle_count: None,
      attributes: Vec::new(),
    }
  }

  #[test]
  fn device_io_success_prevents_later_windows_storage_health_fallbacks() {
    let calls = RefCell::new(Vec::new());

    let disks = collect_with_fallbacks(
      || {
        calls.borrow_mut().push("device_io");
        Ok(vec![smart_disk("device-io")])
      },
      || {
        calls.borrow_mut().push("wmi");
        Ok(vec![smart_disk("wmi")])
      },
      || {
        calls.borrow_mut().push("smartctl");
        Ok(vec![smart_disk("smartctl")])
      },
      || {
        calls.borrow_mut().push("cim");
        Ok(vec![smart_disk("cim")])
      },
    )
    .expect("DeviceIoControl source should win");

    assert_eq!(calls.into_inner(), vec!["device_io"]);
    assert_eq!(disks[0].device_name, "device-io");
  }

  #[test]
  fn storage_management_cim_is_used_after_device_io_wmi_and_smartctl_fail() {
    let calls = RefCell::new(Vec::new());

    let disks = collect_with_fallbacks(
      || {
        calls.borrow_mut().push("device_io");
        Err("unsupported".to_string())
      },
      || {
        calls.borrow_mut().push("wmi");
        Ok(Vec::new())
      },
      || {
        calls.borrow_mut().push("smartctl");
        Err("smartctl is not installed".to_string())
      },
      || {
        calls.borrow_mut().push("cim");
        Ok(vec![smart_disk("cim")])
      },
    )
    .expect("CIM should be the final fallback");

    assert_eq!(
      calls.into_inner(),
      vec!["device_io", "wmi", "smartctl", "cim"]
    );
    assert_eq!(disks[0].device_name, "cim");
  }

  #[test]
  fn guidance_candidate_is_returned_when_smartctl_fails_and_cim_lacks_health_signals() {
    let outcome = collect_with_fallbacks_with_guidance(
      || Err("unsupported".to_string()),
      || Ok(Vec::new()),
      || Err("smartctl is not installed".to_string()),
      || Ok(vec![smart_disk("cim")]),
    );

    let disks = outcome.disks.expect("CIM should still be used as fallback");

    assert_eq!(disks[0].device_name, "cim");
    assert_eq!(outcome.guidance_candidates.len(), 1);
    assert_eq!(
      outcome.guidance_candidates[0].key,
      "smartctl:storage-health:v1"
    );
    assert_eq!(
      outcome.guidance_candidates[0].affected_device_count,
      Some(1)
    );
    assert!(
      outcome.guidance_candidates[0]
        .missing_signals
        .contains(&"temperature".to_string())
    );
  }

  #[test]
  fn guidance_candidate_is_not_returned_when_cim_covers_health_signals() {
    let mut disk = smart_disk("cim");
    disk.protocol = Some("NVMe".to_string());
    disk.temperature_celsius = Some(40);
    disk.attributes = vec![
      cim_attr("Percentage Used", 3),
      cim_attr("Available Spare", 98),
      cim_attr("Media Errors", 0),
    ];

    let outcome = collect_with_fallbacks_with_guidance(
      || Err("unsupported".to_string()),
      || Ok(Vec::new()),
      || Err("Failed to execute smartctl: not found".to_string()),
      || Ok(vec![disk.clone()]),
    );

    assert!(outcome.disks.is_ok());
    assert!(outcome.guidance_candidates.is_empty());
  }

  #[test]
  fn cim_health_status_maps_msft_physical_disk_values() {
    // MSFT_PhysicalDisk HealthStatus: 0 = Healthy, 1 = Warning, 2 = Unhealthy.
    assert_eq!(cim_health_status(Some(0)), SmartHealthStatus::Passed);
    assert_eq!(cim_health_status(Some(1)), SmartHealthStatus::Failed);
    assert_eq!(cim_health_status(Some(2)), SmartHealthStatus::Failed);
    assert_eq!(cim_health_status(Some(5)), SmartHealthStatus::Unknown);
    assert_eq!(cim_health_status(None), SmartHealthStatus::Unknown);
  }

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
