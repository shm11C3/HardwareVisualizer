use super::db;
use crate::persistence::archive_data::GpuData;

pub async fn insert(data: GpuData) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query(
    "INSERT INTO GPU_DATA_ARCHIVE (gpu_id, gpu_name, usage_avg, usage_max, usage_min, temperature_avg, temperature_max, temperature_min, dedicated_memory_avg, dedicated_memory_max, dedicated_memory_min, timestamp)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
  ).bind(data.gpu_id).bind(data.gpu_name).bind(data.usage_avg).bind(data.usage_max).bind(data.usage_min).bind(data.temperature_avg).bind(data.temperature_max).bind(data.temperature_min).bind(data.dedicated_memory_avg).bind(data.dedicated_memory_max).bind(data.dedicated_memory_min).bind(chrono::Utc::now()).execute(&pool).await?;

  Ok(())
}

pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query("DELETE FROM GPU_DATA_ARCHIVE WHERE timestamp < $1")
    .bind(chrono::Utc::now() - chrono::Duration::days(retention_days as i64))
    .execute(&pool)
    .await?;

  Ok(())
}
