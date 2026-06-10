//! Live Storage Health on-demand collection (ADR 0006).
//!
//! Current storage device health signals collected for immediate display
//! and never persisted, distinct from the daily Storage Health Record
//! worker in `crate::persistence::storage_health`.
//!
//! - Device enumeration (WMI) runs at startup via
//!   [`LiveStorageHealthCollector::enumerate_devices`] and caches the
//!   device list; the live read path never enumerates.
//! - A live read performs one `DeviceIoControl` NVMe health log query per
//!   cached device with no fallback chain. Devices that fail to read are
//!   dropped from the read set until the next enumeration.
//! - A minimum-interval guard returns the last reading when callers poll
//!   within [`LIVE_READ_MIN_INTERVAL`], deduplicating concurrent polls.
//! - Platforms without a cheap native read path keep an empty device
//!   cache and return an empty result; displays fall back to the daily
//!   Storage Health Record.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::log_warn;
use crate::models::hardware::{LiveStorageHealth, SmartDiskInfo};
use crate::persistence::storage_health::{
  attr_value_f32, attr_value_u64, storage_device_id,
};
use crate::settings::STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES;

#[cfg(target_os = "windows")]
use crate::infrastructure::providers::windows::device_io::{self, NvmeHealthLogReader};

/// Minimum interval between live collection passes. Calls inside the
/// window return the cached reading instead of issuing device queries.
pub const LIVE_READ_MIN_INTERVAL: Duration = Duration::from_secs(5);

/// Platform-neutral descriptor for one cached storage device. The
/// metadata fields feed the Windows reader's conversion into
/// [`SmartDiskInfo`]; they are kept on every platform so the cache and
/// tests stay uniform.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveStorageDevice {
  pub(crate) device_path: String,
  pub(crate) model: Option<String>,
  pub(crate) serial_number: Option<String>,
  pub(crate) firmware_version: Option<String>,
  pub(crate) capacity_bytes: Option<u64>,
}

/// Reads one cached device's current health signals. Implemented by the
/// DeviceIoControl-backed reader on Windows and by fakes in tests.
pub(crate) trait LiveSignalReader {
  fn read(&self, device: &LiveStorageDevice) -> Result<SmartDiskInfo, String>;
}

#[cfg(target_os = "windows")]
struct PlatformLiveSignalReader;

#[cfg(target_os = "windows")]
impl LiveSignalReader for PlatformLiveSignalReader {
  fn read(&self, device: &LiveStorageDevice) -> Result<SmartDiskInfo, String> {
    let windows_device = device_io::WindowsStorageDevice {
      device_path: device.device_path.clone(),
      model: device.model.clone(),
      serial_number: device.serial_number.clone(),
      firmware_version: device.firmware_version.clone(),
      capacity_bytes: device.capacity_bytes,
      interface_type: None,
    };
    let raw =
      device_io::DeviceIoControlNvmeReader.read_nvme_health_log(&device.device_path)?;
    device_io::nvme_health_log_to_smart_disk_info(&windows_device, &raw)
  }
}

#[cfg(not(target_os = "windows"))]
struct PlatformLiveSignalReader;

#[cfg(not(target_os = "windows"))]
impl LiveSignalReader for PlatformLiveSignalReader {
  fn read(&self, _device: &LiveStorageDevice) -> Result<SmartDiskInfo, String> {
    // Never reached: enumeration keeps the device cache empty here.
    Err("Live Storage Health has no native read path on this platform".to_string())
  }
}

struct CollectorState {
  devices: Vec<LiveStorageDevice>,
  last_reading: Option<(Instant, Vec<LiveStorageHealth>)>,
}

/// On-demand Live Storage Health collector. Holds the cached device list
/// and the last reading; never touches the database.
pub struct LiveStorageHealthCollector {
  state: Mutex<CollectorState>,
  identity_hash_key: [u8; STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES],
}

impl LiveStorageHealthCollector {
  pub fn new(identity_hash_key: [u8; STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES]) -> Self {
    Self {
      state: Mutex::new(CollectorState {
        devices: Vec::new(),
        last_reading: None,
      }),
      identity_hash_key,
    }
  }

