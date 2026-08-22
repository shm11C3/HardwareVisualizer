use super::db;
use crate::persistence::archive_data::HardwareArchiveRow;

pub async fn insert(row: HardwareArchiveRow) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query(
    "INSERT INTO DATA_ARCHIVE (cpu_avg, cpu_max, cpu_min, ram_avg, ram_max, ram_min, cpu_temperature_avg, cpu_temperature_max, cpu_temperature_min, cpu_power_avg, cpu_power_max, cpu_power_min, gpu_power_avg, gpu_power_max, gpu_power_min, ane_power_avg, ane_power_max, ane_power_min, package_power_avg, package_power_max, package_power_min, timestamp)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)",
  )
  .bind(row.cpu.avg).bind(row.cpu.max).bind(row.cpu.min)
  .bind(row.memory.avg).bind(row.memory.max).bind(row.memory.min)
  .bind(row.cpu_temperature.avg).bind(row.cpu_temperature.max).bind(row.cpu_temperature.min)
  .bind(row.cpu_power.avg).bind(row.cpu_power.max).bind(row.cpu_power.min)
  .bind(row.gpu_power.avg).bind(row.gpu_power.max).bind(row.gpu_power.min)
  .bind(row.ane_power.avg).bind(row.ane_power.max).bind(row.ane_power.min)
  .bind(row.package_power.avg).bind(row.package_power.max).bind(row.package_power.min)
  .bind(chrono::Utc::now()).execute(&pool).await?;

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
