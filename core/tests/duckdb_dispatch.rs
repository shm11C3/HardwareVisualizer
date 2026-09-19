#![cfg(feature = "duckdb-archive")]
//! The dispatch boundary itself (#2134): which backend answers, and when the
//! answer is allowed to change.
//!
//! One sequential test walks the whole lifecycle a real App session would
//! drive: not-selected, the window after a durable selection commits but
//! before this boundary has been told, and selected. It is one test rather
//! than several because the boundary's state (`db::init`'s `OnceLock`
//! included) is process-wide, and every step depends on the one before it -
//! splitting it would just reintroduce the ordering dependency as shared
//! fixture state.

mod native_support;

use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AuthorityState, select_native_database,
};
use hardviz_core::persistence::archive_data::ProcessStatData;
use native_support::{NativeFixture, app_native_schema};

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

fn process(pid: i32, name: &str, cpu: f32) -> ProcessStatData {
  ProcessStatData {
    pid,
    process_name: name.to_owned(),
    cpu_usage: cpu,
    memory_usage: 1024,
    execution_sec: 10,
  }
}

const WINDOW_START: &str = "2026-08-31T00:00:00Z";
const WINDOW_END: &str = "2026-09-02T00:00:00Z";

async fn select_all()
-> Vec<hardviz_core::infrastructure::database::archive_queries::ProcessStatRecord> {
  dispatch::process_stats::select_process_stats(WINDOW_START, WINDOW_END, false)
    .await
    .unwrap()
}

#[tokio::test]
async fn dispatch_answers_from_the_durably_selected_backend_and_only_when_told() {
  let fixture = NativeFixture::new();
  // The one and only `db::init` call this test binary makes - see the module
  // doc on why this has to be a single test.
  assert!(db::init(fixture.source.clone()));
  fixture.migrated_pool().await.close().await;

  // --- Not selected: nothing has configured or selected a native database
  // yet, so every dispatch call answers from SQLite exactly as
  // `hardviz_core::infrastructure::database::process_stats` would. ---
  dispatch::process_stats::insert(
    vec![process(1, "sqlite-proc", 10.0)],
    at("2026-09-01T00:00:00Z"),
  )
  .await
  .unwrap();
  let before_selection = select_all().await;
  assert_eq!(before_selection.len(), 1);
  assert_eq!(before_selection[0].process_name, "sqlite-proc");

  // Build a verified, reconciled native database from the same source, but
  // do not yet tell the dispatch boundary anything about it.
  fixture.finalize().await;
  let (_, verified) = fixture.try_reconcile().await.unwrap();
  let paths = fixture.authority_paths();
  assert!(dispatch::init(
    paths.clone(),
    app_native_schema::NATIVE_SCHEMA_VERSION
  ));

  // --- The window while selection is being recorded: the marker and the
  // native database's own metadata now durably agree the database is
  // selected, but this boundary has not been told to look again. It must
  // keep answering from SQLite - not poll disk per call - or a consumer
  // racing the lifecycle owner's `select_native_database` call could read a
  // half-adopted backend. ---
  select_native_database(paths, verified).await.unwrap();
  assert_eq!(fixture.authority_state(), AuthorityState::NativeSelected);
  let still_not_told = select_all().await;
  assert_eq!(
    still_not_told, before_selection,
    "a durable selection alone must not change what dispatch answers"
  );

  // --- Selected: once the lifecycle owner calls the seam, the boundary
  // closes nothing (it never opened anything of its own yet), observes the
  // durable state, and adopts the one live native owner. ---
  let state = dispatch::reobserve_authority().await.unwrap();
  assert_eq!(state, AuthorityState::NativeSelected);
  let after_reobserve = select_all().await;
  assert_eq!(
    after_reobserve, before_selection,
    "native must answer exactly what SQLite recorded before the conversion"
  );

  // A write issued through the boundary after selection is now native, not
  // SQLite: reopening the SQLite source directly must not see it.
  dispatch::process_stats::insert(
    vec![process(2, "native-proc", 20.0)],
    at("2026-09-01T00:05:00Z"),
  )
  .await
  .unwrap();
  let after_native_write = select_all().await;
  assert_eq!(after_native_write.len(), 2);
  let sqlite_pool = native_support::open_pool(&fixture.source, false).await;
  let sqlite_row_count: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM PROCESS_STATS WHERE pid = 2")
      .fetch_one(&sqlite_pool)
      .await
      .unwrap();
  sqlite_pool.close().await;
  assert_eq!(
    sqlite_row_count, 0,
    "a post-selection write must land in the native database, not SQLite"
  );
}