  /// Enumerates storage devices and replaces the cached device list.
  /// Intended for app startup (and the explicit Storage Device Refresh,
  /// tracked separately); the live read path never calls this.
  pub fn enumerate_devices(&self) {
    self.apply_enumeration(platform_enumerate_devices());
  }

  /// Reads live signals for every cached device, returning the last
  /// reading when called again within [`LIVE_READ_MIN_INTERVAL`].
  /// Blocking: callers on an async runtime should use `spawn_blocking`.
  pub fn read_live(&self) -> Vec<LiveStorageHealth> {
    self.read_live_with(&PlatformLiveSignalReader, Instant::now())
  }

  fn apply_enumeration(&self, devices: Vec<LiveStorageDevice>) {
    let mut state = self.state.lock().unwrap();
    state.devices = devices;
    // Signals must not outlive the device list they were read from.
    state.last_reading = None;
  }

  fn read_live_with<R: LiveSignalReader>(
    &self,
    reader: &R,
    now: Instant,
  ) -> Vec<LiveStorageHealth> {
    // The lock is held across the device reads on purpose: concurrent
    // callers serialize here, and whoever loses the race finds a fresh
    // reading inside the interval guard instead of re-collecting.
    let mut state = self.state.lock().unwrap();

    if let Some((collected_at, reading)) = &state.last_reading
      && now.duration_since(*collected_at) < LIVE_READ_MIN_INTERVAL
    {
      return reading.clone();
    }

    let collected_at = chrono::Utc::now().to_rfc3339();
    let mut results = Vec::new();
    let mut surviving = Vec::new();
    for device in &state.devices {
      match reader.read(device) {
        Ok(disk) => {
          results.push(live_signal_from_disk(
            &disk,
            &self.identity_hash_key,
            &collected_at,
          ));
          surviving.push(device.clone());
        }
        Err(e) => {
          log_warn!(
            "Dropping storage device from Live Storage Health until the next enumeration",
            "collector::live_storage_health::read_live",
            Some(format!("{}: {e}", device.device_path))
          );
        }
      }
    }

    state.devices = surviving;
    state.last_reading = Some((now, results.clone()));
    results
  }
}

#[cfg(target_os = "windows")]
fn platform_enumerate_devices() -> Vec<LiveStorageDevice> {
  device_io::enumerate_storage_devices()
    .into_iter()
    .map(|device| LiveStorageDevice {
      device_path: device.device_path,
      model: device.model,
      serial_number: device.serial_number,
      firmware_version: device.firmware_version,
      capacity_bytes: device.capacity_bytes,
    })
    .collect()
}

#[cfg(not(target_os = "windows"))]
fn platform_enumerate_devices() -> Vec<LiveStorageDevice> {
  Vec::new()
}

