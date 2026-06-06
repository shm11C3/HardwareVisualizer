use crate::log_warn;
use crate::models::hardware::{SmartAttribute, SmartDiskInfo, SmartHealthStatus};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
  CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
  IOCTL_STORAGE_QUERY_PROPERTY, NVMeDataTypeLogPage, PropertyStandardQuery,
  ProtocolTypeNvme, STORAGE_PROPERTY_QUERY, STORAGE_PROTOCOL_DATA_DESCRIPTOR,
  STORAGE_PROTOCOL_SPECIFIC_DATA, StorageDeviceProtocolSpecificProperty,
};
use windows::core::PCWSTR;

const MAX_PHYSICAL_DRIVES: usize = 32;
const NVME_HEALTH_LOG_MIN_LEN: usize = 192;
const NVME_HEALTH_LOG_LEN: usize = 512;
const NVME_HEALTH_INFO_LOG_PAGE: u32 = 0x02;
const STORAGE_PROPERTY_QUERY_ADDITIONAL_PARAMETERS_OFFSET: usize = 8;
const PROTOCOL_SPECIFIC_DATA_OFFSET: usize =
  std::mem::offset_of!(STORAGE_PROTOCOL_DATA_DESCRIPTOR, ProtocolSpecificData);
const PROTOCOL_DATA_OFFSET_FIELD: usize = PROTOCOL_SPECIFIC_DATA_OFFSET
  + std::mem::offset_of!(STORAGE_PROTOCOL_SPECIFIC_DATA, ProtocolDataOffset);
const PROTOCOL_DATA_LENGTH_FIELD: usize = PROTOCOL_SPECIFIC_DATA_OFFSET
  + std::mem::offset_of!(STORAGE_PROTOCOL_SPECIFIC_DATA, ProtocolDataLength);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32DiskDriveForDeviceIo {
  #[serde(rename = "DeviceID")]
  device_id: Option<String>,
  model: Option<String>,
  serial_number: Option<String>,
  firmware_revision: Option<String>,
  size: Option<u64>,
  interface_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsStorageDevice {
  device_path: String,
  model: Option<String>,
  serial_number: Option<String>,
  firmware_version: Option<String>,
  capacity_bytes: Option<u64>,
  interface_type: Option<String>,
}

pub fn get_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  let devices = enumerate_storage_devices();
  let reader = DeviceIoControlNvmeReader;
  collect_from_devices(&devices, &reader)
}

fn enumerate_storage_devices() -> Vec<WindowsStorageDevice> {
  match query_win32_disk_drives() {
    Ok(disks) if !disks.is_empty() => {
      let mut devices = disks
        .into_iter()
        .filter_map(WindowsStorageDevice::from_win32_disk_drive)
        .collect::<Vec<_>>();
      if devices.is_empty() {
        return fallback_physical_drive_candidates();
      }
      devices.sort_by_key(|device| {
        !device
          .interface_type
          .as_deref()
          .map(|value| value.eq_ignore_ascii_case("nvme"))
          .unwrap_or(false)
      });
      devices
    }
    Ok(_) => fallback_physical_drive_candidates(),
    Err(e) => {
      log_warn!(
        "Failed to enumerate disks before Windows DeviceIoControl collection",
        "infrastructure::providers::windows::device_io::enumerate_storage_devices",
        Some(e)
      );
      fallback_physical_drive_candidates()
    }
  }
}

impl WindowsStorageDevice {
  fn from_win32_disk_drive(disk: Win32DiskDriveForDeviceIo) -> Option<Self> {
    let device_path = trimmed(disk.device_id.as_deref())?;
    Some(Self {
      device_path,
      model: trimmed(disk.model.as_deref()),
      serial_number: trimmed(disk.serial_number.as_deref()),
      firmware_version: trimmed(disk.firmware_revision.as_deref()),
      capacity_bytes: disk.size,
      interface_type: trimmed(disk.interface_type.as_deref()),
    })
  }
}

fn fallback_physical_drive_candidates() -> Vec<WindowsStorageDevice> {
  (0..MAX_PHYSICAL_DRIVES)
    .map(|index| WindowsStorageDevice {
      device_path: format!(r"\\.\PhysicalDrive{index}"),
      model: None,
      serial_number: None,
      firmware_version: None,
      capacity_bytes: None,
      interface_type: None,
    })
    .collect()
}

