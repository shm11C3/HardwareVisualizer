#![cfg(feature = "duckdb-archive")]
//! A durably selected native database must never leave dispatch answering
//! consumers from the stale SQLite source when App open or hand-off fails.
//!
//! One test, in its own integration-test process: Core's dispatch configuration
//! and active backend are process-wide `OnceLock`s.

use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::candidate_database::build_candidate_database;
use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::migrate;
use hardviz_core::infrastructure::database::native_database::{
  AUTHORITY_MARKER_FILE_NAME, AuthorityPaths, NativeCancellation, NativeDatabase,
  NativeDatabaseError, NativeDatabaseOptions, finalize_candidate_database,
  reconcile_native_database, select_native_database,
};
use hardviz_core::persistence::CoolingRollupController;
use hardviz_core::persistence::archive_data::ProcessStatData;
use hardware_monitor_lib::app::native_conversion::{
  self, ConversionCancellation, ConversionOutcome, ConversionTarget, ProducerResumers,
  SelectionHandoff,
};
use hardware_monitor_lib::app::native_lifecycle::{
  DatabaseLifecycleState, LifecycleIssue, NativeLifecycleOwner,
};
use hardware_monitor_lib::infrastructure::database::{migration, native_schema};
use hardware_monitor_lib::workers::WorkersState;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

fn paths_in(directory: &Path) -> AuthorityPaths {
  AuthorityPaths {
    source_database: directory.join("hv-database.db"),
    native_database: directory.join("hv-database.duckdb"),
    marker: directory.join(AUTHORITY_MARKER_FILE_NAME),
  }
}

fn target(paths: &AuthorityPaths, workspace: &Path) -> ConversionTarget {
  ConversionTarget {
    paths: paths.clone(),
    workspace: workspace.to_path_buf(),
    expected_schema_version: native_schema::NATIVE_SCHEMA_VERSION,
  }
}

fn empty_resumers(runtime: tokio::runtime::Handle) -> ProducerResumers {
  ProducerResumers {
    hw_archive: None,
    cooling_rollup: Box::new(move || CoolingRollupController::setup(runtime).0),
    storage_health: None,
  }
}

async fn assert_dispatch_refuses_insert(pid: i32, process_name: &str) {
  let write = dispatch::process_stats::insert(
    vec![ProcessStatData {
      pid,
      process_name: process_name.to_owned(),
      cpu_usage: 1.0,
      memory_usage: 128,
      execution_sec: 1,
    }],
    at("2026-09-01T00:05:00Z"),
  )
  .await;
  assert!(
    write.is_err(),
    "dispatch must refuse writes instead of falling back to stale SQLite"
  );
}

