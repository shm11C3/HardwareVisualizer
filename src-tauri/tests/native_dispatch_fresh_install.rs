#![cfg(feature = "duckdb-archive")]
//! Fresh-install proof for #2191: an empty profile creates a selected native
//! database before SQLite exists, and the dispatch boundary serves producers
//! through that database on the first boot and after a restart.

use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AUTHORITY_MARKER_FILE_NAME, AuthorityPaths,
};
use hardviz_core::persistence::archive_data::ProcessStatData;
use hardware_monitor_lib::app::native_lifecycle::DatabaseLifecycleState;
use hardware_monitor_lib::infrastructure::database::native_schema;
use hardware_monitor_lib::resolve_native_authority;

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

// A plain test is required because resolve_native_authority creates and drives
// its own short-lived runtime before the application's Tokio runtime starts.
// The second startup phase uses another short-lived runtime for the same
// reason. Dispatch is process-global, so this scenario stays in its own
// integration-test binary.
#[test]
fn an_empty_profile_creates_native_dispatches_and_restarts() {
  let directory = tempfile::tempdir().unwrap();
  let paths = AuthorityPaths {
    source_database: directory.path().join("hv-database.db"),
    native_database: directory.path().join("hv-database.duckdb"),
    marker: directory.path().join(AUTHORITY_MARKER_FILE_NAME),
  };

  // Simulate a process interrupted before publishing the native file. The
  // fresh-install startup is allowed to discard this work only because no
  // source, native database or marker exists yet.
  let interrupted_work = directory
    .path()
    .join(".hardwarevisualizer-duckdb-fresh-interrupted");
  std::fs::create_dir(&interrupted_work).unwrap();

  let state = resolve_native_authority(&paths, native_schema::NATIVE_SCHEMA_VERSION);
  assert_eq!(state, DatabaseLifecycleState::NativeAuthoritative);
  assert!(!paths.source_database.exists());
  assert!(paths.native_database.is_file());
  assert!(paths.marker.is_file());
  assert!(!interrupted_work.exists());

  let first_boot_runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  first_boot_runtime.block_on(async {
    assert!(
      dispatch::process_stats::select_process_stats(
        "2026-08-31T00:00:00Z",
        "2026-09-02T00:00:00Z",
        false,
      )
      .await
      .unwrap()
      .is_empty()
    );

    dispatch::process_stats::insert(
      vec![ProcessStatData {
        pid: 2191,
        process_name: "fresh-native-boot".to_owned(),
        cpu_usage: 4.5,
        memory_usage: 256,
        execution_sec: 1,
      }],
      at("2026-09-01T00:00:00Z"),
    )
    .await
    .unwrap();

    let rows = dispatch::process_stats::select_process_stats(
      "2026-08-31T00:00:00Z",
      "2026-09-02T00:00:00Z",
      false,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].process_name, "fresh-native-boot");
    dispatch::shutdown().await.unwrap();
  });
  drop(first_boot_runtime);

  // The next startup sees the already-selected native file, reopens it
  // through the same dispatch boundary, and retains the first boot's row.
  let restarted = resolve_native_authority(&paths, native_schema::NATIVE_SCHEMA_VERSION);
  assert_eq!(restarted, DatabaseLifecycleState::NativeAuthoritative);
  let restart_runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  restart_runtime.block_on(async {
    let rows = dispatch::process_stats::select_process_stats(
      "2026-08-31T00:00:00Z",
      "2026-09-02T00:00:00Z",
      false,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].process_name, "fresh-native-boot");
    dispatch::shutdown().await.unwrap();
  });
}