fn nvme_health_log_to_smart_disk_info(
  device: &WindowsStorageDevice,
  raw: &[u8],
) -> Result<SmartDiskInfo, String> {
  if raw.len() < NVME_HEALTH_LOG_MIN_LEN {
    return Err(format!(
      "NVMe health log is too short: expected at least {NVME_HEALTH_LOG_MIN_LEN} bytes, got {}",
      raw.len()
    ));
  }

  let critical_warning = raw[0] as u64;
  let temperature_celsius = read_u16(raw, 1)
    .filter(|kelvin| *kelvin > 0)
    .and_then(|kelvin| i32::from(kelvin).checked_sub(273));
  let available_spare = raw[3] as u64;
  let available_spare_threshold = raw[4] as u64;
  let percentage_used = raw[5] as u64;
  let power_cycles = read_u128_as_u64(raw, 112);
  let power_on_hours = read_u128_as_u64(raw, 128);
  let unsafe_shutdowns = read_u128_as_u64(raw, 144);
  let media_errors = read_u128_as_u64(raw, 160);
  let error_log_entries = read_u128_as_u64(raw, 176);

  Ok(SmartDiskInfo {
    device_name: device.device_path.clone(),
    device_type: Some("nvme".to_string()),
    protocol: Some("NVMe".to_string()),
    model_name: device.model.clone(),
    serial_number: device.serial_number.clone(),
    firmware_version: device.firmware_version.clone(),
    capacity_bytes: device.capacity_bytes,
    health_status: if critical_warning == 0 {
      SmartHealthStatus::Passed
    } else {
      SmartHealthStatus::Failed
    },
    temperature_celsius,
    power_on_hours,
    power_cycle_count: power_cycles,
    attributes: vec![
      nvme_attr("Critical Warning", critical_warning),
      nvme_attr(
        "Temperature",
        temperature_celsius.unwrap_or_default() as u64,
      ),
      nvme_attr("Available Spare", available_spare),
      nvme_attr("Available Spare Threshold", available_spare_threshold),
      nvme_attr("Percentage Used", percentage_used),
      nvme_attr_opt("Power Cycles", power_cycles),
      nvme_attr_opt("Power-On Hours", power_on_hours),
      nvme_attr_opt("Unsafe Shutdowns", unsafe_shutdowns),
      nvme_attr_opt("Media Errors", media_errors),
      nvme_attr_opt("Error Information Log Entries", error_log_entries),
    ],
  })
}

trait NvmeHealthLogReader {
  fn read_nvme_health_log(&self, device_path: &str) -> Result<Vec<u8>, String>;
}

struct DeviceIoControlNvmeReader;

impl NvmeHealthLogReader for DeviceIoControlNvmeReader {
  fn read_nvme_health_log(&self, device_path: &str) -> Result<Vec<u8>, String> {
    let handle = StorageHandle::open(device_path)?;
    query_nvme_health_log(handle.raw())
  }
}

fn collect_from_devices<R: NvmeHealthLogReader>(
  devices: &[WindowsStorageDevice],
  reader: &R,
) -> Result<Vec<SmartDiskInfo>, String> {
  let mut disks = Vec::new();
  let mut errors = Vec::new();

  for device in devices {
    match reader
      .read_nvme_health_log(&device.device_path)
      .and_then(|raw| nvme_health_log_to_smart_disk_info(device, &raw))
    {
      Ok(disk) => disks.push(disk),
      Err(e) => errors.push(format!("{}: {e}", device.device_path)),
    }
  }

  if disks.is_empty() {
    let detail = if errors.is_empty() {
      "no devices found".to_string()
    } else {
      errors.join("; ")
    };
    Err(format!(
      "Failed to collect Windows DeviceIoControl storage health info: {detail}"
    ))
  } else {
    if !errors.is_empty() {
      log_warn!(
        "Some Windows DeviceIoControl storage devices failed to collect",
        "infrastructure::providers::windows::device_io::collect_from_devices",
        Some(errors.join("; "))
      );
    }
    Ok(disks)
  }
}

struct StorageHandle(HANDLE);

impl StorageHandle {
  fn open(device_path: &str) -> Result<Self, String> {
    let wide_path = wide_null(device_path);
    let handle = unsafe {
      CreateFileW(
        PCWSTR(wide_path.as_ptr()),
        0,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        None,
      )
    }
    .map_err(|e| format!("Failed to open {device_path}: {e:?}"))?;

    if handle.is_invalid() {
      Err(format!("Failed to open {device_path}: invalid handle"))
    } else {
      Ok(Self(handle))
    }
  }

