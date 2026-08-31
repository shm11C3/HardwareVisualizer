//! Canonical table definitions for Core's own tests.
//!
//! Every module that exercises a query against an in-memory database used
//! to carry its own hand-written `CREATE TABLE` copy of the shipped
//! migration. Five copies of `cooling_daily_summary` meant that adding a
//! column (#2045 added seventeen) broke unrelated test modules one at a
//! time, and - worse - that a copy could silently drift from the shipped
//! schema and keep passing.
//!
//! The DDL still lives here rather than being read from the migration set
//! itself: the migrations are owned by App
//! (`src-tauri/src/infrastructure/database/migration.rs`), which depends
//! on Core rather than the other way round. `src-tauri`'s own
//! `shipped_migrations_*` tests are what pin these definitions to the
//! real ones - they apply the actual migration set and insert against the
//! result.

/// `DATA_ARCHIVE` as the cooling queries read it: the columns migrations
/// 1, 9 and 10 leave behind. Deliberately not the full archive table -
/// the GPU and non-CPU power columns are irrelevant here and only make
/// the fixtures noisier.
pub(crate) const DATA_ARCHIVE_DDL: &str = "CREATE TABLE DATA_ARCHIVE (
  id INTEGER PRIMARY KEY,
  cpu_avg REAL,
  cpu_temperature_avg REAL,
  cpu_temperature_max REAL,
  cpu_temperature_min REAL,
  cpu_power_avg REAL,
  cpu_power_max REAL,
  cpu_power_min REAL,
  timestamp DATETIME
)";

/// The `DATA_ARCHIVE` timestamp index (migration 19). Separate from
/// [`DATA_ARCHIVE_DDL`] so a test can create the table without it and
/// show a query plan changing when it is added.
pub(crate) const DATA_ARCHIVE_TIMESTAMP_INDEX_DDL: &str =
  "CREATE INDEX idx_data_archive_timestamp ON DATA_ARCHIVE(timestamp)";

/// `AMBIENT_ARCHIVE` as migration 15 creates it (#2043): row-per-source,
/// temperature NOT NULL, humidity nullable.
pub(crate) const AMBIENT_ARCHIVE_DDL: &str = "CREATE TABLE AMBIENT_ARCHIVE (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source TEXT NOT NULL,
  temperature REAL NOT NULL,
  humidity REAL,
  timestamp DATETIME NOT NULL
)";

/// The `AMBIENT_ARCHIVE` timestamp index (migration 15).
pub(crate) const AMBIENT_ARCHIVE_TIMESTAMP_INDEX_DDL: &str =
  "CREATE INDEX idx_ambient_archive_timestamp ON AMBIENT_ARCHIVE(timestamp)";

/// `cooling_daily_summary` through migration 16: the four temperature
/// bands, coverage, the CPU package power columns (#2021), and the
/// per-band ambient delta columns plus ambient coverage (#2045).
pub(crate) const COOLING_DAILY_SUMMARY_DDL: &str = "CREATE TABLE cooling_daily_summary (
  date TEXT PRIMARY KEY,
  idle_cpu_temperature_avg REAL,
  idle_cpu_temperature_max REAL,
  idle_cpu_temperature_min REAL,
  idle_sample_minutes INTEGER NOT NULL DEFAULT 0,
  low_cpu_temperature_avg REAL,
  low_cpu_temperature_max REAL,
  low_cpu_temperature_min REAL,
  low_sample_minutes INTEGER NOT NULL DEFAULT 0,
  mid_cpu_temperature_avg REAL,
  mid_cpu_temperature_max REAL,
  mid_cpu_temperature_min REAL,
  mid_sample_minutes INTEGER NOT NULL DEFAULT 0,
  high_cpu_temperature_avg REAL,
  high_cpu_temperature_max REAL,
  high_cpu_temperature_min REAL,
  high_sample_minutes INTEGER NOT NULL DEFAULT 0,
  coverage_minutes INTEGER NOT NULL,
  cpu_power_avg REAL,
  cpu_power_max REAL,
  cpu_power_min REAL,
  power_sample_minutes INTEGER NOT NULL DEFAULT 0,
  idle_delta_temperature_avg REAL,
  idle_delta_temperature_max REAL,
  idle_delta_temperature_min REAL,
  idle_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
  low_delta_temperature_avg REAL,
  low_delta_temperature_max REAL,
  low_delta_temperature_min REAL,
  low_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
  mid_delta_temperature_avg REAL,
  mid_delta_temperature_max REAL,
  mid_delta_temperature_min REAL,
  mid_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
  high_delta_temperature_avg REAL,
  high_delta_temperature_max REAL,
  high_delta_temperature_min REAL,
  high_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
  ambient_coverage_minutes INTEGER NOT NULL DEFAULT 0
)";

/// The single-row pinned baseline table (migration 12).
pub(crate) const COOLING_BASELINE_DDL: &str = "CREATE TABLE cooling_baseline (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  window_start_date TEXT NOT NULL,
  window_end_date TEXT NOT NULL,
  idle_temperature_avg REAL NOT NULL,
  sample_minutes INTEGER NOT NULL,
  established_at TEXT NOT NULL
)";

/// The one-minute fan-speed archive (migration 17): row-per-fan, both
/// value columns NOT NULL because a row exists only for a reading that
/// was actually taken.
pub(crate) const FAN_ARCHIVE_DDL: &str = "CREATE TABLE FAN_ARCHIVE (
  id INTEGER PRIMARY KEY,
  source TEXT NOT NULL,
  rpm INTEGER NOT NULL,
  timestamp DATETIME NOT NULL
)";

/// The per-fan daily rollup (migration 18), keyed by `(date, source)`.
pub(crate) const COOLING_FAN_DAILY_SUMMARY_DDL: &str =
  "CREATE TABLE cooling_fan_daily_summary (
  date TEXT NOT NULL,
  source TEXT NOT NULL,
  rpm_avg REAL NOT NULL,
  rpm_max INTEGER NOT NULL,
  rpm_min INTEGER NOT NULL,
  sample_minutes INTEGER NOT NULL,
  PRIMARY KEY (date, source)
)";

/// The per-hour `(load, temperature)` projection (migration 13).
pub(crate) const COOLING_HOURLY_SUMMARY_DDL: &str =
  "CREATE TABLE cooling_hourly_summary (
  hour_start TEXT PRIMARY KEY,
  cpu_usage_avg REAL,
  cpu_temperature_avg REAL,
  sample_minutes INTEGER NOT NULL
)";

/// Create each of `tables` on `pool`.
pub(crate) async fn create_tables(pool: &sqlx::SqlitePool, tables: &[&str]) {
  for ddl in tables {
    sqlx::query(ddl)
      .execute(pool)
      .await
      .expect("test schema DDL must apply");
  }
}
