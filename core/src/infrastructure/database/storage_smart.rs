use super::db;
use crate::models::hardware::{StorageDeviceRecord, StorageHealthSnapshot};

pub async fn insert_daily_snapshots(
  devices: Vec<StorageDeviceRecord>,
  snapshots: Vec<StorageHealthSnapshot>,
) -> Result<(), sqlx::Error> {
  if devices.is_empty() || snapshots.is_empty() {
    return Ok(());
  }

  let pool = db::get_pool().await?;
  let mut tx = pool.begin().await?;

  for device in devices {
    sqlx::query(
      r#"
      INSERT INTO storage_devices (
        id,
        display_name,
        model,
        serial_hash,
        protocol,
        capacity_bytes,
        first_seen_at,
        last_seen_at,
        is_active
      )
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1)
      ON CONFLICT(id) DO UPDATE SET
        display_name = excluded.display_name,
        model = excluded.model,
        serial_hash = COALESCE(excluded.serial_hash, storage_devices.serial_hash),
        protocol = excluded.protocol,
        capacity_bytes = excluded.capacity_bytes,
        last_seen_at = excluded.last_seen_at,
        is_active = 1
      "#,
    )
    .bind(device.id)
    .bind(device.display_name)
    .bind(device.model)
    .bind(device.serial_hash)
    .bind(device.protocol)
    .bind(to_i64(device.capacity_bytes))
    .bind(device.first_seen_at)
    .bind(device.last_seen_at)
    .execute(&mut *tx)
    .await?;
  }

  for snapshot in snapshots {
    let warning_reasons =
      serde_json::to_string(&snapshot.warning_reasons).unwrap_or_else(|_| "[]".into());

    sqlx::query(
      r#"
      INSERT INTO storage_smart_daily_snapshots (
        device_id,
        date,
        health_status,
        warning_level,
        warning_reasons,
        temperature_celsius,
        power_on_hours,
        percentage_used,
        available_spare_percent,
        reallocated_sector_count,
        current_pending_sector_count,
        offline_uncorrectable_count,
        media_errors,
        error_log_entries,
        unsafe_shutdown_count,
        collected_at
      )
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
      ON CONFLICT(device_id, date) DO NOTHING
      "#,
    )
    .bind(snapshot.device_id)
    .bind(snapshot.date)
    .bind(snapshot.health_status.as_str())
    .bind(snapshot.warning_level.as_str())
    .bind(warning_reasons)
    .bind(snapshot.temperature_celsius.map(f64::from))
    .bind(to_i64(snapshot.power_on_hours))
    .bind(snapshot.percentage_used.map(f64::from))
    .bind(snapshot.available_spare_percent.map(f64::from))
    .bind(to_i64(snapshot.reallocated_sector_count))
    .bind(to_i64(snapshot.current_pending_sector_count))
    .bind(to_i64(snapshot.offline_uncorrectable_count))
    .bind(to_i64(snapshot.media_errors))
    .bind(to_i64(snapshot.error_log_entries))
    .bind(to_i64(snapshot.unsafe_shutdown_count))
    .bind(snapshot.collected_at)
    .execute(&mut *tx)
    .await?;
  }

  tx.commit().await?;
  Ok(())
}

pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  let cutoff = (chrono::Local::now().date_naive()
    - chrono::Duration::days(retention_days as i64))
  .format("%Y-%m-%d")
  .to_string();

  sqlx::query("DELETE FROM storage_smart_daily_snapshots WHERE date < $1")
    .bind(cutoff)
    .execute(&pool)
    .await?;

  Ok(())
}

