#![cfg(feature = "duckdb-archive")]

use std::path::{Path, PathBuf};

use duckdb::{AccessMode, Config, Connection};
use hardviz_core::infrastructure::database::candidate_database::{
  CandidateError, CandidateReport, build_candidate_database,
};
use hardviz_core::infrastructure::database::{
  db, gpu_archive, hardware_archive, migrate,
};
use hardviz_core::persistence::archive_data::{
  GpuData, HardwareArchiveRow, HardwareData,
};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{
  SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions,
};
use sqlx::{ConnectOptions, Executor, Row};
use tempfile::TempDir;

#[path = "../../src-tauri/src/infrastructure/database/migration.rs"]
mod app_migrations;

const STORAGE_ID: &str = "storage:hmac-sha256:v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct Fixture {
  _directory: TempDir,
  source: PathBuf,
  destination: PathBuf,
  rows: u64,
  source_hash: String,
  migration_checksum: Vec<u8>,
}

#[tokio::test]
async fn copies_every_app_table_and_canonical_cell_exactly() {
  let fixture = seeded_source().await;
  let report = copy(&fixture).await.unwrap();

  assert_report(&report, fixture.rows, &fixture.destination);
  assert_eq!(file_hash(&fixture.source), fixture.source_hash);
  assert!(fixture.destination.is_file());
  assert_native_cells(&fixture);
  assert_no_candidate_workdirs(&fixture);
}

#[tokio::test]
async fn production_archive_writers_create_copyable_fractional_cells() {
  // This is the only integration test that initializes Core's process-wide
  // database path. Every other candidate fixture uses an isolated pool.
  let directory = tempfile::tempdir().unwrap();
  let source = directory.path().join("production-writers.sqlite3");
  let destination = directory.path().join("candidate.duckdb");
  assert!(db::init(source.clone()));
  migrate::run(app_migrations::get_migrations())
    .await
    .unwrap();
  let timestamp = "2026-09-01T00:00:00Z"
    .parse::<chrono::DateTime<chrono::Utc>>()
    .unwrap();

  hardware_archive::insert(
    HardwareArchiveRow {
      cpu: hardware_data(Some(25.25), Some(50.5), Some(0.0)),
      memory: hardware_data(Some(25.25), Some(75.5), Some(10.25)),
      cpu_temperature: hardware_data(Some(40.25), Some(55.5), Some(35.0)),
      cpu_power: hardware_data(Some(12.25), Some(30.5), Some(5.0)),
      gpu_power: hardware_data(None, None, None),
      ane_power: hardware_data(None, None, None),
      package_power: hardware_data(Some(15.25), Some(35.5), Some(6.0)),
    },
    timestamp,
  )
  .await
  .unwrap();
  gpu_archive::insert(
    GpuData {
      gpu_id: Some("gpu-persisted-name".to_owned()),
      gpu_name: "Apple M4 Max".to_owned(),
      usage_avg: Some(25.25),
      usage_max: Some(50.5),
      usage_min: Some(0.0),
      temperature_avg: Some(40.25),
      temperature_max: Some(55),
      temperature_min: Some(35),
      dedicated_memory_avg: Some(1024),
      dedicated_memory_max: Some(2048),
      dedicated_memory_min: Some(512),
    },
    timestamp,
  )
  .await
  .unwrap();

  let pool = open_pool(&source, false).await;
  let storage_classes: (String, String, String) = sqlx::query_as(
    "SELECT typeof(cpu_avg),typeof(ram_avg), \
     (SELECT typeof(usage_avg) FROM GPU_DATA_ARCHIVE) FROM DATA_ARCHIVE",
  )
  .fetch_one(&pool)
  .await
  .unwrap();
  assert_eq!(
    storage_classes,
    ("real".to_owned(), "real".to_owned(), "real".to_owned())
  );
  let source_timestamp: String =
    sqlx::query_scalar("SELECT CAST(timestamp AS TEXT) FROM DATA_ARCHIVE")
      .fetch_one(&pool)
      .await
      .unwrap();
  pool.close().await;
  let source_hash = file_hash(&source);

  let report =
    build_candidate_database(&source, &destination, app_migrations::get_migrations())
      .await
      .unwrap();
  assert_eq!(report.tables.len(), 17);
  assert_eq!(report.total_rows, 25);
  assert_eq!(file_hash(&source), source_hash);
  let connection = readonly_candidate(&destination);
  let hardware: (String, f64, f64, String) = connection
    .query_row(
      "SELECT typeof(cpu_avg),cpu_avg,ram_avg,timestamp FROM DATA_ARCHIVE",
      [],
      |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .unwrap();
  assert_eq!(hardware.0, "DOUBLE");
  assert_eq!(hardware.1.to_bits(), f64::from(25.25_f32).to_bits());
  assert_eq!(hardware.2.to_bits(), f64::from(25.25_f32).to_bits());
  assert_eq!(hardware.3, source_timestamp);
  let gpu_usage: f64 = connection
    .query_row("SELECT usage_avg FROM GPU_DATA_ARCHIVE", [], |row| {
      row.get(0)
    })
    .unwrap();
  assert_eq!(gpu_usage.to_bits(), f64::from(25.25_f32).to_bits());
}

#[tokio::test]
async fn empty_integer_columns_keep_bigint_storage() {
  let directory = tempfile::tempdir().unwrap();
  let source = directory.path().join("empty.sqlite3");
  let destination = directory.path().join("candidate.duckdb");
  let pool = open_pool(&source, true).await;
  migrate::run_on_pool(&pool, app_migrations::get_migrations())
    .await
    .unwrap();
  pool.close().await;

  build_candidate_database(&source, &destination, app_migrations::get_migrations())
    .await
    .unwrap();
  let connection = readonly_candidate(&destination);
  let types = connection
    .prepare(
      "SELECT table_name,column_name,data_type FROM information_schema.columns \
       WHERE (table_name='DATA_ARCHIVE' AND column_name='cpu_avg') \
          OR (table_name='sqlite_sequence' AND column_name='seq') \
       ORDER BY table_name,column_name",
    )
    .unwrap()
    .query_map([], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
      ))
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();
  assert_eq!(
    types,
    vec![
      (
        "DATA_ARCHIVE".to_owned(),
        "cpu_avg".to_owned(),
        "BIGINT".to_owned()
      ),
      (
        "sqlite_sequence".to_owned(),
        "seq".to_owned(),
        "BIGINT".to_owned()
      ),
    ]
  );
}

