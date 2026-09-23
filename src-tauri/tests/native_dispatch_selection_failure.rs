#![cfg(feature = "duckdb-archive")]
//! Regression for #2238: a conversion whose selection committed into the
//! native file but then failed (here, publishing the marker) must not let
//! anything keep writing to SQLite, because the next startup adopts the
//! native file and retires SQLite without reconciling again.
//!
//! Drives the App's real driver under `SelectionHandoff::ThroughDispatch`,
//! writes through the dispatch boundary after the failure, then restarts
//! through [`hardware_monitor_lib::resolve_native_authority`] - the function
//! `lib.rs`'s `run()` calls - and checks that the boundary refused that
//! write rather than parking it in the SQLite copy the restart retires.
//!
//! One test, in its own process - see `native_dispatch_conversion.rs`'s
//! module doc for why dispatch's process-wide `OnceLock`s require this.

use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::migrate;
use hardviz_core::infrastructure::database::native_database::{
  AUTHORITY_MARKER_FILE_NAME, AuthorityPaths,
};
use hardviz_core::persistence::CoolingRollupController;
use hardviz_core::persistence::archive_data::ProcessStatData;
use hardware_monitor_lib::app::native_conversion::{
  self, ConversionCancellation, ConversionOutcome, ConversionTarget, ProducerResumers,
  SelectionHandoff,
};
use hardware_monitor_lib::app::native_lifecycle::{
  DatabaseLifecycleState, NativeLifecycleOwner,
};
use hardware_monitor_lib::infrastructure::database::{migration, native_schema};
use hardware_monitor_lib::resolve_native_authority;
use hardware_monitor_lib::workers::WorkersState;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

fn runtime() -> tokio::runtime::Runtime {
  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap()
}

// A plain `#[test]` with explicit runtimes: `resolve_native_authority` drives
// its own runtime, exactly as it does before Tauri's exists, and cannot run
// inside another one - see `native_dispatch_restart.rs`.
#[test]
fn a_failure_after_the_selection_commit_loses_no_row_across_a_restart() {
  let directory = tempfile::tempdir().unwrap();
  // The marker's directory does not exist yet, so the selection commits and
  // then fails to publish (and to repair) the marker.
  let marker_directory = directory.path().join("marker");
  let paths = AuthorityPaths {
    source_database: directory.path().join("hv-database.db"),
    native_database: directory.path().join("hv-database.duckdb"),
    marker: marker_directory.join(AUTHORITY_MARKER_FILE_NAME),
  };

  runtime().block_on(async {
    let pool = SqlitePoolOptions::new()
      .max_connections(1)
      .connect_with(
        SqliteConnectOptions::new()
          .filename(&paths.source_database)
          .create_if_missing(true)
          .disable_statement_logging(),
      )
      .await
      .unwrap();
    migrate::run_on_pool(&pool, migration::get_migrations())
      .await
      .unwrap();
    sqlx::query(
      "INSERT INTO PROCESS_STATS (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
       VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(1_i32)
    .bind("sqlite-seeded-proc")
    .bind(3.0_f32)
    .bind(512_i64)
    .bind(5_i64)
    .bind(at("2026-09-01T00:00:00Z"))
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    assert!(db::init(paths.source_database.clone()));
    assert!(dispatch::init(
      paths.clone(),
      native_schema::NATIVE_SCHEMA_VERSION
    ));

    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();
    let runtime = tokio::runtime::Handle::current();
    let outcome = native_conversion::run_conversion(
      ConversionTarget {
        paths: paths.clone(),
        workspace: directory.path().to_path_buf(),
        expected_schema_version: native_schema::NATIVE_SCHEMA_VERSION,
      },
      &owner,
      &workers,
      ProducerResumers {
        hw_archive: None,
        cooling_rollup: Box::new(move || CoolingRollupController::setup(runtime).0),
        storage_health: None,
      },
      &ConversionCancellation::new(),
      SelectionHandoff::ThroughDispatch,
    )
    .await;

    assert!(
      matches!(outcome, Ok(ConversionOutcome::ActionRequired)),
      "a failure after the commit is not a retryable failure: {outcome:?}"
    );
    assert!(
      matches!(owner.state(), DatabaseLifecycleState::ActionRequired(_)),
      "{:?}",
      owner.state()
    );
    assert!(
      workers.cooling_rollup.lock().unwrap().is_none(),
      "producers must not resume while SQLite is no longer authoritative"
    );

    // A write through the boundary after the failure. Before #2238 it landed
    // in SQLite, which the restart below retires without reconciling; the
    // boundary must refuse it instead.
    let write = dispatch::process_stats::insert(
      vec![ProcessStatData {
        pid: 2,
        process_name: "written-after-failure".to_owned(),
        cpu_usage: 1.0,
        memory_usage: 256,
        execution_sec: 1,
      }],
      at("2026-09-01T00:05:00Z"),
    )
    .await;
    assert!(
      write.is_err(),
      "the boundary must refuse writes while the selection is incomplete"
    );
  });

  // The cause goes away, and the next process starts.
  std::fs::create_dir(&marker_directory).unwrap();
  let state = resolve_native_authority(&paths, native_schema::NATIVE_SCHEMA_VERSION);
  assert_eq!(state, DatabaseLifecycleState::NativeAuthoritative);

  runtime().block_on(async {
    let rows = dispatch::process_stats::select_process_stats(
      "2026-08-31T00:00:00Z",
      "2026-09-02T00:00:00Z",
      false,
    )
    .await
    .unwrap();
    let names: Vec<_> = rows.iter().map(|row| row.process_name.as_str()).collect();
    assert_eq!(names, ["sqlite-seeded-proc"]);
    dispatch::shutdown().await.unwrap();
  });
}