fn to_i64(value: Option<u64>) -> Option<i64> {
  value.and_then(|v| i64::try_from(v).ok())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::db;
  use crate::models::hardware::{StorageHealthStatus, StorageWarningLevel};

  async fn setup_test_db() -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    assert!(db::init(file.path().to_path_buf()));

    let pool = db::get_pool().await.unwrap();
    sqlx::query(
      r#"
      CREATE TABLE storage_devices (
        id TEXT PRIMARY KEY,
        display_name TEXT NOT NULL,
        model TEXT,
        serial_hash TEXT,
        protocol TEXT,
        capacity_bytes INTEGER,
        first_seen_at TEXT NOT NULL,
        last_seen_at TEXT NOT NULL,
        is_active INTEGER NOT NULL DEFAULT 1
      )
      "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
      r#"
      CREATE TABLE storage_smart_daily_snapshots (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        device_id TEXT NOT NULL,
        date TEXT NOT NULL,
        health_status TEXT NOT NULL,
        warning_level TEXT NOT NULL DEFAULT 'none',
        warning_reasons TEXT,
        temperature_celsius REAL,
        power_on_hours INTEGER,
        percentage_used REAL,
        available_spare_percent REAL,
        reallocated_sector_count INTEGER,
        current_pending_sector_count INTEGER,
        offline_uncorrectable_count INTEGER,
        media_errors INTEGER,
        error_log_entries INTEGER,
        unsafe_shutdown_count INTEGER,
        collected_at TEXT NOT NULL,
        UNIQUE(device_id, date),
        FOREIGN KEY(device_id) REFERENCES storage_devices(id)
      )
      "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    file
  }

  fn device_record(id: &str) -> StorageDeviceRecord {
    StorageDeviceRecord {
      id: id.to_string(),
      display_name: format!("Device {id}"),
      model: Some("Example SSD".to_string()),
      serial_hash: Some(format!("serial-{id}")),
      protocol: Some("NVMe".to_string()),
      capacity_bytes: Some(1_000_000),
      first_seen_at: "2026-05-10T00:00:00Z".to_string(),
      last_seen_at: "2026-05-10T00:00:00Z".to_string(),
    }
  }

  fn snapshot(device_id: &str, date: &str) -> StorageHealthSnapshot {
    StorageHealthSnapshot {
      device_id: device_id.to_string(),
      date: date.to_string(),
      health_status: StorageHealthStatus::Good,
      warning_level: StorageWarningLevel::None,
      temperature_celsius: Some(38.0),
      power_on_hours: Some(120),
      percentage_used: None,
      available_spare_percent: None,
      reallocated_sector_count: None,
      current_pending_sector_count: None,
      offline_uncorrectable_count: None,
      media_errors: None,
      error_log_entries: None,
      unsafe_shutdown_count: None,
      warning_reasons: vec!["test reason".to_string()],
      collected_at: "2026-05-10T00:00:00Z".to_string(),
    }
  }

  #[tokio::test]
  async fn insert_daily_snapshots_deduplicates_and_preserves_active_flags() {
    let _dir = setup_test_db().await;
    let pool = db::get_pool().await.unwrap();

    sqlx::query(
      r#"
      INSERT INTO storage_devices (
        id, display_name, first_seen_at, last_seen_at, is_active
      )
      VALUES
        ('disk-b', 'Device disk-b', '2026-05-09T00:00:00Z', '2026-05-09T00:00:00Z', 0),
        ('disk-c', 'Device disk-c', '2026-05-09T00:00:00Z', '2026-05-09T00:00:00Z', 1)
      "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    insert_daily_snapshots(
      vec![device_record("disk-a")],
      vec![snapshot("disk-a", "2026-05-10")],
    )
    .await
    .unwrap();
    insert_daily_snapshots(
      vec![device_record("disk-a"), device_record("disk-b")],
      vec![
        snapshot("disk-a", "2026-05-10"),
        snapshot("disk-b", "2026-05-10"),
      ],
    )
    .await
    .unwrap();

    let total_for_date: (i64,) = sqlx::query_as(
      "SELECT COUNT(1) FROM storage_smart_daily_snapshots WHERE date = $1",
    )
    .bind("2026-05-10")
    .fetch_one(&pool)
    .await
    .unwrap();
    let disk_a_count: (i64,) = sqlx::query_as(
      "SELECT COUNT(1) FROM storage_smart_daily_snapshots WHERE device_id = $1 AND date = $2",
    )
    .bind("disk-a")
    .bind("2026-05-10")
    .fetch_one(&pool)
    .await
    .unwrap();
    let disk_b_active: (i64,) =
      sqlx::query_as("SELECT is_active FROM storage_devices WHERE id = $1")
        .bind("disk-b")
        .fetch_one(&pool)
        .await
        .unwrap();
    let disk_c_active: (i64,) =
      sqlx::query_as("SELECT is_active FROM storage_devices WHERE id = $1")
        .bind("disk-c")
        .fetch_one(&pool)
        .await
        .unwrap();
    let warning_reasons: (String,) = sqlx::query_as(
      "SELECT warning_reasons FROM storage_smart_daily_snapshots WHERE device_id = $1 AND date = $2",
    )
    .bind("disk-b")
    .bind("2026-05-10")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(total_for_date.0, 2);
    assert_eq!(disk_a_count.0, 1);
    assert_eq!(disk_b_active.0, 1);
    assert_eq!(disk_c_active.0, 1);
    assert_eq!(warning_reasons.0, r#"["test reason"]"#);
  }
}
