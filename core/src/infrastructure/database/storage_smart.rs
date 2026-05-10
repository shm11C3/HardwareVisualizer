use super::db;
use crate::models::hardware::{StorageDeviceRecord, StorageHealthSnapshot};

pub async fn has_snapshot_for_date(date: &str) -> Result<bool, sqlx::Error> {
  let pool = db::get_pool().await?;
  let row: (i64,) =
    sqlx::query_as("SELECT COUNT(1) FROM storage_smart_daily_snapshots WHERE date = $1")
      .bind(date)
      .fetch_one(&pool)
      .await?;

  Ok(row.0 > 0)
}

pub async fn insert_daily_snapshots(
  devices: Vec<StorageDeviceRecord>,
  snapshots: Vec<StorageHealthSnapshot>,
) -> Result<(), sqlx::Error> {
  if devices.is_empty() || snapshots.is_empty() {
    return Ok(());
  }

  let pool = db::get_pool().await?;
  let mut tx = pool.begin().await?;

  sqlx::query("UPDATE storage_devices SET is_active = 0")
    .execute(&mut *tx)
    .await?;

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
