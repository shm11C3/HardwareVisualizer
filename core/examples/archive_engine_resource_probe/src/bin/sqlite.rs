use std::path::Path;

use archive_engine_resource_probe::{
  AMBIENT_QUERY, END_MS, END_TEXT, PROCESS_QUERY, START_MS, START_TEXT, ambient_rows,
  ambient_values, emit_and_wait, fnv_update, hash_field, parse_args, process_values,
  runtime_workers,
};
use serde_json::{Value, json};
use sqlx::sqlite::{
  SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous,
};
use sqlx::{Row, Sqlite, Transaction};

fn options(path: &Path, create: bool) -> SqliteConnectOptions {
  SqliteConnectOptions::new()
    .filename(path)
    .create_if_missing(create)
    .journal_mode(SqliteJournalMode::Wal)
    .busy_timeout(std::time::Duration::from_secs(5))
    .synchronous(SqliteSynchronous::Normal)
}

async fn schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
  sqlx::query(
    "CREATE TABLE PROCESS_STATS (
       id INTEGER PRIMARY KEY AUTOINCREMENT, pid INTEGER NOT NULL, process_name TEXT NOT NULL,
       cpu_usage REAL NOT NULL, memory_usage INTEGER NOT NULL,
       execution_sec INTEGER NOT NULL, timestamp DATETIME NOT NULL
     )",
  )
  .execute(pool)
  .await?;
  sqlx::query("CREATE INDEX idx_process_timestamp ON PROCESS_STATS(timestamp)")
    .execute(pool)
    .await?;
  sqlx::query(
    "CREATE TABLE AMBIENT_ARCHIVE (
       id INTEGER PRIMARY KEY AUTOINCREMENT, source TEXT NOT NULL, temperature REAL NOT NULL,
       humidity REAL, timestamp DATETIME NOT NULL, epoch_ms INTEGER NOT NULL
     )",
  )
  .execute(pool)
  .await?;
  sqlx::query("CREATE INDEX idx_ambient_epoch ON AMBIENT_ARCHIVE(epoch_ms, id)")
    .execute(pool)
    .await?;
  Ok(())
}

async fn seed_process(
  tx: &mut Transaction<'_, Sqlite>,
  seed_rows: u64,
) -> Result<(), sqlx::Error> {
  for id in 1..=seed_rows {
    let (pid, name, cpu, memory, execution, timestamp) = process_values(id);
    sqlx::query(
      "INSERT INTO PROCESS_STATS
       (id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
       VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id as i64)
    .bind(pid)
    .bind(name)
    .bind(cpu)
    .bind(memory)
    .bind(execution)
    .bind(timestamp)
    .execute(&mut **tx)
    .await?;
  }
  Ok(())
}

async fn seed_ambient(
  tx: &mut Transaction<'_, Sqlite>,
  process_rows: u64,
) -> Result<(), sqlx::Error> {
  for id in 1..=ambient_rows(process_rows) {
    let (source, temperature, humidity, timestamp, epoch_ms) = ambient_values(id);
    sqlx::query(
      "INSERT INTO AMBIENT_ARCHIVE
       (id, source, temperature, humidity, timestamp, epoch_ms)
       VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id as i64)
    .bind(source)
    .bind(temperature)
    .bind(humidity)
    .bind(timestamp)
    .bind(epoch_ms)
    .execute(&mut **tx)
    .await?;
  }
  Ok(())
}

async fn prepare(path: &Path, seed_rows: u64) -> Result<(), String> {
  if path.exists() {
    return Err(format!("refusing existing database: {}", path.display()));
  }
  if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(|error| error.to_string())?;
  }
  let pool = SqlitePool::connect_with(options(path, true))
    .await
    .map_err(|error| error.to_string())?;
  schema(&pool).await.map_err(|error| error.to_string())?;
  let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
  seed_process(&mut tx, seed_rows)
    .await
    .map_err(|error| error.to_string())?;
  seed_ambient(&mut tx, seed_rows)
    .await
    .map_err(|error| error.to_string())?;
  tx.commit().await.map_err(|error| error.to_string())?;
  sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
    .execute(&pool)
    .await
    .map_err(|error| error.to_string())?;
  pool.close().await;
  println!(
    "{}",
    json!({
      "prepared": true,
      "engine": "sqlite_sqlx",
      "database": path.display().to_string(),
      "seed_rows": seed_rows,
      "ambient_rows": ambient_rows(seed_rows),
    })
  );
  Ok(())
}

async fn configuration(pool: &SqlitePool) -> Result<Value, String> {
  let version: String = sqlx::query_scalar("SELECT sqlite_version()")
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
  let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
  let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
  let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
  Ok(json!({
    "profile": "release-size",
    "binding": "sqlx 0.8.6",
    "sqlite_version": version,
    "journal_mode": journal_mode,
    "synchronous": synchronous,
    "busy_timeout_ms": busy_timeout,
    "pool_size": pool.size(),
    "pool_num_idle": pool.num_idle(),
    "tokio_runtime": "multi_thread",
      "tokio_worker_threads": runtime_workers(),
    "handshake": "serde_json"
  }))
}

