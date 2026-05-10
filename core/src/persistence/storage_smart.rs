//! Daily SMART snapshot worker.

use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{Duration, MissedTickBehavior, interval};

use crate::infrastructure::database;
use crate::models::hardware::{
  SmartAttribute, SmartDiskInfo, SmartHealthStatus, StorageDeviceRecord,
  StorageHealthSnapshot, StorageHealthStatus, StorageWarningLevel,
};
use crate::{log_error, log_info, log_warn};

const STORAGE_SMART_CHECK_INTERVAL_SECONDS: u64 = 60;
const TEMPERATURE_WARNING_CELSIUS: f32 = 55.0;
const TEMPERATURE_CRITICAL_CELSIUS: f32 = 70.0;
const PERCENTAGE_USED_WARNING: f32 = 80.0;
const PERCENTAGE_USED_CRITICAL: f32 = 100.0;
const AVAILABLE_SPARE_WARNING_PERCENT: f32 = 20.0;
const AVAILABLE_SPARE_CRITICAL_PERCENT: f32 = 10.0;

pub struct StorageSmartController {
  handle: JoinHandle<()>,
  stop_tx: watch::Sender<bool>,
}

impl StorageSmartController {
  pub fn setup(runtime: Handle, retention_days: u32) -> Self {
    let (stop_tx, mut stop_rx) = watch::channel(false);

    let handle = runtime.spawn(async move {
      let mut last_checked_date = None;
      if run_daily_snapshot_if_needed(retention_days).await {
        last_checked_date = Some(local_date_string());
      }

      let mut ticker =
        interval(Duration::from_secs(STORAGE_SMART_CHECK_INTERVAL_SECONDS));
      ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
      ticker.tick().await;

      loop {
        tokio::select! {
          biased;
          changed = stop_rx.changed() => {
            if changed.is_err() || *stop_rx.borrow() {
              log_info!(
                "storage SMART worker shutdown signal received",
                "persistence::storage_smart",
                None::<&str>
              );
              break;
            }
          }
          _ = ticker.tick() => {
            let today = local_date_string();
            if last_checked_date.as_deref() != Some(today.as_str())
              && run_daily_snapshot_if_needed(retention_days).await
            {
              last_checked_date = Some(today);
            }
          }
        }
      }
    });

    Self { handle, stop_tx }
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}

async fn run_daily_snapshot_if_needed(retention_days: u32) -> bool {
  let date = local_date_string();

  match database::storage_smart::has_snapshot_for_date(&date).await {
    Ok(true) => {
      if let Err(e) = database::storage_smart::delete_old_data(retention_days).await {
        log_error!(
          "Failed to delete old storage SMART snapshots",
          "persistence::storage_smart::run_daily_snapshot_if_needed",
          Some(e.to_string())
        );
        return false;
      }
      return true;
    }
    Ok(false) => {}
    Err(e) => {
      log_error!(
        "Failed to check storage SMART snapshot existence",
        "persistence::storage_smart::run_daily_snapshot_if_needed",
        Some(e.to_string())
      );
      return false;
    }
  }

  let collected_at = chrono::Utc::now().to_rfc3339();
  let disks = match tokio::task::spawn_blocking(collect_platform_smart_info).await {
    Ok(Ok(disks)) => disks,
    Ok(Err(e)) => {
      log_error!(
        "Failed to collect storage SMART info",
        "persistence::storage_smart::run_daily_snapshot_if_needed",
        Some(e)
      );
      return false;
    }
    Err(e) => {
      log_error!(
        "Failed to join storage SMART collection task",
        "persistence::storage_smart::run_daily_snapshot_if_needed",
        Some(e.to_string())
      );
      return false;
    }
  };

  let (devices, snapshots): (Vec<_>, Vec<_>) = disks
    .iter()
    .map(|disk| build_daily_snapshot(disk, &date, &collected_at))
    .unzip();

  if snapshots.is_empty() {
    log_warn!(
      "Storage SMART collection returned no disks",
      "persistence::storage_smart::run_daily_snapshot_if_needed",
      None::<&str>
    );
    return false;
  }

  if let Err(e) =
    database::storage_smart::insert_daily_snapshots(devices, snapshots).await
  {
    log_error!(
      "Failed to insert storage SMART daily snapshots",
      "persistence::storage_smart::run_daily_snapshot_if_needed",
      Some(e.to_string())
    );
    return false;
  }

  if let Err(e) = database::storage_smart::delete_old_data(retention_days).await {
    log_error!(
      "Failed to delete old storage SMART snapshots",
      "persistence::storage_smart::run_daily_snapshot_if_needed",
      Some(e.to_string())
    );
    return false;
  }

  true
}

fn build_daily_snapshot(
  disk: &SmartDiskInfo,
  date: &str,
  collected_at: &str,
) -> (StorageDeviceRecord, StorageHealthSnapshot) {
  let device_id = storage_device_id(disk);
  let display_name = disk
    .model_name
    .as_deref()
    .filter(|v| !v.trim().is_empty())
    .unwrap_or(&disk.device_name)
    .to_string();
  let serial_hash = normalized_serial(disk).map(hash_identifier);

  let temperature_celsius = disk.temperature_celsius.map(|v| v as f32);
  let percentage_used = attr_value_f32(disk, None, "Percentage Used");
  let available_spare_percent = attr_value_f32(disk, None, "Available Spare");
  let reallocated_sector_count =
    attr_value_u64(disk, Some(5), "Reallocated Sector Count");
  let current_pending_sector_count =
    attr_value_u64(disk, Some(197), "Current Pending Sector Count");
  let offline_uncorrectable_count =
    attr_value_u64(disk, Some(198), "Offline Uncorrectable Sector Count");
  let media_errors = attr_value_u64(disk, None, "Media Errors");
  let error_log_entries = attr_value_u64(disk, None, "Error Information Log Entries");
  let unsafe_shutdown_count = attr_value_u64(disk, None, "Unsafe Shutdowns");

  let (health_status, warning_level, warning_reasons) =
    classify_snapshot_warnings(SnapshotSignals {
      smart_health_status: &disk.health_status,
      temperature_celsius,
      percentage_used,
      available_spare_percent,
      reallocated_sector_count,
      current_pending_sector_count,
      offline_uncorrectable_count,
      media_errors,
    });

  (
    StorageDeviceRecord {
      id: device_id.clone(),
      display_name,
      model: disk.model_name.clone(),
      serial_hash,
      protocol: disk.protocol.clone().or_else(|| disk.device_type.clone()),
      capacity_bytes: disk.capacity_bytes,
      first_seen_at: collected_at.to_string(),
      last_seen_at: collected_at.to_string(),
    },
    StorageHealthSnapshot {
      device_id,
      date: date.to_string(),
      health_status,
      warning_level,
      temperature_celsius,
      power_on_hours: disk.power_on_hours,
      percentage_used,
      available_spare_percent,
      reallocated_sector_count,
      current_pending_sector_count,
      offline_uncorrectable_count,
      media_errors,
      error_log_entries,
      unsafe_shutdown_count,
      warning_reasons,
      collected_at: collected_at.to_string(),
    },
  )
}

struct SnapshotSignals<'a> {
  smart_health_status: &'a SmartHealthStatus,
  temperature_celsius: Option<f32>,
  percentage_used: Option<f32>,
  available_spare_percent: Option<f32>,
  reallocated_sector_count: Option<u64>,
  current_pending_sector_count: Option<u64>,
  offline_uncorrectable_count: Option<u64>,
  media_errors: Option<u64>,
}

