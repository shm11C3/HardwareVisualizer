use std::fs;
use std::path::{Path, PathBuf};

use archive_engine_resource_probe::{
  AMBIENT_QUERY, END_MS, END_TEXT, PROCESS_QUERY, START_MS, START_TEXT, ambient_rows,
  ambient_values, emit_and_wait, fnv_update, hash_field, parse_args, process_values,
  runtime_workers,
};
use duckdb::{Connection, params};
use serde_json::{Value, json};

fn sql_literal(value: &Path) -> String {
  value.to_string_lossy().replace(char::from(39), "''")
}

fn configure(connection: &Connection, temp_dir: &Path) -> Result<(), String> {
  fs::create_dir_all(temp_dir).map_err(|error| error.to_string())?;
  connection
    .execute_batch(&format!(
      "SET threads=2; SET memory_limit='128MB'; SET temp_directory='{}';",
      sql_literal(temp_dir)
    ))
    .map_err(|error| error.to_string())
}

fn schema(connection: &Connection) -> Result<(), String> {
  connection
    .execute_batch(
      "CREATE TABLE PROCESS_STATS (
         id BIGINT PRIMARY KEY, pid BIGINT NOT NULL, process_name VARCHAR NOT NULL,
         cpu_usage DOUBLE NOT NULL, memory_usage BIGINT NOT NULL,
         execution_sec BIGINT NOT NULL, timestamp VARCHAR NOT NULL
       );
       CREATE INDEX idx_process_timestamp ON PROCESS_STATS(timestamp);
       CREATE TABLE AMBIENT_ARCHIVE (
         id BIGINT PRIMARY KEY, source VARCHAR NOT NULL, temperature DOUBLE NOT NULL,
         humidity DOUBLE, timestamp VARCHAR NOT NULL, epoch_ms BIGINT NOT NULL
       );
       CREATE INDEX idx_ambient_epoch ON AMBIENT_ARCHIVE(epoch_ms, id);",
    )
    .map_err(|error| error.to_string())
}

fn prepare(path: &Path, seed_rows: u64) -> Result<(), String> {
  if path.exists() {
    return Err(format!("refusing existing database: {}", path.display()));
  }
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
  }
  let temp_dir = path.with_extension("prepare-temp");
  let connection = Connection::open(path).map_err(|error| error.to_string())?;
  configure(&connection, &temp_dir)?;
  schema(&connection)?;
  connection
    .execute_batch("BEGIN TRANSACTION")
    .map_err(|error| error.to_string())?;
  {
    let mut statement = connection
      .prepare(
        "INSERT INTO PROCESS_STATS
         (id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
      )
      .map_err(|error| error.to_string())?;
    for id in 1..=seed_rows {
      let (pid, name, cpu, memory, execution, timestamp) = process_values(id);
      statement
        .execute(params![
          id as i64, pid, name, cpu, memory, execution, timestamp
        ])
        .map_err(|error| error.to_string())?;
    }
  }
  {
    let mut statement = connection
      .prepare(
        "INSERT INTO AMBIENT_ARCHIVE
         (id, source, temperature, humidity, timestamp, epoch_ms)
         VALUES (?, ?, ?, ?, ?, ?)",
      )
      .map_err(|error| error.to_string())?;
    for id in 1..=ambient_rows(seed_rows) {
      let (source, temperature, humidity, timestamp, epoch_ms) = ambient_values(id);
      statement
        .execute(params![
          id as i64,
          source,
          temperature,
          humidity,
          timestamp,
          epoch_ms
        ])
        .map_err(|error| error.to_string())?;
    }
  }
  connection
    .execute_batch("COMMIT; CHECKPOINT")
    .map_err(|error| error.to_string())?;
  let version: String = connection
    .query_row("SELECT version()", [], |row| row.get(0))
    .map_err(|error| error.to_string())?;
  drop(connection);
  println!(
    "{}",
    json!({
      "prepared": true,
      "engine": "duckdb_bundled",
      "engine_version": version,
      "database": path.display().to_string(),
      "seed_rows": seed_rows,
      "ambient_rows": ambient_rows(seed_rows),
    })
  );
  Ok(())
}