  fn raw(&self) -> HANDLE {
    self.0
  }
}

impl Drop for StorageHandle {
  fn drop(&mut self) {
    let _ = unsafe { CloseHandle(self.0) };
  }
}

fn query_nvme_health_log(handle: HANDLE) -> Result<Vec<u8>, String> {
  let protocol_data_offset = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();
  let buffer_len = STORAGE_PROPERTY_QUERY_ADDITIONAL_PARAMETERS_OFFSET
    + protocol_data_offset
    + NVME_HEALTH_LOG_LEN;
  let mut buffer = vec![0_u8; buffer_len];

  let query = buffer.as_mut_ptr().cast::<STORAGE_PROPERTY_QUERY>();
  unsafe {
    (*query).PropertyId = StorageDeviceProtocolSpecificProperty;
    (*query).QueryType = PropertyStandardQuery;
  }

  let protocol = unsafe {
    buffer
      .as_mut_ptr()
      .add(STORAGE_PROPERTY_QUERY_ADDITIONAL_PARAMETERS_OFFSET)
      .cast::<STORAGE_PROTOCOL_SPECIFIC_DATA>()
  };
  unsafe {
    *protocol = STORAGE_PROTOCOL_SPECIFIC_DATA {
      ProtocolType: ProtocolTypeNvme,
      DataType: NVMeDataTypeLogPage.0 as u32,
      ProtocolDataRequestValue: NVME_HEALTH_INFO_LOG_PAGE,
      ProtocolDataRequestSubValue: 0,
      ProtocolDataOffset: protocol_data_offset as u32,
      ProtocolDataLength: NVME_HEALTH_LOG_LEN as u32,
      FixedProtocolReturnData: 0,
      ProtocolDataRequestSubValue2: 0,
      ProtocolDataRequestSubValue3: 0,
      ProtocolDataRequestSubValue4: 0,
    };
  }

  let mut bytes_returned = 0_u32;
  let input_ptr = buffer.as_ptr().cast::<c_void>();
  let output_ptr = buffer.as_mut_ptr().cast::<c_void>();
  unsafe {
    DeviceIoControl(
      handle,
      IOCTL_STORAGE_QUERY_PROPERTY,
      Some(input_ptr),
      buffer_len as u32,
      Some(output_ptr),
      buffer_len as u32,
      Some(&mut bytes_returned),
      None,
    )
  }
  .map_err(|e| format!("DeviceIoControl NVMe health log query failed: {e:?}"))?;

  extract_nvme_health_log(&buffer, bytes_returned)
}

fn extract_nvme_health_log(
  buffer: &[u8],
  bytes_returned: u32,
) -> Result<Vec<u8>, String> {
  let descriptor_len = size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>();
  if (bytes_returned as usize) < descriptor_len {
    return Err(format!(
      "DeviceIoControl NVMe health log query returned too few bytes: {bytes_returned}"
    ));
  }

  // Per the IOCTL_STORAGE_QUERY_PROPERTY protocol query contract, the storage
  // stack reports both Version and Size as sizeof(STORAGE_PROTOCOL_DATA_DESCRIPTOR).
  let version = read_u32(buffer, 0).unwrap_or_default();
  let size = read_u32(buffer, 4).unwrap_or_default();
  if version as usize != descriptor_len || size as usize != descriptor_len {
    return Err(format!(
      "DeviceIoControl NVMe health log descriptor is invalid: version {version}, size {size}, expected {descriptor_len}"
    ));
  }

  let protocol_data_offset =
    read_u32(buffer, PROTOCOL_DATA_OFFSET_FIELD).unwrap_or_default() as usize;
  let protocol_data_len =
    read_u32(buffer, PROTOCOL_DATA_LENGTH_FIELD).unwrap_or_default() as usize;
  if protocol_data_offset < size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>() {
    return Err(format!(
      "DeviceIoControl NVMe health log descriptor has invalid data offset {protocol_data_offset}"
    ));
  }

  // ProtocolDataOffset is relative to the start of STORAGE_PROTOCOL_SPECIFIC_DATA.
  let data_offset = PROTOCOL_SPECIFIC_DATA_OFFSET + protocol_data_offset;
  let data_len = protocol_data_len.min(NVME_HEALTH_LOG_LEN);
  let data = buffer
    .get(data_offset..data_offset + data_len)
    .ok_or_else(|| {
      format!(
        "DeviceIoControl NVMe health log data is out of range: offset {data_offset}, length {data_len}"
      )
    })?;

  Ok(data.to_vec())
}