fn classify_snapshot_warnings(
  signals: SnapshotSignals<'_>,
) -> (StorageHealthStatus, StorageWarningLevel, Vec<String>) {
  let mut level = match signals.smart_health_status {
    SmartHealthStatus::Passed => StorageWarningLevel::None,
    SmartHealthStatus::Failed => StorageWarningLevel::Critical,
    SmartHealthStatus::Unknown => StorageWarningLevel::Unknown,
  };
  let mut reasons = Vec::new();

  if matches!(signals.smart_health_status, SmartHealthStatus::Failed) {
    reasons.push("SMART overall health check failed".to_string());
  }

  if let Some(temp) = signals.temperature_celsius {
    if temp >= TEMPERATURE_CRITICAL_CELSIUS {
      promote_level(&mut level, StorageWarningLevel::Critical);
      reasons.push(format!("Temperature is critical ({temp:.0}°C)"));
    } else if temp >= TEMPERATURE_WARNING_CELSIUS {
      promote_level(&mut level, StorageWarningLevel::Warning);
      reasons.push(format!("Temperature is elevated ({temp:.0}°C)"));
    }
  }

  if let Some(used) = signals.percentage_used {
    if used >= PERCENTAGE_USED_CRITICAL {
      promote_level(&mut level, StorageWarningLevel::Critical);
      reasons.push(format!("NVMe percentage used is critical ({used:.0}%)"));
    } else if used >= PERCENTAGE_USED_WARNING {
      promote_level(&mut level, StorageWarningLevel::Warning);
      reasons.push(format!("NVMe percentage used is high ({used:.0}%)"));
    }
  }

  if let Some(spare) = signals.available_spare_percent {
    if spare <= AVAILABLE_SPARE_CRITICAL_PERCENT {
      promote_level(&mut level, StorageWarningLevel::Critical);
      reasons.push(format!("Available spare is critical ({spare:.0}%)"));
    } else if spare <= AVAILABLE_SPARE_WARNING_PERCENT {
      promote_level(&mut level, StorageWarningLevel::Warning);
      reasons.push(format!("Available spare is low ({spare:.0}%)"));
    }
  }

  if signals.reallocated_sector_count.unwrap_or(0) > 0 {
    promote_level(&mut level, StorageWarningLevel::Warning);
    reasons.push("Reallocated sectors are present".to_string());
  }
  if signals.current_pending_sector_count.unwrap_or(0) > 0 {
    promote_level(&mut level, StorageWarningLevel::Critical);
    reasons.push("Current pending sectors are present".to_string());
  }
  if signals.offline_uncorrectable_count.unwrap_or(0) > 0 {
    promote_level(&mut level, StorageWarningLevel::Critical);
    reasons.push("Offline uncorrectable sectors are present".to_string());
  }
  if signals.media_errors.unwrap_or(0) > 0 {
    promote_level(&mut level, StorageWarningLevel::Critical);
    reasons.push("NVMe media errors are present".to_string());
  }

  let health_status = match level {
    StorageWarningLevel::None => StorageHealthStatus::Good,
    StorageWarningLevel::Warning => StorageHealthStatus::Warning,
    StorageWarningLevel::Critical => StorageHealthStatus::Critical,
    StorageWarningLevel::Unknown => StorageHealthStatus::Unknown,
  };

  (health_status, level, reasons)
}

