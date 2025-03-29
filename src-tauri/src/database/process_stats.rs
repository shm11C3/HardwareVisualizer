use super::db;
use crate::structs;

pub async fn insert(
  processes: Vec<structs::hardware_archive::ProcessStatData>,
) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  for proc in processes {
    sqlx::query(
      "INSERT INTO PROCESS_STATS (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
       VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(proc.pid)
    .bind(&proc.process_name)
    .bind(proc.cpu_usage)
    .bind(proc.memory_usage)
    .bind(proc.execution_sec)
    .bind(chrono::Utc::now())
    .execute(&pool)
    .await?;
  }

  Ok(())
}

pub async fn delete_old_data(refresh_interval_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query("DELETE FROM PROCESS_STATS WHERE timestamp < $1")
    .bind(chrono::Utc::now() - chrono::Duration::days(refresh_interval_days as i64))
    .execute(&pool)
    .await?;

  Ok(())
}
