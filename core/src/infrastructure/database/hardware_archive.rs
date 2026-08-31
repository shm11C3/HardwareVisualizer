use super::db;
use crate::persistence::archive_data::HardwareArchiveRow;

/// Insert one archived minute, stamped with the write cycle's own
/// `timestamp` rather than a fresh `Utc::now()` read here.
///
/// The caller supplies the instant so every table this cycle writes -
/// hardware, GPU, process stats, ambient - lands on one identical stamp
/// (#2045). Reading the clock per insert meant a cycle that straddled a
/// minute boundary could file its hardware row in one minute and its
/// ambient rows in the next, and the ambient pairing join is defined on
/// exactly that minute: the pair would simply be lost, silently and only
/// for the minutes where it mattered most (a busy cycle is a slow one).
pub async fn insert(
  row: HardwareArchiveRow,
  timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
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
  .bind(timestamp).execute(&pool).await?;

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