fn promote_level(current: &mut StorageWarningLevel, next: StorageWarningLevel) {
  let current_rank = warning_rank(current);
  let next_rank = warning_rank(&next);
  if next_rank > current_rank {
    *current = next;
  }
}

fn warning_rank(level: &StorageWarningLevel) -> u8 {
  match level {
    StorageWarningLevel::None => 0,
    StorageWarningLevel::Unknown => 1,
    StorageWarningLevel::Warning => 2,
    StorageWarningLevel::Critical => 3,
  }
}

fn storage_device_id(disk: &SmartDiskInfo) -> String {
  let identity = if let Some(serial) = normalized_serial(disk) {
    format!("{}:{}", disk_protocol(disk), serial)
  } else {
    format!(
      "{}:{}:{}:{:?}",
      disk_protocol(disk),
      disk.model_name.as_deref().unwrap_or("unknown"),
      disk.device_name,
      disk.capacity_bytes,
    )
  };

  format!("storage:{}", hash_identifier(&identity))
}

fn normalized_serial(disk: &SmartDiskInfo) -> Option<&str> {
  disk
    .serial_number
    .as_deref()
    .map(str::trim)
    .filter(|serial| !serial.is_empty())
}

fn disk_protocol(disk: &SmartDiskInfo) -> &str {
  disk
    .protocol
    .as_deref()
    .or(disk.device_type.as_deref())
    .unwrap_or("unknown")
}

fn hash_identifier(value: &str) -> String {
  const FNV_OFFSET: u64 = 0xcbf29ce484222325;
  const FNV_PRIME: u64 = 0x100000001b3;

  let hash = value
    .trim()
    .as_bytes()
    .iter()
    .fold(FNV_OFFSET, |acc, byte| {
      (acc ^ (*byte as u64)).wrapping_mul(FNV_PRIME)
    });
  format!("{hash:016x}")
}

fn attr_value_u64(disk: &SmartDiskInfo, id: Option<u32>, name: &str) -> Option<u64> {
  find_attr(disk, id, name).and_then(|attr| {
    attr
      .raw_value
      .as_deref()
      .and_then(parse_smart_u64)
      .or(attr.current)
  })
}

fn attr_value_f32(disk: &SmartDiskInfo, id: Option<u32>, name: &str) -> Option<f32> {
  find_attr(disk, id, name).and_then(|attr| {
    attr
      .raw_value
      .as_deref()
      .and_then(parse_smart_f32)
      .or_else(|| attr.current.map(|v| v as f32))
  })
}

fn find_attr<'a>(
  disk: &'a SmartDiskInfo,
  id: Option<u32>,
  name: &str,
) -> Option<&'a SmartAttribute> {
  match id {
    Some(id) => disk.attributes.iter().find(|attr| attr.id == Some(id)),
    None => disk
      .attributes
      .iter()
      .find(|attr| attr.name.eq_ignore_ascii_case(name)),
  }
}

fn parse_smart_u64(raw: &str) -> Option<u64> {
  raw.split_whitespace().next()?.parse::<u64>().ok()
}

fn parse_smart_f32(raw: &str) -> Option<f32> {
  raw.split_whitespace().next()?.parse::<f32>().ok()
}

fn local_date_string() -> String {
  chrono::Local::now()
    .date_naive()
    .format("%Y-%m-%d")
    .to_string()
}

#[cfg(target_os = "linux")]
fn collect_platform_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  crate::infrastructure::providers::linux::smart::get_smart_info()
}

