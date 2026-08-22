use super::db;
use crate::persistence::archive_data::HardwareData;

pub async fn insert(
  cpu: HardwareData,
  ram: HardwareData,
  cpu_temperature: HardwareData,
) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query(
    "INSERT INTO DATA_ARCHIVE (cpu_avg, cpu_max, cpu_min, ram_avg, ram_max, ram_min, cpu_temperature_avg, cpu_temperature_max, cpu_temperature_min, timestamp)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
  ).bind(cpu.avg).bind(cpu.max).bind(cpu.min).bind(ram.avg).bind(ram.max).bind(ram.min).bind(cpu_temperature.avg).bind(cpu_temperature.max).bind(cpu_temperature.min).bind(chrono::Utc::now()).execute(&pool).await?;

  Ok(())
}

pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query("DELETE FROM DATA_ARCHIVE WHERE timestamp < $1")
    .bind(chrono::Utc::now() - chrono::Duration::days(retention_days as i64))
    .execute(&pool)
    .await?;

  Ok(())
}
