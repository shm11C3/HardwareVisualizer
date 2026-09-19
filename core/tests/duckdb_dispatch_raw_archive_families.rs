#![cfg(feature = "duckdb-archive")]
//! Dispatch routing (#2134) for the four raw archive families:
//! `DATA_ARCHIVE`, `GPU_DATA_ARCHIVE`, Ambient and Fan.
//!
//! `core/tests/duckdb_dispatch.rs` already proves the boundary's state
//! machine (not-selected, the mid-selection window, selected) against
//! Process Stats; the arithmetic each native family reproduces is proven
//! bit-for-bit by `duckdb_data_archive.rs`, `duckdb_gpu_archive.rs` and
//! `duckdb_ambient_fan.rs`. This file's job is narrower: prove that
//! `dispatch::{data_archive, gpu_archive, ambient_archive, fan_archive}`
//! actually reach those native modules once selected, and stay on SQLite
//! until then - for every family this PR adds, not just one.
//!
//! One test, for the same `db::init`/`OnceLock` reason `duckdb_dispatch.rs`
//! documents.

mod native_support;

use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::archive_queries::{
  ArchiveBucketTimestamp, DataArchiveColumn, GpuArchiveColumn,
};
use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AuthorityState, select_native_database,
};
use hardviz_core::persistence::archive_data::{
  AmbientData, FanArchiveRow, GpuData, HardwareArchiveRow, HardwareData,
};
use native_support::{NativeFixture, app_native_schema};

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

fn reading(avg: f32) -> HardwareData {
  HardwareData {
    avg: Some(avg),
    max: Some(avg + 1.0),
    min: Some(avg - 1.0),
  }
}

fn hardware_row(cpu_avg: f32) -> HardwareArchiveRow {
  HardwareArchiveRow {
    cpu: reading(cpu_avg),
    memory: reading(50.0),
    cpu_temperature: reading(60.0),
    cpu_power: reading(10.0),
    gpu_power: reading(20.0),
    ane_power: reading(1.0),
    package_power: reading(30.0),
  }
}

fn gpu_data(usage_avg: f32) -> GpuData {
  GpuData {
    gpu_id: Some("gpu-0".to_owned()),
    gpu_name: "Test GPU".to_owned(),
    usage_avg: Some(usage_avg),
    usage_max: Some(usage_avg + 1.0),
    usage_min: Some(usage_avg - 1.0),
    temperature_avg: Some(55.0),
    temperature_max: Some(60),
    temperature_min: Some(50),
    dedicated_memory_avg: Some(1024),
    dedicated_memory_max: Some(1100),
    dedicated_memory_min: Some(900),
  }
}

// The window brackets both seeded rows with margin on each side: endpoints
// are compared as raw bytes (see `archive_queries::format_datetime`'s doc),
// so a bound landing exactly on a stored spelling can exclude that row - the
// margin here sidesteps that rather than depending on it.
const ROW_TIME: &str = "2026-09-01T00:00:00Z";
const SECOND_ROW_TIME: &str = "2026-09-01T00:05:00Z";
const WINDOW_START: &str = "2026-08-31T23:59:00Z";
const WINDOW_END: &str = "2026-09-01T00:10:00Z";
const BUCKET_WIDTH_MS: i64 = 60_000;

