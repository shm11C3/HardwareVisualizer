use super::db;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataArchiveColumn {
  CpuAvg,
  CpuMax,
  CpuMin,
  RamAvg,
  RamMax,
  RamMin,
}

impl DataArchiveColumn {
  fn sql(self) -> &'static str {
    match self {
      Self::CpuAvg => "cpu_avg",
      Self::CpuMax => "cpu_max",
      Self::CpuMin => "cpu_min",
      Self::RamAvg => "ram_avg",
      Self::RamMax => "ram_max",
      Self::RamMin => "ram_min",
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuArchiveColumn {
  UsageAvg,
  UsageMax,
  UsageMin,
  TemperatureAvg,
  TemperatureMax,
  TemperatureMin,
  DedicatedMemoryAvg,
  DedicatedMemoryMax,
  DedicatedMemoryMin,
}

impl GpuArchiveColumn {
  fn sql(self) -> &'static str {
    match self {
      Self::UsageAvg => "usage_avg",
      Self::UsageMax => "usage_max",
      Self::UsageMin => "usage_min",
      Self::TemperatureAvg => "temperature_avg",
      Self::TemperatureMax => "temperature_max",
      Self::TemperatureMin => "temperature_min",
      Self::DedicatedMemoryAvg => "dedicated_memory_avg",
      Self::DedicatedMemoryMax => "dedicated_memory_max",
      Self::DedicatedMemoryMin => "dedicated_memory_min",
    }
  }
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ArchiveRecord {
  pub id: i64,
  pub value: Option<f64>,
  pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ProcessStatRecord {
  pub pid: i64,
  pub process_name: String,
  pub avg_cpu_usage: f64,
  pub avg_memory_usage: f64,
  pub total_execution_sec: i64,
  pub latest_timestamp: String,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct GpuNameRow {
  gpu_name: String,
}

pub async fn select_data_archive_records(
  column: DataArchiveColumn,
  start: &str,
  end: &str,
) -> Result<Vec<ArchiveRecord>, sqlx::Error> {
  let pool = db::get_pool().await?;
  let sql = data_archive_select_sql(column);

  sqlx::query_as::<_, ArchiveRecord>(&sql)
    .bind(start)
    .bind(end)
    .fetch_all(&pool)
    .await
}

pub async fn select_gpu_archive_records(
  column: GpuArchiveColumn,
  gpu_name: &str,
  start: &str,
  end: &str,
) -> Result<Vec<ArchiveRecord>, sqlx::Error> {
  let pool = db::get_pool().await?;
  let sql = gpu_archive_select_sql(column);

  sqlx::query_as::<_, ArchiveRecord>(&sql)
    .bind(gpu_name)
    .bind(start)
    .bind(end)
    .fetch_all(&pool)
    .await
}

pub async fn select_process_stats(
  start: &str,
  end: &str,
  order_by_cpu_desc: bool,
) -> Result<Vec<ProcessStatRecord>, sqlx::Error> {
  let pool = db::get_pool().await?;
  let order_by = if order_by_cpu_desc {
    " ORDER BY avg_cpu_usage DESC"
  } else {
    ""
  };
  let sql = format!(
    "SELECT
       pid,
       process_name,
       AVG(cpu_usage) AS avg_cpu_usage,
       AVG(memory_usage) AS avg_memory_usage,
       MAX(execution_sec) AS total_execution_sec,
       MAX(timestamp) AS latest_timestamp
     FROM PROCESS_STATS
     WHERE timestamp BETWEEN $1 AND $2
     GROUP BY pid, process_name{order_by}"
  );

  sqlx::query_as::<_, ProcessStatRecord>(&sql)
    .bind(start)
    .bind(end)
    .fetch_all(&pool)
    .await
}

pub async fn select_gpu_names() -> Result<Vec<String>, sqlx::Error> {
  let pool = db::get_pool().await?;
  let rows = sqlx::query_as::<_, GpuNameRow>(
    "SELECT DISTINCT gpu_name
     FROM GPU_DATA_ARCHIVE
     WHERE gpu_name IS NOT NULL
       AND gpu_name != 'Unknown'",
  )
  .fetch_all(&pool)
  .await?;

  Ok(rows.into_iter().map(|row| row.gpu_name).collect())
}

fn data_archive_select_sql(column: DataArchiveColumn) -> String {
  format!(
    "SELECT id, CAST({} AS REAL) AS value, timestamp
     FROM DATA_ARCHIVE
     WHERE timestamp BETWEEN $1 AND $2",
    column.sql()
  )
}

fn gpu_archive_select_sql(column: GpuArchiveColumn) -> String {
  format!(
    "SELECT id, CAST({} AS REAL) AS value, timestamp
     FROM GPU_DATA_ARCHIVE
     WHERE gpu_name = $1
       AND timestamp BETWEEN $2 AND $3",
    column.sql()
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::sqlite::SqlitePool;

  #[test]
  fn data_archive_columns_are_whitelisted() {
    assert_eq!(DataArchiveColumn::CpuAvg.sql(), "cpu_avg");
    assert_eq!(DataArchiveColumn::CpuMax.sql(), "cpu_max");
    assert_eq!(DataArchiveColumn::CpuMin.sql(), "cpu_min");
    assert_eq!(DataArchiveColumn::RamAvg.sql(), "ram_avg");
    assert_eq!(DataArchiveColumn::RamMax.sql(), "ram_max");
    assert_eq!(DataArchiveColumn::RamMin.sql(), "ram_min");
  }

  #[test]
  fn gpu_archive_columns_are_whitelisted() {
    assert_eq!(GpuArchiveColumn::UsageAvg.sql(), "usage_avg");
    assert_eq!(GpuArchiveColumn::UsageMax.sql(), "usage_max");
    assert_eq!(GpuArchiveColumn::UsageMin.sql(), "usage_min");
    assert_eq!(GpuArchiveColumn::TemperatureAvg.sql(), "temperature_avg");
    assert_eq!(GpuArchiveColumn::TemperatureMax.sql(), "temperature_max");
    assert_eq!(GpuArchiveColumn::TemperatureMin.sql(), "temperature_min");
    assert_eq!(
      GpuArchiveColumn::DedicatedMemoryAvg.sql(),
      "dedicated_memory_avg"
    );
    assert_eq!(
      GpuArchiveColumn::DedicatedMemoryMax.sql(),
      "dedicated_memory_max"
    );
    assert_eq!(
      GpuArchiveColumn::DedicatedMemoryMin.sql(),
      "dedicated_memory_min"
    );
  }

  #[tokio::test]
  async fn data_archive_integer_values_decode_as_f64() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_avg INTEGER,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (id, cpu_avg, timestamp)
       VALUES (1, 42, '2026-06-08T00:00:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, ArchiveRecord>(&data_archive_select_sql(
      DataArchiveColumn::CpuAvg,
    ))
    .bind("2026-06-08T00:00:00.000Z")
    .bind("2026-06-08T00:01:00.000Z")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows[0].value, Some(42.0));
  }

  #[tokio::test]
  async fn gpu_archive_integer_values_decode_as_f64() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE GPU_DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        gpu_name TEXT,
        usage_avg INTEGER,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO GPU_DATA_ARCHIVE (id, gpu_name, usage_avg, timestamp)
       VALUES (1, 'GPU', 65, '2026-06-08T00:00:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, ArchiveRecord>(&gpu_archive_select_sql(
      GpuArchiveColumn::UsageAvg,
    ))
    .bind("GPU")
    .bind("2026-06-08T00:00:00.000Z")
    .bind("2026-06-08T00:01:00.000Z")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows[0].value, Some(65.0));
  }
}
