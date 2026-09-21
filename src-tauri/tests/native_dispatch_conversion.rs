#![cfg(feature = "duckdb-archive")]
//! End-to-end proof that a seeded SQLite database, once converted and
//! selected by the App's own production conversion driver
//! ([`app::native_conversion::run_conversion`]), answers #2134's dispatch
//! consumers from the native file: under `SelectionHandoff::ThroughDispatch`
//! the driver runs the `// #2134 seam:`
//! ([`app::native_conversion::adopt_selected_database_via_dispatch`]) itself,
//! while the producers are still paused, so by the time it returns the
//! boundary already answers from the native file and
//! `NativeLifecycleOwner` holds no handle of its own beside it.
//!
//! One test, in its own process: `dispatch::init`/`reobserve_authority` are
//! process-wide (a `OnceLock`-backed configuration — see their own
//! documentation), so a second scenario in the same test binary would
//! corrupt this one's configuration the moment both ran. This mirrors
//! `core/tests/duckdb_dispatch.rs`, which is one sequential test per file
//! for the identical reason.
//!
//! [`app::native_conversion::run_conversion`]: hardware_monitor_lib::app::native_conversion::run_conversion
//! [`app::native_conversion::adopt_selected_database_via_dispatch`]: hardware_monitor_lib::app::native_conversion::adopt_selected_database_via_dispatch

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
use hardware_monitor_lib::workers::WorkersState;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

/// A `ProducerResumers` that only restarts the cooling rollup, matching
/// `native_conversion`'s own test helper — nothing else was running to pause
/// in the first place in this scenario.
fn empty_resumers(runtime: tokio::runtime::Handle) -> ProducerResumers {
  ProducerResumers {
    hw_archive: None,
    cooling_rollup: Box::new(move || CoolingRollupController::setup(runtime).0),
    storage_health: None,
  }
}

