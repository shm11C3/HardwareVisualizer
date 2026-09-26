//! SQLite-backed persistence used by [`crate::persistence`].
//!
//! Core owns the database: callers initialize its location once at startup
//! via [`db::init`] (Core never resolves the path itself because that
//! depends on Tauri's bundle identifier), and Core applies the schema
//! migrations through [`migrate::run`] before any worker writes. App owns
//! the ordered migration definitions and hands them to the runner. Every
//! read / write that Core executes goes through this module.

/// 64 KiB reduces append-time memory in the measured sparse history workload;
/// dense synthetic files grew about 40% versus 256 KiB. DuckDB applies this
/// only when creating a file; existing archives keep their recorded block size.
#[cfg(feature = "duckdb-archive")]
pub(crate) const NATIVE_DATABASE_DEFAULT_BLOCK_SIZE_BYTES: &str = "65536";

pub mod ambient_archive;
pub mod archive_queries;
#[cfg(feature = "duckdb-archive")]
pub mod candidate_database;
pub mod cooling_baseline;
pub mod cooling_covariate_daily_summary;
pub mod cooling_daily_summary;
pub mod cooling_delta_baseline;
pub mod cooling_fan_daily_summary;
pub mod cooling_hourly_summary;
pub mod cooling_thermal_delta_daily_summary;
pub mod db;
pub mod dispatch;
pub mod fan_archive;
pub mod gpu_archive;
pub mod hardware_archive;
pub mod migrate;
#[cfg(feature = "duckdb-archive")]
pub mod native_database;
pub mod process_stats;
pub mod storage_health;
#[cfg(test)]
pub(crate) mod test_schema;
