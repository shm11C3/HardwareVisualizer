use super::db;
use crate::persistence::archive_data::ProcessStatData;

pub async fn insert(processes: Vec<ProcessStatData>) -> Result<(), sqlx::Error> {
  if processes.is_empty() {
    return Ok(());
  }

  let pool = db::get_pool().await?;
  let mut tx = pool.begin().await?;

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
    .execute(&mut *tx)
    .await?;
  }

  tx.commit().await?;
  Ok(())
}

pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;

  sqlx::query("DELETE FROM PROCESS_STATS WHERE timestamp < $1")
    .bind(chrono::Utc::now() - chrono::Duration::days(retention_days as i64))
    .execute(&pool)
    .await?;

  Ok(())
}