fn nvme_attr(name: &str, value: u64) -> SmartAttribute {
  nvme_attr_opt(name, Some(value))
}

fn nvme_attr_opt(name: &str, value: Option<u64>) -> SmartAttribute {
  SmartAttribute {
    id: None,
    name: name.to_string(),
    current: value,
    worst: None,
    threshold: None,
    raw_value: value.map(|v| v.to_string()),
    when_failed: None,
  }
}

fn read_u16(raw: &[u8], offset: usize) -> Option<u16> {
  let bytes: [u8; 2] = raw.get(offset..offset + 2)?.try_into().ok()?;
  Some(u16::from_le_bytes(bytes))
}

fn read_u32(raw: &[u8], offset: usize) -> Option<u32> {
  let bytes: [u8; 4] = raw.get(offset..offset + 4)?.try_into().ok()?;
  Some(u32::from_le_bytes(bytes))
}

fn read_u128_as_u64(raw: &[u8], offset: usize) -> Option<u64> {
  let bytes: [u8; 16] = raw.get(offset..offset + 16)?.try_into().ok()?;
  u64::try_from(u128::from_le_bytes(bytes)).ok()
}

fn query_win32_disk_drives() -> Result<Vec<Win32DiskDriveForDeviceIo>, String> {
  wmi_query_in_namespace(
    "ROOT\\CIMV2",
    "SELECT DeviceID, Model, SerialNumber, FirmwareRevision, Size, InterfaceType FROM Win32_DiskDrive",
  )
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
      let wmi_con = wmi::WMIConnection::with_namespace_path(namespace)
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

fn trimmed(value: Option<&str>) -> Option<String> {
  value
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToOwned::to_owned)
}