#[cfg(target_os = "macos")]
fn collect_platform_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  crate::infrastructure::providers::macos::smart::get_smart_info()
}

#[cfg(target_os = "windows")]
fn collect_platform_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  crate::infrastructure::providers::windows::smart::get_smart_info()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn disk(attributes: Vec<SmartAttribute>) -> SmartDiskInfo {
    SmartDiskInfo {
      device_name: "/dev/sda".to_string(),
      device_type: Some("sat".to_string()),
      protocol: Some("ATA".to_string()),
      model_name: Some("Example SSD".to_string()),
      serial_number: Some("SERIAL123".to_string()),
      firmware_version: None,
      capacity_bytes: Some(512_110_190_592),
      health_status: SmartHealthStatus::Passed,
      temperature_celsius: Some(40),
      power_on_hours: Some(100),
      power_cycle_count: Some(10),
      attributes,
    }
  }

  fn attr(id: Option<u32>, name: &str, raw: &str) -> SmartAttribute {
    SmartAttribute {
      id,
      name: name.to_string(),
      current: None,
      worst: None,
      threshold: None,
      raw_value: Some(raw.to_string()),
      when_failed: None,
    }
  }

  #[test]
  fn builds_snapshot_fields_from_smart_attributes() {
    let input = disk(vec![
      attr(Some(5), "Reallocated Sector Count", "3"),
      attr(Some(197), "Current Pending Sector Count", "0"),
      attr(Some(198), "Offline Uncorrectable Sector Count", "0"),
    ]);

    let (device, snapshot) =
      build_daily_snapshot(&input, "2026-05-10", "2026-05-10T00:00:00Z");

    assert!(device.id.starts_with("storage:"));
    assert_eq!(device.serial_hash.as_deref(), Some("81b0d204d2475417"));
    assert_eq!(snapshot.device_id, device.id);
    assert_eq!(snapshot.date, "2026-05-10");
    assert_eq!(snapshot.health_status, StorageHealthStatus::Warning);
    assert_eq!(snapshot.warning_level, StorageWarningLevel::Warning);
    assert_eq!(snapshot.reallocated_sector_count, Some(3));
  }

  #[test]
  fn classifies_critical_nvme_signals() {
    let (_, snapshot) = build_daily_snapshot(
      &disk(vec![
        attr(None, "Percentage Used", "101"),
        attr(None, "Available Spare", "5"),
        attr(None, "Media Errors", "1"),
      ]),
      "2026-05-10",
      "2026-05-10T00:00:00Z",
    );

    assert_eq!(snapshot.health_status, StorageHealthStatus::Critical);
    assert_eq!(snapshot.warning_level, StorageWarningLevel::Critical);
    assert_eq!(snapshot.percentage_used, Some(101.0));
    assert_eq!(snapshot.available_spare_percent, Some(5.0));
    assert_eq!(snapshot.media_errors, Some(1));
    assert!(!snapshot.warning_reasons.is_empty());
  }

  #[test]
  fn stable_device_id_prefers_serial_identity() {
    let mut first = disk(Vec::new());
    first.device_name = "/dev/sda".to_string();
    let mut second = first.clone();
    second.device_name = "/dev/sdb".to_string();

    assert_eq!(storage_device_id(&first), storage_device_id(&second));
  }

  #[test]
  fn blank_serial_is_treated_as_missing_identity() {
    let mut first = disk(Vec::new());
    first.serial_number = Some("   ".to_string());
    first.device_name = "/dev/sda".to_string();
    let mut second = first.clone();
    second.device_name = "/dev/sdb".to_string();

    let (device, _) = build_daily_snapshot(&first, "2026-05-10", "2026-05-10T00:00:00Z");

    assert_eq!(device.serial_hash, None);
    assert_ne!(storage_device_id(&first), storage_device_id(&second));
  }

  #[test]
  fn device_id_uses_device_type_when_protocol_is_missing() {
    let mut input = disk(Vec::new());
    input.protocol = None;
    input.device_type = Some("nvme".to_string());

    let expected = format!("storage:{}", hash_identifier("nvme:SERIAL123"));

    assert_eq!(storage_device_id(&input), expected);
  }

  #[test]
  fn attr_lookup_uses_id_only_when_id_is_specified() {
    let input = disk(vec![
      attr(Some(9), "Reallocated Sector Count", "99"),
      attr(Some(5), "Vendor Specific", "3"),
    ]);

    assert_eq!(
      attr_value_u64(&input, Some(5), "Reallocated Sector Count"),
      Some(3)
    );
  }

  #[test]
  fn attr_lookup_does_not_fallback_to_name_when_id_is_missing() {
    let input = disk(vec![attr(None, "Reallocated Sector Count", "99")]);

    assert_eq!(
      attr_value_u64(&input, Some(5), "Reallocated Sector Count"),
      None
    );
  }
}
