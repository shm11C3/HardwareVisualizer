use super::db;
use crate::models;

pub async fn insert(
  cpu: models::hardware_archive::HardwareData,
  ram: models::hardware_archive::HardwareData,
) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query(
    "INSERT INTO DATA_ARCHIVE (cpu_avg, cpu_max, cpu_min, ram_avg, ram_max, ram_min, timestamp)
    VALUES ($1, $2, $3, $4, $5, $6, $7)",
  ).bind(cpu.avg).bind(cpu.max).bind(cpu.min).bind(ram.avg).bind(ram.max).bind(ram.min).bind(chrono::Utc::now()).execute(&pool).await?;

  Ok(())
}

pub async fn delete_old_data(refresh_interval_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query("DELETE FROM DATA_ARCHIVE WHERE timestamp < $1")
    .bind(chrono::Utc::now() - chrono::Duration::days(refresh_interval_days as i64))
    .execute(&pool)
    .await?;

  Ok(())
}