async fn run_queries(pool: &SqlitePool) -> Result<(u64, u64), String> {
  let mut hash = 0xcbf29ce484222325_u64;
  let mut result_rows = 0_u64;
  {
    let rows = sqlx::query(PROCESS_QUERY)
      .bind(START_TEXT)
      .bind(END_TEXT)
      .fetch_all(pool)
      .await
      .map_err(|error| error.to_string())?;
    result_rows += rows.len() as u64;
    for row in rows {
      let pid: i64 = row.try_get(0).map_err(|error| error.to_string())?;
      let name: String = row.try_get(1).map_err(|error| error.to_string())?;
      let avg_cpu: f64 = row.try_get(2).map_err(|error| error.to_string())?;
      let avg_memory: f64 = row.try_get(3).map_err(|error| error.to_string())?;
      let count: i64 = row.try_get(4).map_err(|error| error.to_string())?;
      let max_execution: i64 = row.try_get(5).map_err(|error| error.to_string())?;
      let latest: String = row.try_get(6).map_err(|error| error.to_string())?;
      hash = hash_field(hash, pid.to_le_bytes());
      hash = hash_field(hash, name);
      hash = hash_field(hash, format!("{avg_cpu:.9}"));
      hash = hash_field(hash, format!("{avg_memory:.9}"));
      hash = hash_field(hash, count.to_le_bytes());
      hash = hash_field(hash, max_execution.to_le_bytes());
      hash = hash_field(hash, latest);
    }
  }
  {
    let rows = sqlx::query(AMBIENT_QUERY)
      .bind(START_MS)
      .bind(END_MS)
      .fetch_all(pool)
      .await
      .map_err(|error| error.to_string())?;
    result_rows += rows.len() as u64;
    for row in rows {
      let id: i64 = row.try_get(0).map_err(|error| error.to_string())?;
      let source: String = row.try_get(1).map_err(|error| error.to_string())?;
      let temperature: f64 = row.try_get(2).map_err(|error| error.to_string())?;
      let humidity: Option<f64> = row.try_get(3).map_err(|error| error.to_string())?;
      let timestamp: String = row.try_get(4).map_err(|error| error.to_string())?;
      hash = hash_field(hash, id.to_le_bytes());
      hash = hash_field(hash, source);
      hash = hash_field(hash, temperature.to_bits().to_le_bytes());
      hash = match humidity {
        Some(value) => hash_field(hash, value.to_bits().to_le_bytes()),
        None => fnv_update(hash, &[0]),
      };
      hash = hash_field(hash, timestamp);
    }
  }
  Ok((result_rows, hash))
}

#[tokio::main]
async fn main() -> Result<(), String> {
  let args = parse_args()?;
  let path = args
    .database
    .as_deref()
    .ok_or("sqlite probe requires --database")?;
  if args.prepare {
    return prepare(path, args.seed_rows).await;
  }
  if !path.is_file() {
    return Err(format!("database does not exist: {}", path.display()));
  }

  let before = json!({
    "profile": "release-size",
    "binding": "sqlx 0.8.6",
    "journal_mode": "WAL",
    "synchronous": "NORMAL",
    "busy_timeout_ms": 5000,
    "pool_size": 0,
    "pool_num_idle": 0,
    "tokio_runtime": "multi_thread",
      "tokio_worker_threads": runtime_workers(),
    "handshake": "serde_json"
  });
  emit_and_wait(
    "sqlite_sqlx",
    "before_open",
    args.seed_rows,
    0,
    None,
    before,
  )?;

  let pool = SqlitePool::connect_with(options(path, false))
    .await
    .map_err(|error| error.to_string())?;
  let opened = configuration(&pool).await?;
  emit_and_wait(
    "sqlite_sqlx",
    if args.seed_rows == 0 {
      "open_empty"
    } else {
      "open_seeded"
    },
    args.seed_rows,
    0,
    None,
    opened,
  )?;

  let (result_rows, digest) = run_queries(&pool).await?;
  let post_query = configuration(&pool).await?;
  emit_and_wait(
    "sqlite_sqlx",
    "post_query",
    args.seed_rows,
    result_rows,
    Some(digest),
    post_query,
  )?;

  pool.close().await;
  emit_and_wait(
    "sqlite_sqlx",
    "after_close",
    args.seed_rows,
    result_rows,
    Some(digest),
    json!({
      "profile": "release-size",
      "binding": "sqlx 0.8.6",
      "pool_size": 0,
      "pool_num_idle": 0,
      "tokio_runtime": "multi_thread",
      "tokio_worker_threads": runtime_workers(),
      "handshake": "serde_json"
    }),
  )
}