#[tokio::test]
async fn selected_open_and_dispatch_handoff_failures_refuse_sqlite_consumers() {
  let directory = tempfile::tempdir().unwrap();
  let paths = paths_in(directory.path());

  let sqlite = SqlitePoolOptions::new()
    .max_connections(1)
    .connect_with(
      SqliteConnectOptions::new()
        .filename(&paths.source_database)
        .create_if_missing(true)
        .disable_statement_logging(),
    )
    .await
    .unwrap();
  migrate::run_on_pool(&sqlite, migration::get_migrations())
    .await
    .unwrap();
  sqlx::query(
    "INSERT INTO PROCESS_STATS (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
     VALUES ($1, $2, $3, $4, $5, $6)",
  )
  .bind(2265_i32)
  .bind("sqlite-source")
  .bind(2.0_f32)
  .bind(256_i64)
  .bind(3_i64)
  .bind(at("2026-09-01T00:00:00Z"))
  .execute(&sqlite)
  .await
  .unwrap();
  sqlite.close().await;

  assert!(db::init(paths.source_database.clone()));
  assert!(dispatch::init(
    paths.clone(),
    native_schema::NATIVE_SCHEMA_VERSION
  ));
  let before_selection = dispatch::process_stats::select_process_stats(
    "2026-08-31T00:00:00Z",
    "2026-09-02T00:00:00Z",
    false,
  )
  .await
  .unwrap();
  assert_eq!(before_selection.len(), 1);
  assert_eq!(before_selection[0].process_name, "sqlite-source");

  let candidate = directory.path().join("candidate.duckdb");
  build_candidate_database(
    &paths.source_database,
    &candidate,
    migration::get_migrations(),
  )
  .await
  .unwrap();
  finalize_candidate_database(
    &candidate,
    &paths.native_database,
    native_schema::get_native_schema(),
  )
  .await
  .unwrap();
  let verified = reconcile_native_database(
    &paths.source_database,
    &paths.native_database,
    migration::get_migrations(),
    native_schema::get_native_schema(),
  )
  .await
  .unwrap()
  .1;
  select_native_database(paths.clone(), verified)
    .await
    .unwrap();

  // Make the selected file readable for authority inspection but unavailable
  // to the runtime's read/write open. SQLite remains writable, so a consumer
  // falling back to it would make this regression observable.
  let original_permissions = std::fs::metadata(&paths.native_database)
    .unwrap()
    .permissions();
  let mut read_only_permissions = original_permissions.clone();
  read_only_permissions.set_readonly(true);
  std::fs::set_permissions(&paths.native_database, read_only_permissions).unwrap();

  let workers = WorkersState::default();
  let retry_owner = NativeLifecycleOwner::new();
  let open_outcome = native_conversion::run_conversion(
    target(&paths, directory.path()),
    &retry_owner,
    &workers,
    empty_resumers(tokio::runtime::Handle::current()),
    &ConversionCancellation::new(),
    SelectionHandoff::ThroughDispatch,
  )
  .await
  .unwrap();
  std::fs::set_permissions(&paths.native_database, original_permissions).unwrap();
  assert!(matches!(open_outcome, ConversionOutcome::ActionRequired));
  assert!(matches!(
    retry_owner.state(),
    DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed { .. })
  ));
  assert_dispatch_refuses_insert(2265, "stale-sqlite-after-open-failure").await;

  // The failure is recoverable once the file can be opened. Reobserve it to
  // establish the native owner, then explicitly close that owner before the
  // hand-off-failure scenario below.
  assert_eq!(
    dispatch::reobserve_authority().await.unwrap(),
    hardviz_core::infrastructure::database::native_database::AuthorityState::NativeSelected
  );
  dispatch::refuse_consumers("prepare the hand-off failure test".to_owned())
    .await
    .unwrap();

  let handoff_owner = NativeLifecycleOwner::new();
  handoff_owner.set_state(DatabaseLifecycleState::NativeAuthoritative);
  handoff_owner.set_selected_database(
    NativeDatabase::open(
      &paths.native_database,
      NativeDatabaseOptions::new(native_schema::NATIVE_SCHEMA_VERSION),
    )
    .await
    .unwrap(),
  );

  // Cause the next authority observation to report an interrupted conversion:
  // the selected file is still there, but its metadata is unreadable and its
  // marker is absent. The test damages only its temporary database, through
  // the sole native owner, after successful selection.
  handoff_owner
    .selected_database()
    .unwrap()
    .request_write(NativeCancellation::new(), |context| {
      context
        .connection()
        .execute_batch("DROP TABLE \"__hv_native_metadata\"")
        .map_err(|error| NativeDatabaseError::Worker {
          message: error.to_string(),
        })
    })
    .await
    .unwrap();
  std::fs::remove_file(&paths.marker).unwrap();

  let handoff =
    native_conversion::adopt_selected_database_via_dispatch(&handoff_owner).await;
  assert!(handoff.is_err());
  assert!(matches!(
    handoff_owner.state(),
    DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed { .. })
  ));
  assert_dispatch_refuses_insert(2266, "stale-sqlite-after-handoff-failure").await;
}
