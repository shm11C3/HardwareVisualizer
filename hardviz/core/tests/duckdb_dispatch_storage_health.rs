#![cfg(feature = "duckdb-archive")]
//! Dispatch routing (#2134) for the Storage Health family.
//!
//! Companion to `duckdb_dispatch.rs` and
//! `duckdb_dispatch_raw_archive_families.rs`: this file only proves that
//! `dispatch::storage_health` reaches `native_database::storage_health` once
//! selected and stays on SQLite until then. `duckdb_storage_health.rs`
//! already proves the native family reproduces SQLite bit for bit, including
//! the `NOCASE` collation difference and the id-burning upsert behaviour.
//!
//! Identities are producer-shaped (`storage_device_id`), for the reason
//! `duckdb_storage_health.rs`'s module doc gives: a fixture with an invented
//! id shape would certify routing against an id namespace production never
//! writes.

mod native_support;

use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AuthorityState, select_native_database,
};
use hardviz_core::models::hardware::{
  SmartDiskInfo, SmartHealthStatus, StorageDeviceRecord, StorageHealthRecordDraft,
  StorageHealthStatus, StorageWarningLevel,
};
use hardviz_core::persistence::storage_health::storage_device_id;
use hardviz_core::settings::STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES;
use native_support::{NativeFixture, app_native_schema};

const IDENTITY_HASH_KEY: [u8; STORAGE_HEALTH_IDENTITY_HASH_KEY_BYTES] = [0x11; 32];

fn disk(device_name: &str) -> SmartDiskInfo {
  SmartDiskInfo {
    device_name: device_name.to_owned(),
    device_type: Some("nvme".to_owned()),
    protocol: Some("NVMe".to_owned()),
    model_name: Some("Dispatch Test SSD".to_owned()),
    serial_number: Some(format!("SERIAL-{device_name}")),
    firmware_version: None,
    capacity_bytes: Some(1_000_000_000),
    health_status: SmartHealthStatus::Passed,
    temperature_celsius: Some(35),
    power_on_hours: Some(10),
    power_cycle_count: Some(1),
    attributes: Vec::new(),
  }
}

fn device(id: &str) -> StorageDeviceRecord {
  StorageDeviceRecord {
    id: id.to_owned(),
    display_name: "Dispatch Test SSD".to_owned(),
    model: Some("Dispatch Test SSD".to_owned()),
    serial_hash: None,
    protocol: Some("NVMe".to_owned()),
    capacity_bytes: Some(1_000_000_000),
    first_seen_at: "2026-05-10T00:00:00Z".to_owned(),
    last_seen_at: "2026-05-10T00:00:00Z".to_owned(),
  }
}

fn record(device_id: &str, date: &str) -> StorageHealthRecordDraft {
  StorageHealthRecordDraft {
    device_id: device_id.to_owned(),
    date: date.to_owned(),
    health_status: StorageHealthStatus::Good,
    warning_level: StorageWarningLevel::None,
    temperature_celsius: Some(35.0),
    power_on_hours: Some(10),
    percentage_used: None,
    available_spare_percent: None,
    reallocated_sector_count: None,
    current_pending_sector_count: None,
    offline_uncorrectable_count: None,
    media_errors: None,
    error_log_entries: None,
    unsafe_shutdown_count: None,
    warning_reasons: Vec::new(),
    collected_at: "2026-05-10T00:00:00Z".to_owned(),
  }
}

#[tokio::test]
async fn dispatch_routes_storage_health_to_the_selected_backend() {
  let fixture = NativeFixture::new();
  assert!(db::init(fixture.source.clone()));
  fixture.migrated_pool().await.close().await;

  let device_id = storage_device_id(&disk("disk-a"), &IDENTITY_HASH_KEY);

  // --- Not selected: answers from SQLite. ---
  dispatch::storage_health::insert_daily_records(
    vec![device(&device_id)],
    vec![record(&device_id, "2026-05-10")],
  )
  .await
  .unwrap();

  let before_selection = dispatch::storage_health::latest_records().await.unwrap();
  let seeded_before: Vec<_> = before_selection
    .iter()
    .filter(|record| record.device_id == device_id)
    .collect();
  assert_eq!(seeded_before.len(), 1);
  assert_eq!(seeded_before[0].date, "2026-05-10");

  // --- Build, reconcile and select a native database from the same source. ---
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

  // --- Selected: native answers what SQLite recorded. ---
  let after_reobserve = dispatch::storage_health::latest_records().await.unwrap();
  let seeded_after: Vec<_> = after_reobserve
    .iter()
    .filter(|record| record.device_id == device_id)
    .collect();
  assert_eq!(seeded_after.len(), 1);
  assert_eq!(seeded_after[0].date, "2026-05-10");
  assert_eq!(
    seeded_after[0].health_status,
    seeded_before[0].health_status
  );

  // A `refresh_daily_records` write through the boundary lands in native, not
  // SQLite: the SQLite table must not gain the new day's row.
  dispatch::storage_health::refresh_daily_records(
    std::slice::from_ref(&device_id),
    vec![device(&device_id)],
    vec![record(&device_id, "2026-05-11")],
  )
  .await
  .unwrap();

  let sqlite_pool = native_support::open_pool(&fixture.source, false).await;
  let sqlite_row_count: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM storage_health_daily_records WHERE device_id = ?",
  )
  .bind(&device_id)
  .fetch_one(&sqlite_pool)
  .await
  .unwrap();
  sqlite_pool.close().await;
  assert_eq!(
    sqlite_row_count, 1,
    "a post-selection write must land in the native database, not SQLite"
  );

  let after_native_write = dispatch::storage_health::latest_records().await.unwrap();
  let latest_for_device = after_native_write
    .iter()
    .find(|record| record.device_id == device_id)
    .expect("the device must still be listed");
  assert_eq!(latest_for_device.date, "2026-05-11");

  // Retention through the boundary also reaches the native database: deleting
  // everything (a huge retention window's complement - 0 days back) removes
  // the row dispatch just wrote, proving `delete_old_data` is routed too.
  dispatch::storage_health::delete_old_data(0).await.unwrap();
  let after_delete = dispatch::storage_health::latest_records().await.unwrap();
  assert!(
    !after_delete
      .iter()
      .any(|record| record.device_id == device_id),
    "retention through the boundary must reach the native database"
  );
}