fn live_signal_from_disk(
  disk: &SmartDiskInfo,
  identity_hash_key: &[u8; STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES],
  collected_at: &str,
) -> LiveStorageHealth {
  let display_name = disk
    .model_name
    .as_deref()
    .filter(|v| !v.trim().is_empty())
    .unwrap_or(&disk.device_name)
    .to_string();

  LiveStorageHealth {
    device_id: storage_device_id(disk, identity_hash_key),
    display_name,
    temperature_celsius: disk.temperature_celsius.map(|v| v as f32),
    available_spare_percent: attr_value_f32(disk, None, "Available Spare"),
    percentage_used: attr_value_f32(disk, None, "Percentage Used"),
    media_errors: attr_value_u64(disk, None, "Media Errors"),
    unsafe_shutdown_count: attr_value_u64(disk, None, "Unsafe Shutdowns"),
    power_on_hours: disk.power_on_hours,
    power_cycle_count: disk.power_cycle_count,
    collected_at: collected_at.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::hardware::{SmartAttribute, SmartHealthStatus};
  use std::cell::RefCell;
  use std::collections::HashMap;

  const TEST_IDENTITY_HASH_KEY: [u8; STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES] = [0x42; 32];

  struct FakeReader {
    disks: HashMap<String, Result<SmartDiskInfo, String>>,
    calls: RefCell<Vec<String>>,
  }

  impl FakeReader {
    fn new(disks: Vec<(&str, Result<SmartDiskInfo, String>)>) -> Self {
      Self {
        disks: disks
          .into_iter()
          .map(|(path, disk)| (path.to_string(), disk))
          .collect(),
        calls: RefCell::new(Vec::new()),
      }
    }
  }

  impl LiveSignalReader for FakeReader {
    fn read(&self, device: &LiveStorageDevice) -> Result<SmartDiskInfo, String> {
      self.calls.borrow_mut().push(device.device_path.clone());
      self
        .disks
        .get(&device.device_path)
        .cloned()
        .unwrap_or_else(|| Err("missing fake disk".to_string()))
    }
  }

  fn device(path: &str) -> LiveStorageDevice {
    LiveStorageDevice {
      device_path: path.to_string(),
      model: Some("Example NVMe".to_string()),
      serial_number: Some("SERIAL123".to_string()),
      firmware_version: Some("1.0".to_string()),
      capacity_bytes: Some(1_000_204_886_016),
    }
  }

  fn attr(name: &str, value: u64) -> SmartAttribute {
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

  fn nvme_disk(path: &str, serial: &str) -> SmartDiskInfo {
    SmartDiskInfo {
      device_name: path.to_string(),
      device_type: Some("nvme".to_string()),
      protocol: Some("NVMe".to_string()),
      model_name: Some("Example NVMe".to_string()),
      serial_number: Some(serial.to_string()),
      firmware_version: Some("1.0".to_string()),
      capacity_bytes: Some(1_000_204_886_016),
      health_status: SmartHealthStatus::Passed,
      temperature_celsius: Some(42),
      power_on_hours: Some(3_200),
      power_cycle_count: Some(55),
      attributes: vec![
        attr("Available Spare", 97),
        attr("Percentage Used", 8),
        attr("Media Errors", 1),
        attr("Unsafe Shutdowns", 2),
      ],
    }
  }

  fn collector_with_devices(
    devices: Vec<LiveStorageDevice>,
  ) -> LiveStorageHealthCollector {
    let collector = LiveStorageHealthCollector::new(TEST_IDENTITY_HASH_KEY);
    collector.state.lock().unwrap().devices = devices;
    collector
  }

  #[test]
  fn returns_cached_reading_within_min_interval() {
    let collector = collector_with_devices(vec![device(r"\\.\PhysicalDrive0")]);
    let reader = FakeReader::new(vec![(
      r"\\.\PhysicalDrive0",
      Ok(nvme_disk(r"\\.\PhysicalDrive0", "SERIAL123")),
    )]);
    let t0 = Instant::now();

    let first = collector.read_live_with(&reader, t0);
    let second = collector.read_live_with(&reader, t0 + Duration::from_secs(1));

    assert_eq!(first.len(), 1);
    assert_eq!(first, second);
    assert_eq!(reader.calls.borrow().len(), 1);
  }

  #[test]
  fn recollects_after_min_interval_elapses() {
    let collector = collector_with_devices(vec![device(r"\\.\PhysicalDrive0")]);
    let reader = FakeReader::new(vec![(
      r"\\.\PhysicalDrive0",
      Ok(nvme_disk(r"\\.\PhysicalDrive0", "SERIAL123")),
    )]);
    let t0 = Instant::now();

    let first = collector.read_live_with(&reader, t0);
    let second = collector.read_live_with(
      &reader,
      t0 + LIVE_READ_MIN_INTERVAL + Duration::from_secs(1),
    );

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(reader.calls.borrow().len(), 2);
  }

  #[test]
  fn drops_failed_device_until_next_enumeration() {
    let collector = collector_with_devices(vec![
      device(r"\\.\PhysicalDrive0"),
      device(r"\\.\PhysicalDrive1"),
    ]);
    let reader = FakeReader::new(vec![
      (
        r"\\.\PhysicalDrive0",
        Ok(nvme_disk(r"\\.\PhysicalDrive0", "GOOD")),
      ),
      (r"\\.\PhysicalDrive1", Err("access denied".to_string())),
    ]);
    let t0 = Instant::now();

    let first = collector.read_live_with(&reader, t0);
    let second = collector.read_live_with(
      &reader,
      t0 + LIVE_READ_MIN_INTERVAL + Duration::from_secs(1),
    );

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    // The failed device is removed from the read set, not retried.
    assert_eq!(
      *reader.calls.borrow(),
      vec![
        r"\\.\PhysicalDrive0".to_string(),
        r"\\.\PhysicalDrive1".to_string(),
        r"\\.\PhysicalDrive0".to_string(),
      ]
    );
    assert_eq!(
      collector.state.lock().unwrap().devices,
      vec![device(r"\\.\PhysicalDrive0")]
    );
  }

  #[test]
  fn all_devices_failing_returns_empty_not_error() {
    let collector = collector_with_devices(vec![
      device(r"\\.\PhysicalDrive0"),
      device(r"\\.\PhysicalDrive1"),
    ]);
    let reader = FakeReader::new(vec![
      (r"\\.\PhysicalDrive0", Err("offline".to_string())),
      (r"\\.\PhysicalDrive1", Err("access denied".to_string())),
    ]);

    let reading = collector.read_live_with(&reader, Instant::now());

    assert!(reading.is_empty());
    assert!(collector.state.lock().unwrap().devices.is_empty());
  }

  #[test]
  fn empty_device_cache_returns_empty_without_reader_calls() {
    let collector = collector_with_devices(Vec::new());
    let reader = FakeReader::new(Vec::new());

    let reading = collector.read_live_with(&reader, Instant::now());

    assert!(reading.is_empty());
    assert!(reader.calls.borrow().is_empty());
  }

  #[test]
  fn live_signal_carries_all_nvme_signals_and_daily_device_id() {
    let disk = nvme_disk(r"\\.\PhysicalDrive0", "SERIAL123");

    let signal =
      live_signal_from_disk(&disk, &TEST_IDENTITY_HASH_KEY, "2026-06-09T00:00:00+00:00");

    assert_eq!(
      signal.device_id,
      storage_device_id(&disk, &TEST_IDENTITY_HASH_KEY)
    );
    assert_eq!(signal.display_name, "Example NVMe");
    assert_eq!(signal.temperature_celsius, Some(42.0));
    assert_eq!(signal.available_spare_percent, Some(97.0));
    assert_eq!(signal.percentage_used, Some(8.0));
    assert_eq!(signal.media_errors, Some(1));
    assert_eq!(signal.unsafe_shutdown_count, Some(2));
    assert_eq!(signal.power_on_hours, Some(3_200));
    assert_eq!(signal.power_cycle_count, Some(55));
    assert_eq!(signal.collected_at, "2026-06-09T00:00:00+00:00");
  }

  #[test]
  fn display_name_falls_back_to_device_name_when_model_is_missing() {
    let mut disk = nvme_disk(r"\\.\PhysicalDrive0", "SERIAL123");
    disk.model_name = Some("   ".to_string());

    let signal =
      live_signal_from_disk(&disk, &TEST_IDENTITY_HASH_KEY, "2026-06-09T00:00:00+00:00");

    assert_eq!(signal.display_name, r"\\.\PhysicalDrive0");
  }

  #[test]
  fn apply_enumeration_replaces_devices_and_clears_last_reading() {
    let collector = collector_with_devices(vec![device(r"\\.\PhysicalDrive0")]);
    let reader = FakeReader::new(vec![(
      r"\\.\PhysicalDrive0",
      Ok(nvme_disk(r"\\.\PhysicalDrive0", "SERIAL123")),
    )]);
    let reading = collector.read_live_with(&reader, Instant::now());
    assert_eq!(reading.len(), 1);

    collector.apply_enumeration(vec![device(r"\\.\PhysicalDrive1")]);

    let state = collector.state.lock().unwrap();
    assert_eq!(state.devices, vec![device(r"\\.\PhysicalDrive1")]);
    assert!(state.last_reading.is_none());
  }

  #[cfg(not(target_os = "windows"))]
  #[test]
  fn read_live_returns_empty_on_non_windows() {
    let collector = LiveStorageHealthCollector::new(TEST_IDENTITY_HASH_KEY);
    collector.enumerate_devices();

    assert!(collector.read_live().is_empty());
  }
}
