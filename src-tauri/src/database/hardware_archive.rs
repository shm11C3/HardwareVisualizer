use crate::structs;
use crate::utils;
use sqlx::sqlite::SqlitePool;

pub async fn insert(
  cpu: structs::hardware_archive::HardwareData,
  ram: structs::hardware_archive::HardwareData,
) -> Result<(), sqlx::Error> {
  let dir_path = utils::file::get_app_data_dir("hv-database.db");
  let database_url = format!("sqlite:{}", dir_path.to_str().unwrap());

  let pool = SqlitePool::connect(&database_url).await?;

  sqlx::query(

    "INSERT INTO DATA_ARCHIVE (cpu_avg, cpu_max, cpu_min, ram_avg, ram_max, ram_min, timestamp)
    VALUES ($1, $2, $3, $4, $5, $6, $7)",
  ).bind(0).bind(cpu.avg).bind(cpu.max).bind(cpu.min).bind(ram.avg).bind(ram.max).bind(ram.min).bind(chrono::Utc::now().timestamp()).execute(&pool).await?;

  Ok(())
}
