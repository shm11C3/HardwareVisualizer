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
use std::path::Path;
use std::process::Command;

const RESTART_PROFILE_ENV: &str = "HARDVIZ_FRESH_INSTALL_RESTART_PROFILE";

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

// A plain test is required because resolve_native_authority creates and drives
// its own short-lived runtime before the application's Tokio runtime starts.
// The restart phase runs this same test in a child process: dispatch shutdown
// is terminal for one process, and a second runtime cannot model a new one.
// This scenario stays in its own integration-test binary because dispatch is
// process-global.
#[test]
fn an_empty_profile_creates_native_dispatches_and_restarts() {
  if let Some(profile) = std::env::var_os(RESTART_PROFILE_ENV) {
    let paths = paths_in(Path::new(&profile));
    let restarted =
      resolve_native_authority(&paths, native_schema::NATIVE_SCHEMA_VERSION);
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
    return;
  }

  let directory = tempfile::tempdir().unwrap();
  let paths = paths_in(directory.path());

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

  // The next process sees the already-selected file, reopens it through its
  // own dispatch boundary, and retains the first process's row.
  let output = Command::new(std::env::current_exe().unwrap())
    .arg("--exact")
    .arg("an_empty_profile_creates_native_dispatches_and_restarts")
    .arg("--nocapture")
    .env(RESTART_PROFILE_ENV, directory.path())
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "restart child failed ({}):\n{}\n{}",
    output.status,
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr),
  );
}

fn paths_in(profile: &Path) -> AuthorityPaths {
  AuthorityPaths {
    source_database: profile.join("hv-database.db"),
    native_database: profile.join("hv-database.duckdb"),
    marker: profile.join(AUTHORITY_MARKER_FILE_NAME),
  }
}