fn configuration(connection: &Connection, temp_dir: &Path) -> Result<Value, String> {
  let version: String = connection
    .query_row("SELECT version()", [], |row| row.get(0))
    .map_err(|error| error.to_string())?;
  if version.trim_start_matches('v') != "1.5.5" {
    return Err(format!("expected DuckDB 1.5.5, got {version}"));
  }
  Ok(json!({
    "profile": "release-size",
    "binding": "duckdb-rs 1.10505.0 bundled",
    "engine_version": version,
    "threads": 2,
    "memory_limit": "128MB",
    "temp_directory": temp_dir.display().to_string(),
    "tokio_runtime": "multi_thread",
      "tokio_worker_threads": runtime_workers(),
    "handshake": "serde_json"
  }))
}

fn run_queries(connection: &Connection) -> Result<(u64, u64), String> {
  let mut hash = 0xcbf29ce484222325_u64;
  let mut result_rows = 0_u64;
  {
    let mut statement = connection
      .prepare(PROCESS_QUERY)
      .map_err(|error| error.to_string())?;
    let mut rows = statement
      .query(params![START_TEXT, END_TEXT])
      .map_err(|error| error.to_string())?;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
      let pid: i64 = row.get(0).map_err(|error| error.to_string())?;
      let name: String = row.get(1).map_err(|error| error.to_string())?;
      let avg_cpu: f64 = row.get(2).map_err(|error| error.to_string())?;
      let avg_memory: f64 = row.get(3).map_err(|error| error.to_string())?;
      let count: i64 = row.get(4).map_err(|error| error.to_string())?;
      let max_execution: i64 = row.get(5).map_err(|error| error.to_string())?;
      let latest: String = row.get(6).map_err(|error| error.to_string())?;
      result_rows += 1;
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
    let mut statement = connection
      .prepare(AMBIENT_QUERY)
      .map_err(|error| error.to_string())?;
    let mut rows = statement
      .query(params![START_MS, END_MS])
      .map_err(|error| error.to_string())?;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
      let id: i64 = row.get(0).map_err(|error| error.to_string())?;
      let source: String = row.get(1).map_err(|error| error.to_string())?;
      let temperature: f64 = row.get(2).map_err(|error| error.to_string())?;
      let humidity: Option<f64> = row.get(3).map_err(|error| error.to_string())?;
      let timestamp: String = row.get(4).map_err(|error| error.to_string())?;
      result_rows += 1;
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
    .ok_or("duckdb probe requires --database")?;
  if args.prepare {
    return prepare(path, args.seed_rows);
  }
  if !path.is_file() {
    return Err(format!("database does not exist: {}", path.display()));
  }

  let temp_dir = PathBuf::from(format!(
    "{}.resource-probe-{}-temp",
    path.display(),
    std::process::id()
  ));
  let before = json!({
    "profile": "release-size",
    "binding": "duckdb-rs 1.10505.0 bundled",
    "engine_version": "1.5.5",
    "threads": 2,
    "memory_limit": "128MB",
    "temp_directory": temp_dir.display().to_string(),
    "tokio_runtime": "multi_thread",
      "tokio_worker_threads": runtime_workers(),
    "handshake": "serde_json"
  });
  emit_and_wait(
    "duckdb_bundled",
    "before_open",
    args.seed_rows,
    0,
    None,
    before,
  )?;

  let connection = Connection::open(path).map_err(|error| error.to_string())?;
  configure(&connection, &temp_dir)?;
  emit_and_wait(
    "duckdb_bundled",
    if args.seed_rows == 0 {
      "open_empty"
    } else {
      "open_seeded"
    },
    args.seed_rows,
    0,
    None,
    configuration(&connection, &temp_dir)?,
  )?;

  let (result_rows, digest) = run_queries(&connection)?;
  emit_and_wait(
    "duckdb_bundled",
    "post_query",
    args.seed_rows,
    result_rows,
    Some(digest),
    configuration(&connection, &temp_dir)?,
  )?;

  drop(connection);
  emit_and_wait(
    "duckdb_bundled",
    "after_close",
    args.seed_rows,
    result_rows,
    Some(digest),
    json!({
      "profile": "release-size",
      "binding": "duckdb-rs 1.10505.0 bundled",
      "engine_version": "1.5.5",
      "threads": 2,
      "memory_limit": "128MB",
      "temp_directory": temp_dir.display().to_string(),
      "connection_open": false,
      "tokio_runtime": "multi_thread",
      "tokio_worker_threads": runtime_workers(),
      "handshake": "serde_json"
    }),
  )
}
