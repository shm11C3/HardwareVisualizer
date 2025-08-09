use crate::structs;
use crate::utils;
use sqlx::sqlite::SqlitePool;
use tauri::Config;

pub async fn get_pool(config: &Config) -> Result<SqlitePool, sqlx::Error> {
  let dir_path = utils::file::get_app_data_dir(config, "hv-database.db");
  let database_url = format!("sqlite:{dir_path}", dir_path = dir_path.to_str().unwrap());

  let pool = SqlitePool::connect(&database_url).await?;

  Ok(pool)
}

pub async fn insert(
  data: structs::hardware_archive::GpuData,
  config: tauri::State<'_, Config>,
) -> Result<(), sqlx::Error> {
  let pool = get_pool(&config).await?;

  sqlx::query(
    "INSERT INTO GPU_DATA_ARCHIVE (gpu_name, usage_avg, usage_max, usage_min, temperature_avg, temperature_max, temperature_min, dedicated_memory_avg, dedicated_memory_max, dedicated_memory_min, timestamp)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
  ).bind(data.gpu_name).bind(data.usage_avg).bind(data.usage_max).bind(data.usage_min).bind(data.temperature_avg).bind(data.temperature_max).bind(data.temperature_min).bind(data.dedicated_memory_avg).bind(data.dedicated_memory_max).bind(data.dedicated_memory_min).bind(chrono::Utc::now()).execute(&pool).await?;

  Ok(())
}

pub async fn delete_old_data(
  refresh_interval_days: u32,
  config: tauri::State<'_, Config>,
) -> Result<(), sqlx::Error> {
  let pool = get_pool(&config).await?;

  sqlx::query("DELETE FROM GPU_DATA_ARCHIVE WHERE timestamp < $1")
    .bind(chrono::Utc::now() - chrono::Duration::days(refresh_interval_days as i64))
    .execute(&pool)
    .await?;

  Ok(())
}