#[tokio::test]
async fn converted_and_selected_database_answers_dispatch_consumers() {
  let directory = tempfile::tempdir().unwrap();
  let paths = AuthorityPaths {
    source_database: directory.path().join("hv-database.db"),
    native_database: directory.path().join("hv-database.duckdb"),
    marker: directory.path().join(AUTHORITY_MARKER_FILE_NAME),
  };

  // Seed a real SQLite source through App's own ordered migrations, then
  // write one process-stats row directly against the pool — the fixture
  // philosophy `native_conversion`'s own tests and `core/tests/native_support`
  // already established.
  let options = SqliteConnectOptions::new()
    .filename(&paths.source_database)
    .create_if_missing(true)
    .disable_statement_logging();
  let pool = SqlitePoolOptions::new()
    .max_connections(1)
    .connect_with(options)
    .await
    .unwrap();
  migrate::run_on_pool(&pool, migration::get_migrations())
    .await
    .unwrap();
  sqlx::query(
    "INSERT INTO PROCESS_STATS (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
     VALUES ($1, $2, $3, $4, $5, $6)",
  )
  .bind(4321_i32)
  .bind("sqlite-seeded-proc")
  .bind(12.5_f32)
  .bind(2048_i64)
  .bind(30_i64)
  .bind(at("2026-09-01T00:00:00Z"))
  .execute(&pool)
  .await
  .unwrap();
  pool.close().await;

  // Register the same source with Core's SQLite pool (`db::init`'s own
  // `OnceLock` — separate from dispatch's, and the reason dispatch's SQLite
  // path can answer at all before any selection exists): the one and only
  // call this test binary makes.
  assert!(db::init(paths.source_database.clone()));

  // Tell the dispatch boundary where to look, the same call `lib.rs`'s
  // `resolve_native_authority` makes at real startup — the one and only
  // `dispatch::init` this test binary calls, per the module doc above.
  assert!(dispatch::init(
    paths.clone(),
    native_schema::NATIVE_SCHEMA_VERSION
  ));

  // A consumer routed through dispatch before anything is selected answers
  // from the seeded SQLite source, exactly as it would with no `// #2135`
  // conversion in the picture at all.
  let before_conversion = dispatch::process_stats::select_process_stats(
    "2026-08-31T00:00:00Z",
    "2026-09-02T00:00:00Z",
    false,
  )
  .await
  .unwrap();
  assert_eq!(before_conversion.len(), 1);
  assert_eq!(before_conversion[0].process_name, "sqlite-seeded-proc");

  // Drive the App's real conversion path: preflight, candidate, finalize,
  // pause producers (none running here), reconcile, select, and - still
  // inside the paused block - the hand-over to the dispatch boundary.
  let owner = NativeLifecycleOwner::new();
  let workers = WorkersState::default();
  let outcome = native_conversion::run_conversion(
    ConversionTarget {
      paths: paths.clone(),
      workspace: directory.path().to_path_buf(),
      expected_schema_version: native_schema::NATIVE_SCHEMA_VERSION,
    },
    &owner,
    &workers,
    empty_resumers(tokio::runtime::Handle::current()),
    &ConversionCancellation::new(),
    SelectionHandoff::ThroughDispatch,
  )
  .await
  .unwrap();
  assert!(
    matches!(outcome, ConversionOutcome::Selected { total_rows: 1 }),
    "{outcome:?}"
  );
  assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
  assert!(
    owner.selected_database().is_none(),
    "the seam must hand the database over, not leave a second instance open beside it"
  );

  // Every consumer now answers from the native file, with exactly the row
  // the conversion carried over from SQLite.
  let after_adoption = dispatch::process_stats::select_process_stats(
    "2026-08-31T00:00:00Z",
    "2026-09-02T00:00:00Z",
    false,
  )
  .await
  .unwrap();
  assert_eq!(after_adoption, before_conversion);

  // A write issued through the boundary after adoption is native, not
  // SQLite: reopening the SQLite source directly must not see it.
  dispatch::process_stats::insert(
    vec![ProcessStatData {
      pid: 9999,
      process_name: "native-seam-proc".to_owned(),
      cpu_usage: 5.0,
      memory_usage: 4096,
      execution_sec: 1,
    }],
    at("2026-09-01T00:05:00Z"),
  )
  .await
  .unwrap();
  let after_native_write = dispatch::process_stats::select_process_stats(
    "2026-08-31T00:00:00Z",
    "2026-09-02T00:00:00Z",
    false,
  )
  .await
  .unwrap();
  assert_eq!(after_native_write.len(), 2);

  let sqlite_pool = SqlitePoolOptions::new()
    .max_connections(1)
    .connect_with(
      SqliteConnectOptions::new()
        .filename(&paths.source_database)
        .disable_statement_logging(),
    )
    .await
    .unwrap();
  let sqlite_row_count: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM PROCESS_STATS WHERE pid = 9999")
      .fetch_one(&sqlite_pool)
      .await
      .unwrap();
  sqlite_pool.close().await;
  assert_eq!(
    sqlite_row_count, 0,
    "a post-adoption write must land in the native database, not the retained SQLite source"
  );

  // Asking again while the boundary holds the file must not re-inspect
  // authority: that would open a second DuckDB instance beside the boundary's
  // live owner and report a healthy selection as unreadable.
  let again = native_conversion::run_conversion(
    ConversionTarget {
      paths: paths.clone(),
      workspace: directory.path().to_path_buf(),
      expected_schema_version: native_schema::NATIVE_SCHEMA_VERSION,
    },
    &owner,
    &workers,
    empty_resumers(tokio::runtime::Handle::current()),
    &ConversionCancellation::new(),
    SelectionHandoff::ThroughDispatch,
  )
  .await
  .unwrap();
  assert!(
    matches!(again, ConversionOutcome::AlreadySelected),
    "{again:?}"
  );
  assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
  let still_native = dispatch::process_stats::select_process_stats(
    "2026-08-31T00:00:00Z",
    "2026-09-02T00:00:00Z",
    false,
  )
  .await
  .unwrap();
  assert_eq!(still_native.len(), 2);

  // `run_conversion` resumed the cooling-rollup worker into `workers`, and it
  // reads and writes through dispatch. `terminate_all` drains it first and
  // closes the dispatch boundary last, the order the App's own quit path uses.
  workers.terminate_all().await;
}