#[tokio::test]
async fn mixed_numeric_staging_preserves_multiple_batches_generically() {
  let directory = tempfile::tempdir().unwrap();
  let source = directory.path().join("mixed.sqlite3");
  let destination = directory.path().join("candidate.duckdb");
  let migrations = vec![migrate::SchemaMigration {
    version: 1,
    description: "create_generic_mixed_numeric",
    sql: "CREATE TABLE generic_numeric (id INTEGER PRIMARY KEY, value INTEGER);",
  }];
  let pool = open_pool(&source, true).await;
  migrate::run_on_pool(&pool, migrations.clone())
    .await
    .unwrap();
  pool
    .execute(
      "WITH RECURSIVE seq(n) AS (VALUES(0) UNION ALL SELECT n+1 FROM seq WHERE n<512) \
     INSERT INTO generic_numeric(id,value) SELECT n,n FROM seq",
    )
    .await
    .unwrap();
  sqlx::query("UPDATE generic_numeric SET value=? WHERE id=0")
    .bind(i64::MIN)
    .execute(&pool)
    .await
    .unwrap();
  sqlx::query("UPDATE generic_numeric SET value=? WHERE id=510")
    .bind(i64::MAX)
    .execute(&pool)
    .await
    .unwrap();
  for (id, value) in [(511_i64, 25.25_f32), (512, 50.5_f32)] {
    sqlx::query("UPDATE generic_numeric SET value=? WHERE id=?")
      .bind(value)
      .bind(id)
      .execute(&pool)
      .await
      .unwrap();
  }
  let classes: Vec<String> = sqlx::query_scalar(
    "SELECT typeof(value) FROM generic_numeric WHERE id IN (0,510,511,512) ORDER BY id",
  )
  .fetch_all(&pool)
  .await
  .unwrap();
  assert_eq!(classes, ["integer", "integer", "real", "real"]);
  pool.close().await;
  let source_hash = file_hash(&source);

  let report = build_candidate_database(&source, &destination, migrations)
    .await
    .unwrap();
  let table = report
    .tables
    .iter()
    .find(|table| table.name == "generic_numeric")
    .unwrap();
  assert_eq!(table.source_rows, 513);
  assert_eq!(table.reopened_rows, 513);
  assert_eq!(table.source_sha256, table.reopened_sha256);
  assert_eq!(report.total_rows, 514);
  assert_eq!(file_hash(&source), source_hash);

  let connection = readonly_candidate(&destination);
  let data_type: String = connection
    .query_row(
      "SELECT data_type FROM information_schema.columns \
       WHERE table_name='generic_numeric' AND column_name='value'",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert_eq!(data_type, "UNION(i BIGINT, r DOUBLE)");
  for (id, tag, integer, real) in [
    (0_i64, "i", Some(i64::MIN), None),
    (510, "i", Some(i64::MAX), None),
    (511, "r", None, Some(f64::from(25.25_f32))),
    (512, "r", None, Some(f64::from(50.5_f32))),
  ] {
    let actual: (String, Option<i64>, Option<f64>) = connection
      .query_row(
        "SELECT CAST(union_tag(value) AS VARCHAR),union_extract(value,'i'), \
         union_extract(value,'r') FROM generic_numeric WHERE id=?",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
      )
      .unwrap();
    assert_eq!(actual.0, tag);
    assert_eq!(actual.1, integer);
    assert_eq!(actual.2.map(f64::to_bits), real.map(f64::to_bits));
  }
  drop(connection);
  assert_no_candidate_workdirs_in(directory.path());
}

#[tokio::test]
async fn copies_a_consistent_snapshot_while_a_source_write_is_uncommitted() {
  let fixture = seeded_source().await;
  let writer = open_pool(&fixture.source, false).await;
  let mut transaction = writer.begin().await.unwrap();
  sqlx::query(
    "INSERT INTO PROCESS_STATS VALUES \
     (901,7777,'concurrent-write',1.25,4096,60,'2026-09-02T00:01:00+00:00')",
  )
  .execute(&mut *transaction)
  .await
  .unwrap();

  let report = copy(&fixture).await.unwrap();
  assert_report(&report, fixture.rows, &fixture.destination);
  transaction.commit().await.unwrap();
  let committed_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM PROCESS_STATS")
    .fetch_one(&writer)
    .await
    .unwrap();
  assert_eq!(committed_rows, 514);
  writer.close().await;
  assert_no_candidate_workdirs(&fixture);
}

#[tokio::test]
async fn refuses_existing_and_source_destinations_without_mutation() {
  let fixture = seeded_source().await;
  let sentinel = b"existing candidate must survive";
  std::fs::write(&fixture.destination, sentinel).unwrap();
  let error = copy(&fixture).await.unwrap_err();
  assert!(matches!(error, CandidateError::DestinationExists { .. }));
  assert_eq!(std::fs::read(&fixture.destination).unwrap(), sentinel);
  assert_no_candidate_workdirs(&fixture);

  let error = build_candidate_database(
    &fixture.source,
    &fixture.source,
    app_migrations::get_migrations(),
  )
  .await
  .unwrap_err();
  assert!(matches!(error, CandidateError::DestinationExists { .. }));
  assert_eq!(file_hash(&fixture.source), fixture.source_hash);
  assert_no_candidate_workdirs(&fixture);
}

#[cfg(unix)]
#[tokio::test]
async fn refuses_a_broken_destination_symlink() {
  let fixture = seeded_source().await;
  std::os::unix::fs::symlink("missing.duckdb", &fixture.destination).unwrap();
  let error = copy(&fixture).await.unwrap_err();
  assert!(matches!(error, CandidateError::DestinationExists { .. }));
  assert!(std::fs::symlink_metadata(&fixture.destination).is_ok());
  assert_no_candidate_workdirs(&fixture);
}

#[tokio::test]
async fn refuses_schema_drift_noncanonical_cells_and_invalid_utf8() {
  assert_refused(
    "ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN unexpected TEXT",
    |error| matches!(error, CandidateError::SchemaMismatch { .. }),
  )
  .await;
  assert_refused(
    "UPDATE DATA_ARCHIVE SET cpu_avg = 'wrong-class' WHERE id = 0",
    |error| matches!(error, CandidateError::NonCanonicalCell { .. }),
  )
  .await;
  assert_refused(
    "UPDATE GPU_DATA_ARCHIVE SET gpu_name = CAST(X'FF' AS TEXT) WHERE id = 1",
    |error| matches!(error, CandidateError::InvalidUtf8 { .. }),
  )
  .await;
  assert_refused(
    "UPDATE GPU_DATA_ARCHIVE SET gpu_name = CAST(zeroblob(1048577) AS TEXT) WHERE id = 1",
    |error| matches!(error, CandidateError::CellTooLarge { .. }),
  )
  .await;
}

async fn copy(fixture: &Fixture) -> Result<CandidateReport, CandidateError> {
  build_candidate_database(
    &fixture.source,
    &fixture.destination,
    app_migrations::get_migrations(),
  )
  .await
}

async fn assert_refused(sql: &str, expected: impl FnOnce(&CandidateError) -> bool) {
  let fixture = seeded_source().await;
  let pool = open_pool(&fixture.source, false).await;
  sqlx::query(sql).execute(&pool).await.unwrap();
  pool.close().await;
  let changed_hash = file_hash(&fixture.source);

  let error = copy(&fixture).await.unwrap_err();
  assert!(expected(&error), "unexpected error: {error:?}");
  assert!(!fixture.destination.exists());
  assert_eq!(file_hash(&fixture.source), changed_hash);
  assert_no_candidate_workdirs(&fixture);
}

async fn seeded_source() -> Fixture {
  let directory = tempfile::tempdir().unwrap();
  let source = directory.path().join("source.sqlite3");
  let destination = directory.path().join("candidate.duckdb");
  let pool = open_pool(&source, true).await;
  migrate::run_on_pool(&pool, app_migrations::get_migrations())
    .await
    .unwrap();
  seed_domain_tables(&pool).await;
  seed_deleted_identity_high_waters(&pool).await;
  let rows = count_all_rows(&pool).await;
  let migration_checksum: Vec<u8> =
    sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 1")
      .fetch_one(&pool)
      .await
      .unwrap();
  pool.close().await;

  Fixture {
    source_hash: file_hash(&source),
    _directory: directory,
    source,
    destination,
    rows,
    migration_checksum,
  }
}

async fn open_pool(path: &Path, create: bool) -> SqlitePool {
  let options = SqliteConnectOptions::new()
    .filename(path)
    .create_if_missing(create)
    .foreign_keys(true)
    .journal_mode(SqliteJournalMode::Wal)
    .disable_statement_logging();
  SqlitePoolOptions::new()
    .max_connections(1)
    .connect_with(options)
    .await
    .unwrap()
}

async fn seed_domain_tables(pool: &SqlitePool) {
  // Record ids remain separate from domain identity: Process is (pid, name),
  // Storage Health shares the producer's storage key, and ambient/fan rows
  // preserve their source labels without translating namespaces.
  pool.execute(
    r#"
    INSERT INTO DATA_ARCHIVE
      (id,cpu_avg,cpu_max,cpu_min,ram_avg,ram_max,ram_min,timestamp,
       cpu_temperature_avg,cpu_temperature_max,cpu_temperature_min,
       cpu_power_avg,cpu_power_max,cpu_power_min,package_power_avg)
    VALUES
      (-9223372036854775808,-9223372036854775808,0,9223372036854775807,
       NULL,42,NULL,'2026-09-01T00:00:00+00:00',40.25,NULL,55.5,10.125,NULL,20.5,30.25),
      (-1,NULL,2,0,NULL,4,NULL,'2026-09-01T00:01:00+00:00',NULL,NULL,NULL,NULL,NULL,NULL,NULL),
      (0,NULL,40,10,NULL,60,NULL,'2026-09-01T00:02:00+00:00',
       45.12500000000001,50.0,40.0,-0.0,15.0,5.0,16.0),
      (1,0,0,0,NULL,0,NULL,'2026-09-01T00:03:00+00:00',
       NULL,NULL,NULL,NULL,NULL,NULL,NULL),
      (9223372036854775807,9223372036854775807,NULL,NULL,NULL,NULL,NULL,NULL,
       NULL,NULL,NULL,NULL,NULL,NULL,NULL);
    INSERT INTO GPU_DATA_ARCHIVE
      (id,gpu_name,usage_avg,usage_max,usage_min,temperature_avg,
       temperature_max,temperature_min,timestamp,dedicated_memory_avg,gpu_id)
    VALUES
      (1,'Apple M4 Max',NULL,50,5,42,55,35,'2026-09-01T00:00:00+00:00',1024,'gpu-persisted-name'),
      (2,'Discrete GPU'||char(0)||'Secondary',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
    WITH RECURSIVE seq(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM seq WHERE n<513)
    INSERT INTO PROCESS_STATS(pid,process_name,cpu_usage,memory_usage,execution_sec,timestamp)
    SELECT 4000+(n%17), CASE WHEN n=1 THEN 'renderer'||char(0)||'helper'
      ELSE 'process-'||printf('%02d',n%23) END, (n%401)*0.25,
      CASE WHEN n=2 THEN 9223372036854775807 ELSE n*4096 END, n*60,
      printf('2026-09-01T%02d:%02d:00+00:00',(n/60)%24,n%60) FROM seq;
    INSERT INTO storage_devices
      (id,display_name,model,serial_hash,protocol,capacity_bytes,first_seen_at,last_seen_at,is_active)
    VALUES
      ('storage:hmac-sha256:v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
       'System SSD','NVMe Model','serial-hash','NVMe',9223372036854775807,'2026-01-01','2026-09-01',1);
    INSERT INTO storage_health_daily_records
      (device_id,date,health_status,temperature_celsius,power_on_hours,percentage_used,collected_at)
    VALUES
      ('storage:hmac-sha256:v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
       '2026-09-01','healthy',38.125,12345,2.5,'2026-09-01T23:59:00+00:00');
    INSERT INTO cooling_daily_summary
      (date,idle_cpu_temperature_avg,idle_sample_minutes,low_cpu_temperature_avg,
       low_sample_minutes,mid_sample_minutes,high_cpu_temperature_avg,
       high_sample_minutes,coverage_minutes,cpu_power_avg,power_sample_minutes)
    VALUES ('2026-09-01',35.25,240,45.5,360,0,75.125,120,720,22.25,600);
    INSERT INTO cooling_baseline VALUES
      (1,'2026-08-01','2026-08-30',36.25,7200,'2026-09-01T00:00:00+00:00');
    INSERT INTO cooling_hourly_summary VALUES
      ('2026-09-01T00:00:00+00:00',12.25,41.5,60);
    INSERT INTO AMBIENT_ARCHIVE(source,temperature,humidity,timestamp) VALUES
      ('SwitchBot Meter (a1b2)',24.125,45.5,'2026-09-01T00:00:00+00:00'),
      ('SwitchBot Meter (c3d4)',23.75,NULL,'2026-09-01T00:01:00+00:00');
    INSERT INTO FAN_ARCHIVE VALUES
      (0,'CPU Fan',0,'2026-09-01T00:00:00+00:00'),
      (-1,'System Fan',1400,'2026-09-01T00:01:00+00:00');
    INSERT INTO cooling_fan_daily_summary VALUES
      ('2026-09-01','CPU Fan',1250.25,2200,0,720);
    INSERT INTO cooling_delta_baseline VALUES
      (1,'SwitchBot Meter (a1b2)','2026-08-01','2026-08-30',12.125,7200,
       '2026-09-01T00:00:00+00:00');
    INSERT INTO cooling_thermal_delta_daily_summary
      (date,source,coverage_minutes,idle_delta_temperature_avg,idle_delta_sample_minutes,
       low_delta_temperature_avg,low_delta_sample_minutes,mid_delta_sample_minutes,
       high_delta_temperature_avg,high_delta_sample_minutes)
    VALUES ('2026-09-01','SwitchBot Meter (a1b2)',700,10.0,200,20.0,300,0,50.0,200);
    INSERT INTO cooling_covariate_daily_summary
      (date,source,band,sample_minutes,band_share,ambient_temperature_median,
       delta_minutes,delta_temperature_median,power_minutes,cpu_power_median,
       power_fit_n,power_fit_sum_x,power_fit_sum_y,power_fit_sum_xy,power_fit_sum_xx,power_fit_sum_yy)
    VALUES ('2026-09-01','SwitchBot Meter (a1b2)','low',300,0.25,24.125,
       300,20.5,280,22.25,280,6230.0,5740.0,127755.0,140000.0,120000.0);
    INSERT INTO cooling_fan_covariate_daily_summary VALUES
      ('2026-09-01','SwitchBot Meter (a1b2)','CPU Fan','low',280,1250.25,
       280,350000.0,5740.0,7175000.0,438000000.0,120000.0);
    "#,
  )
  .await
  .unwrap();

  // These are the same Option<f32> SQLx bindings used by the production
  // Hardware and GPU archive writers. INTEGER affinity stores fractional
  // readings as SQLite REAL cells, so the candidate must preserve both tags.
  sqlx::query("UPDATE DATA_ARCHIVE SET cpu_avg = ?, ram_avg = ? WHERE id = 0")
    .bind(Some(25.25_f32))
    .bind(Some(25.25_f32))
    .execute(pool)
    .await
    .unwrap();
  sqlx::query("UPDATE GPU_DATA_ARCHIVE SET usage_avg = ? WHERE id = 1")
    .bind(Some(25.25_f32))
    .execute(pool)
    .await
    .unwrap();

  let cpu_avg_classes: Vec<(i64, String)> =
    sqlx::query("SELECT id,typeof(cpu_avg) FROM DATA_ARCHIVE ORDER BY id")
      .fetch_all(pool)
      .await
      .unwrap()
      .into_iter()
      .map(|row| (row.get(0), row.get(1)))
      .collect();
  assert_eq!(
    cpu_avg_classes,
    vec![
      (i64::MIN, "integer".to_owned()),
      (-1, "null".to_owned()),
      (0, "real".to_owned()),
      (1, "integer".to_owned()),
      (i64::MAX, "integer".to_owned()),
    ]
  );
  let stored_health_id: String =
    sqlx::query_scalar("SELECT device_id FROM storage_health_daily_records")
      .fetch_one(pool)
      .await
      .unwrap();
  assert_eq!(stored_health_id, STORAGE_ID);
}

async fn seed_deleted_identity_high_waters(pool: &SqlitePool) {
  pool.execute(
    r#"
    INSERT INTO PROCESS_STATS VALUES
      (900,9999,'deleted-process',0.0,0,0,'2026-09-02T00:00:00+00:00');
    DELETE FROM PROCESS_STATS WHERE id=900;
    INSERT INTO AMBIENT_ARCHIVE VALUES
      (700,'deleted ambient',0.0,NULL,'2026-09-02T00:00:00+00:00');
    DELETE FROM AMBIENT_ARCHIVE WHERE id=700;
    INSERT INTO storage_health_daily_records(id,device_id,date,health_status,collected_at)
    VALUES (80,
      'storage:hmac-sha256:v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      '2026-09-02','healthy','2026-09-02T23:59:00+00:00');
    DELETE FROM storage_health_daily_records WHERE id=80;
    "#,
  )
  .await
  .unwrap();

  let sequences: Vec<(String, i64)> = sqlx::query("SELECT name,seq FROM sqlite_sequence")
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect();
  for expected in [
    ("PROCESS_STATS".to_owned(), 900),
    ("AMBIENT_ARCHIVE".to_owned(), 700),
    ("storage_health_daily_records".to_owned(), 80),
  ] {
    assert!(sequences.contains(&expected));
  }
}

async fn count_all_rows(pool: &SqlitePool) -> u64 {
  let tables: Vec<String> =
    sqlx::query_scalar("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
      .fetch_all(pool)
      .await
      .unwrap();
  assert_eq!(tables.len(), 17);
  let mut total = 0_u64;
  for table in tables {
    let sql = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\""));
    let count: i64 = sqlx::query_scalar(&sql).fetch_one(pool).await.unwrap();
    total += u64::try_from(count).unwrap();
  }
  assert_eq!(total, 560);
  total
}

fn assert_report(report: &CandidateReport, rows: u64, destination: &Path) {
  assert_eq!(report.candidate_database_path, destination);
  assert_eq!(report.snapshot_kind, "immutable_source_snapshot");
  assert_eq!(report.source_migration_max_version, 23);
  assert_eq!(report.source_migration_count, 23);
  assert_eq!(report.tables.len(), 17);
  assert_eq!(report.total_rows, rows);
  assert!(report.candidate_bytes > 0);
  assert_eq!(report.source_schema_sha256.len(), 64);
  assert_eq!(
    report
      .tables
      .iter()
      .map(|table| table.source_rows)
      .sum::<u64>(),
    rows
  );
  for table in &report.tables {
    assert_eq!(table.source_rows, table.reopened_rows, "{}", table.name);
    assert_eq!(table.source_sha256, table.reopened_sha256, "{}", table.name);
  }
  let data = report
    .tables
    .iter()
    .find(|table| table.name == "DATA_ARCHIVE")
    .unwrap();
  assert_eq!(data.source_rows, 5);
  let process = report
    .tables
    .iter()
    .find(|table| table.name == "PROCESS_STATS")
    .unwrap();
  assert_eq!(process.source_rows, 513);
}

fn hardware_data(avg: Option<f32>, max: Option<f32>, min: Option<f32>) -> HardwareData {
  HardwareData { avg, max, min }
}

fn readonly_candidate(path: &Path) -> Connection {
  let config = Config::default().access_mode(AccessMode::ReadOnly).unwrap();
  Connection::open_with_flags(path, config).unwrap()
}

fn file_hash(path: &Path) -> String {
  format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

fn assert_no_candidate_workdirs(fixture: &Fixture) {
  assert_no_candidate_workdirs_in(fixture._directory.path());
}

fn assert_no_candidate_workdirs_in(directory: &Path) {
  let leftovers = std::fs::read_dir(directory)
    .unwrap()
    .map(|entry| entry.unwrap().file_name())
    .filter(|name| {
      name
        .to_string_lossy()
        .starts_with(".hardwarevisualizer-duckdb-candidate-")
    })
    .collect::<Vec<_>>();
  assert!(
    leftovers.is_empty(),
    "candidate workdirs remain: {leftovers:?}"
  );
}

fn assert_native_cells(fixture: &Fixture) {
  let connection = readonly_candidate(&fixture.destination);

  let tables = connection
    .prepare(
      "SELECT table_name FROM information_schema.tables \
       WHERE table_schema = 'main' ORDER BY table_name",
    )
    .unwrap()
    .query_map([], |row| row.get::<_, String>(0))
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();
  assert_eq!(tables.len(), 18);
  for table in [
    "DATA_ARCHIVE",
    "GPU_DATA_ARCHIVE",
    "PROCESS_STATS",
    "storage_devices",
    "storage_health_daily_records",
    "cooling_daily_summary",
    "cooling_baseline",
    "cooling_hourly_summary",
    "AMBIENT_ARCHIVE",
    "FAN_ARCHIVE",
    "cooling_fan_daily_summary",
    "cooling_delta_baseline",
    "cooling_thermal_delta_daily_summary",
    "cooling_covariate_daily_summary",
    "cooling_fan_covariate_daily_summary",
    "_sqlx_migrations",
    "sqlite_sequence",
    "__hv_snapshot_metadata",
  ] {
    assert!(tables.iter().any(|candidate| candidate == table), "{table}");
  }

  let minimum = read_cpu_avg_union(&connection, i64::MIN);
  assert_eq!(minimum, (Some("i".to_owned()), Some(i64::MIN), None));
  let zero = read_cpu_avg_union(&connection, 1);
  assert_eq!(zero, (Some("i".to_owned()), Some(0), None));
  let maximum = read_cpu_avg_union(&connection, i64::MAX);
  assert_eq!(maximum, (Some("i".to_owned()), Some(i64::MAX), None));
  let real = read_cpu_avg_union(&connection, 0);
  assert_eq!(real.0.as_deref(), Some("r"));
  assert_eq!(real.1, None);
  assert_eq!(real.2.unwrap().to_bits(), f64::from(25.25_f32).to_bits());
  let exact_real: f64 = connection
    .query_row(
      "SELECT cpu_temperature_avg FROM DATA_ARCHIVE WHERE id=0",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert_eq!(exact_real.to_bits(), 45.12500000000001_f64.to_bits());
  let null_union: bool = connection
    .query_row(
      "SELECT cpu_avg IS NULL FROM DATA_ARCHIVE WHERE id=-1",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert!(null_union);

  let real_only: (String, f64) = connection
    .query_row(
      "SELECT typeof(ram_avg),ram_avg FROM DATA_ARCHIVE WHERE id=0",
      [],
      |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap();
  assert_eq!(real_only.0, "DOUBLE");
  assert_eq!(real_only.1.to_bits(), f64::from(25.25_f32).to_bits());
  let gpu_real_only: (String, f64) = connection
    .query_row(
      "SELECT typeof(usage_avg),usage_avg FROM GPU_DATA_ARCHIVE WHERE id=1",
      [],
      |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap();
  assert_eq!(gpu_real_only.0, "DOUBLE");
  assert_eq!(gpu_real_only.1.to_bits(), f64::from(25.25_f32).to_bits());
  let all_null_integer_type: String = connection
    .query_row(
      "SELECT data_type FROM information_schema.columns \
       WHERE table_name='GPU_DATA_ARCHIVE' AND column_name='dedicated_memory_min'",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert_eq!(all_null_integer_type, "BIGINT");
  let integer_only_type: String = connection
    .query_row(
      "SELECT data_type FROM information_schema.columns \
       WHERE table_name='GPU_DATA_ARCHIVE' AND column_name='usage_max'",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert_eq!(integer_only_type, "BIGINT");
  let nul_text: String = connection
    .query_row(
      "SELECT gpu_name FROM GPU_DATA_ARCHIVE WHERE id=2",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert_eq!(nul_text.as_bytes(), b"Discrete GPU\0Secondary");
  let checksum: Vec<u8> = connection
    .query_row(
      "SELECT checksum FROM _sqlx_migrations WHERE version=1",
      [],
      |row| row.get(0),
    )
    .unwrap();
  assert_eq!(checksum, fixture.migration_checksum);
  let sequences = connection
    .prepare("SELECT name,seq FROM sqlite_sequence ORDER BY name")
    .unwrap()
    .query_map([], |row| {
      Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();
  for expected in [
    ("PROCESS_STATS".to_owned(), 900),
    ("AMBIENT_ARCHIVE".to_owned(), 700),
    ("storage_health_daily_records".to_owned(), 80),
  ] {
    assert!(sequences.contains(&expected));
  }
}

fn read_cpu_avg_union(
  connection: &Connection,
  id: i64,
) -> (Option<String>, Option<i64>, Option<f64>) {
  connection
    .query_row(
      "SELECT CAST(union_tag(cpu_avg) AS VARCHAR), \
       union_extract(cpu_avg,'i'),union_extract(cpu_avg,'r') \
       FROM DATA_ARCHIVE WHERE id=?",
      [id],
      |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .unwrap()
}