async fn sqlite_row_count(pool: &sqlx::SqlitePool, table: &str) -> i64 {
  sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn dispatch_routes_every_raw_archive_family_to_the_selected_backend() {
  let fixture = NativeFixture::new();
  assert!(db::init(fixture.source.clone()));
  fixture.migrated_pool().await.close().await;

  let timestamp = at(ROW_TIME);

  // --- Not selected: every family answers from SQLite. ---
  dispatch::data_archive::insert(hardware_row(10.0), timestamp)
    .await
    .unwrap();
  dispatch::gpu_archive::insert(gpu_data(40.0), timestamp)
    .await
    .unwrap();
  dispatch::ambient_archive::insert(
    vec![AmbientData {
      source: "Sensor A".to_owned(),
      temperature: 22.0,
      humidity: Some(45.0),
    }],
    timestamp,
  )
  .await
  .unwrap();
  dispatch::fan_archive::insert(
    vec![FanArchiveRow {
      source: "Fan 1".to_owned(),
      rpm: 1200,
    }],
    timestamp,
  )
  .await
  .unwrap();

  let start = at(WINDOW_START);
  let end = at(WINDOW_END);

  let data_before = dispatch::data_archive::select_data_archive_series(
    DataArchiveColumn::CpuAvg,
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(data_before.iter().filter_map(|p| p.value).count(), 1);

  let gpu_before = dispatch::gpu_archive::select_gpu_archive_series(
    GpuArchiveColumn::UsageAvg,
    "Test GPU",
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(gpu_before.iter().filter_map(|p| p.value).count(), 1);
  assert_eq!(
    dispatch::gpu_archive::select_gpu_names().await.unwrap(),
    vec!["Test GPU".to_owned()]
  );

  let ambient_before = dispatch::ambient_archive::select_ambient_archive_series(
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(ambient_before.sources, vec!["Sensor A".to_owned()]);

  let fan_before = dispatch::fan_archive::select_fan_archive_series(
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(fan_before.len(), 1);
  assert_eq!(fan_before[0].source, "Fan 1");

  // --- Build, reconcile and select a native database from the same source,
  // then tell the boundary to adopt it. ---
  fixture.finalize().await;
  let (_, verified) = fixture.try_reconcile().await.unwrap();
  let paths = fixture.authority_paths();
  assert!(dispatch::init(
    paths.clone(),
    app_native_schema::NATIVE_SCHEMA_VERSION
  ));
  select_native_database(paths, verified).await.unwrap();
  assert_eq!(
    dispatch::reobserve_authority().await.unwrap(),
    AuthorityState::NativeSelected
  );

  // --- Selected: every family now answers what SQLite recorded, but from
  // the native database. ---
  let data_after = dispatch::data_archive::select_data_archive_series(
    DataArchiveColumn::CpuAvg,
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(data_after, data_before);

  let gpu_after = dispatch::gpu_archive::select_gpu_archive_series(
    GpuArchiveColumn::UsageAvg,
    "Test GPU",
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(gpu_after, gpu_before);
  assert_eq!(
    dispatch::gpu_archive::select_gpu_names().await.unwrap(),
    vec!["Test GPU".to_owned()]
  );

  let ambient_after = dispatch::ambient_archive::select_ambient_archive_series(
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(ambient_after, ambient_before);

  let fan_after = dispatch::fan_archive::select_fan_archive_series(
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_eq!(fan_after, fan_before);

  // --- A write issued through the boundary after selection lands in the
  // native database: the SQLite tables must not grow. ---
  let sqlite_pool = native_support::open_pool(&fixture.source, false).await;
  assert_eq!(sqlite_row_count(&sqlite_pool, "DATA_ARCHIVE").await, 1);
  assert_eq!(sqlite_row_count(&sqlite_pool, "GPU_DATA_ARCHIVE").await, 1);
  assert_eq!(sqlite_row_count(&sqlite_pool, "AMBIENT_ARCHIVE").await, 1);
  assert_eq!(sqlite_row_count(&sqlite_pool, "FAN_ARCHIVE").await, 1);

  let second_timestamp = at(SECOND_ROW_TIME);
  dispatch::data_archive::insert(hardware_row(20.0), second_timestamp)
    .await
    .unwrap();
  dispatch::gpu_archive::insert(gpu_data(50.0), second_timestamp)
    .await
    .unwrap();
  dispatch::ambient_archive::insert(
    vec![AmbientData {
      source: "Sensor A".to_owned(),
      temperature: 23.0,
      humidity: Some(44.0),
    }],
    second_timestamp,
  )
  .await
  .unwrap();
  dispatch::fan_archive::insert(
    vec![FanArchiveRow {
      source: "Fan 1".to_owned(),
      rpm: 1300,
    }],
    second_timestamp,
  )
  .await
  .unwrap();

  assert_eq!(sqlite_row_count(&sqlite_pool, "DATA_ARCHIVE").await, 1);
  assert_eq!(sqlite_row_count(&sqlite_pool, "GPU_DATA_ARCHIVE").await, 1);
  assert_eq!(sqlite_row_count(&sqlite_pool, "AMBIENT_ARCHIVE").await, 1);
  assert_eq!(sqlite_row_count(&sqlite_pool, "FAN_ARCHIVE").await, 1);
  sqlite_pool.close().await;

  // And dispatch itself sees both rows now, proving the native write is
  // queryable through the same boundary that made it.
  let data_final = dispatch::data_archive::select_data_archive_series(
    DataArchiveColumn::CpuAvg,
    &start,
    &end,
    BUCKET_WIDTH_MS,
    ArchiveBucketTimestamp::Start,
  )
  .await
  .unwrap();
  assert_ne!(data_final, data_before, "the second write must be visible");
}