fn wide_null(value: &str) -> Vec<u16> {
  value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  struct FakeReader {
    logs: HashMap<String, Result<Vec<u8>, String>>,
  }

  impl NvmeHealthLogReader for FakeReader {
    fn read_nvme_health_log(&self, device_path: &str) -> Result<Vec<u8>, String> {
      self
        .logs
        .get(device_path)
        .cloned()
        .unwrap_or_else(|| Err("missing fake log".to_string()))
    }
  }

  fn descriptor_buffer(
    version: u32,
    size: u32,
    data_offset: u32,
    data_len: u32,
    payload: &[u8],
  ) -> Vec<u8> {
    let mut buffer = vec![
      0_u8;
      PROTOCOL_SPECIFIC_DATA_OFFSET
        + (data_offset as usize)
          .max(size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>())
        + payload.len()
    ];
    buffer[0..4].copy_from_slice(&version.to_le_bytes());
    buffer[4..8].copy_from_slice(&size.to_le_bytes());
    buffer[PROTOCOL_DATA_OFFSET_FIELD..PROTOCOL_DATA_OFFSET_FIELD + 4]
      .copy_from_slice(&data_offset.to_le_bytes());
    buffer[PROTOCOL_DATA_LENGTH_FIELD..PROTOCOL_DATA_LENGTH_FIELD + 4]
      .copy_from_slice(&data_len.to_le_bytes());
    let start = PROTOCOL_SPECIFIC_DATA_OFFSET + data_offset as usize;
    buffer[start..start + payload.len()].copy_from_slice(payload);
    buffer
  }

  #[test]
  fn extracts_health_log_from_descriptor_with_driver_reported_version() {
    // Real Windows drivers report Version and Size as
    // sizeof(STORAGE_PROTOCOL_DATA_DESCRIPTOR), not STORAGE_PROTOCOL_STRUCTURE_VERSION.
    let payload = vec![0xAB_u8; NVME_HEALTH_LOG_LEN];
    let descriptor_len = size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>() as u32;
    let buffer = descriptor_buffer(
      descriptor_len,
      descriptor_len,
      size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>() as u32,
      NVME_HEALTH_LOG_LEN as u32,
      &payload,
    );

    let log = extract_nvme_health_log(&buffer, buffer.len() as u32)
      .expect("driver-reported descriptor version should be accepted");

    assert_eq!(log, payload);
  }

  #[test]
  fn rejects_descriptor_with_unexpected_version() {
    let payload = vec![0_u8; NVME_HEALTH_LOG_LEN];
    let buffer = descriptor_buffer(
      1,
      1,
      size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>() as u32,
      NVME_HEALTH_LOG_LEN as u32,
      &payload,
    );

    let error = extract_nvme_health_log(&buffer, buffer.len() as u32)
      .expect_err("unexpected descriptor version should be rejected");

    assert!(error.contains("descriptor"));
  }

  #[test]
  fn parses_nvme_health_log_into_existing_smart_fields() {
    let device = WindowsStorageDevice {
      device_path: r"\\.\PhysicalDrive0".to_string(),
      model: Some("Example NVMe".to_string()),
      serial_number: Some("SERIAL123".to_string()),
      firmware_version: Some("1.0".to_string()),
      capacity_bytes: Some(1_000_204_886_016),
      interface_type: Some("NVMe".to_string()),
    };
    let mut raw = vec![0_u8; 512];
    raw[1..3].copy_from_slice(&315_u16.to_le_bytes());
    raw[3] = 97;
    raw[4] = 10;
    raw[5] = 8;
    raw[112..128].copy_from_slice(&55_u128.to_le_bytes());
    raw[128..144].copy_from_slice(&3_200_u128.to_le_bytes());
    raw[144..160].copy_from_slice(&2_u128.to_le_bytes());
    raw[160..176].copy_from_slice(&1_u128.to_le_bytes());
    raw[176..192].copy_from_slice(&7_u128.to_le_bytes());

    let disk = nvme_health_log_to_smart_disk_info(&device, &raw)
      .expect("NVMe health log should parse");

    assert_eq!(disk.device_name, r"\\.\PhysicalDrive0");
    assert_eq!(disk.device_type.as_deref(), Some("nvme"));
    assert_eq!(disk.protocol.as_deref(), Some("NVMe"));
    assert_eq!(disk.model_name.as_deref(), Some("Example NVMe"));
    assert_eq!(disk.serial_number.as_deref(), Some("SERIAL123"));
    assert_eq!(disk.firmware_version.as_deref(), Some("1.0"));
    assert_eq!(disk.capacity_bytes, Some(1_000_204_886_016));
    assert_eq!(disk.health_status, SmartHealthStatus::Passed);
    assert_eq!(disk.temperature_celsius, Some(42));
    assert_eq!(disk.power_on_hours, Some(3_200));
    assert!(disk.attributes.iter().any(
      |attr| attr.name == "Available Spare" && attr.raw_value.as_deref() == Some("97")
    ));
    assert!(disk.attributes.iter().any(
      |attr| attr.name == "Percentage Used" && attr.raw_value.as_deref() == Some("8")
    ));
    assert!(
      disk
        .attributes
        .iter()
        .any(|attr| attr.name == "Media Errors" && attr.raw_value.as_deref() == Some("1"))
    );
    assert!(disk.attributes.iter().any(
      |attr| attr.name == "Unsafe Shutdowns" && attr.raw_value.as_deref() == Some("2")
    ));
  }

  #[test]
  fn returns_successful_devices_when_one_device_io_read_fails() {
    let mut raw = vec![0_u8; 512];
    raw[1..3].copy_from_slice(&300_u16.to_le_bytes());

    let devices = vec![
      WindowsStorageDevice {
        device_path: r"\\.\PhysicalDrive0".to_string(),
        model: Some("Healthy NVMe".to_string()),
        serial_number: Some("GOOD".to_string()),
        firmware_version: None,
        capacity_bytes: None,
        interface_type: Some("NVMe".to_string()),
      },
      WindowsStorageDevice {
        device_path: r"\\.\PhysicalDrive1".to_string(),
        model: Some("Blocked NVMe".to_string()),
        serial_number: Some("BAD".to_string()),
        firmware_version: None,
        capacity_bytes: None,
        interface_type: Some("NVMe".to_string()),
      },
    ];
    let reader = FakeReader {
      logs: HashMap::from([
        (r"\\.\PhysicalDrive0".to_string(), Ok(raw)),
        (
          r"\\.\PhysicalDrive1".to_string(),
          Err("access denied".to_string()),
        ),
      ]),
    };

    let disks = collect_from_devices(&devices, &reader)
      .expect("partial success should be returned");

    assert_eq!(disks.len(), 1);
    assert_eq!(disks[0].device_name, r"\\.\PhysicalDrive0");
    assert_eq!(disks[0].model_name.as_deref(), Some("Healthy NVMe"));
  }
}
