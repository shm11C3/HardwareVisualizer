#![cfg(feature = "duckdb-archive")]
//! A restart, not a conversion: authority is already `NativeSelected` on
//! disk (as if a previous process had already converted and selected it),
//! and this process's own startup decision function —
//! [`hardware_monitor_lib::resolve_native_authority`], exactly what
//! `lib.rs`'s `run()` calls — must recognize `NativeAuthoritative`, adopt it
//! through #2134's dispatch boundary, and answer both reads and writes
//! routed through it. This is the "producers start on a native-authoritative
//! boot" half of the integration: this test does not construct a
//! `WorkersState` (that needs no `AppHandle` either, but starting real
//! archive/cooling/storage-health controllers needs an `EventBus` and a
//! Tauri-managed runtime this crate's plain `#[tokio::test]` does not
//! provide) — it proves the thing that makes starting them *correct*
//! instead: once `resolve_native_authority` returns, the dispatch boundary
//! those producers write through (`persistence::archive::write_archive`,
//! `persistence::cooling_rollup`, etc. — see `core/src/persistence/archive.rs`)
//! is already the live native owner, not still on SQLite.
//!
//! One test, in its own process — see `native_dispatch_conversion.rs`'s
//! module doc for why dispatch's process-wide `OnceLock`s require this.

use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::candidate_database::build_candidate_database;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::migrate;
use hardviz_core::infrastructure::database::native_database::{
  AUTHORITY_MARKER_FILE_NAME, AuthorityPaths, finalize_candidate_database,
  select_native_database,
};
use hardviz_core::persistence::archive_data::ProcessStatData;
use hardware_monitor_lib::app::native_lifecycle::DatabaseLifecycleState;
use hardware_monitor_lib::infrastructure::database::{migration, native_schema};
use hardware_monitor_lib::resolve_native_authority;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

fn at(text: &str) -> DateTime<Utc> {
  text.parse().unwrap()
}

// A plain `#[test]`, not `#[tokio::test]`: `resolve_native_authority` builds
// and drives its own short-lived runtime, exactly as it does in real
// `run()` - before Tauri's runtime exists - and a runtime cannot be started
// from inside another one already driving the current thread. The setup and
// post-restart phases below each get their own short-lived runtime for the
// same reason `apply_pending_migrations` and `resolve_native_authority`
// themselves do.
#[test]
fn a_native_selected_boot_adopts_it_and_producers_answer_through_dispatch() {
  let directory = tempfile::tempdir().unwrap();
  let paths = AuthorityPaths {
    source_database: directory.path().join("hv-database.db"),
    native_database: directory.path().join("hv-database.duckdb"),
    marker: directory.path().join(AUTHORITY_MARKER_FILE_NAME),
  };

  // Build and durably select a native database directly through Core's own
  // steps, deliberately bypassing the App's conversion driver: this test's
  // job is the *startup* half of the seam, not the conversion that produced
  // what startup finds. `native_dispatch_conversion.rs` already proves the
  // driver end of this.
  let setup_runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  setup_runtime.block_on(async {
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
    .bind(1_i32)
    .bind("carried-over-proc")
    .bind(3.0_f32)
    .bind(512_i64)
    .bind(5_i64)
    .bind(at("2026-09-01T00:00:00Z"))
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

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
    let verified =
      hardviz_core::infrastructure::database::native_database::reconcile_native_database(
        &paths.source_database,
        &paths.native_database,
        migration::get_migrations(),
        native_schema::get_native_schema(),
      )
      .await
      .unwrap()
      .1;
    select_native_database(paths.clone(), verified).await.unwrap();
  });
  drop(setup_runtime);

  // --- The restart: a fresh process's startup, with no live owner of its
  // own yet, driven by the exact function `lib.rs`'s `run()` calls. ---
  let state = resolve_native_authority(&paths, native_schema::NATIVE_SCHEMA_VERSION);
  assert_eq!(
    state,
    DatabaseLifecycleState::NativeAuthoritative,
    "a native-selected boot must be recognized without ActionRequired"
  );

  // Producers would now be started unconditionally (see `lib.rs`'s `run()` —
  // the `start_sqlite_backed_producers` gate this integration removed).
  // What makes that correct is that the dispatch boundary they write
  // through is already the live native owner: a read sees what the
  // selection carried over from SQLite, and a write lands natively.
  let post_restart_runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  post_restart_runtime.block_on(async {
    let carried_over = dispatch::process_stats::select_process_stats(
      "2026-08-31T00:00:00Z",
      "2026-09-02T00:00:00Z",
      false,
    )
    .await
    .unwrap();
    assert_eq!(carried_over.len(), 1);
    assert_eq!(carried_over[0].process_name, "carried-over-proc");

    dispatch::process_stats::insert(
      vec![ProcessStatData {
        pid: 2,
        process_name: "restart-boot-write".to_owned(),
        cpu_usage: 7.0,
        memory_usage: 1024,
        execution_sec: 2,
      }],
      at("2026-09-01T01:00:00Z"),
    )
    .await
    .unwrap();
    let after_write = dispatch::process_stats::select_process_stats(
      "2026-08-31T00:00:00Z",
      "2026-09-02T00:00:00Z",
      false,
    )
    .await
    .unwrap();
    assert_eq!(after_write.len(), 2);

    // The checkpoint-after-expiry call path (#2135's other half of this
    // integration): a no-op-shaped call through the same boundary, reaching
    // the live owner `resolve_native_authority` just adopted — exactly what
    // `persistence::archive::cleanup_old_data` calls once after its own
    // daily expiry pass.
    dispatch::checkpoint().await.unwrap();

    // A second checkpoint call is not an error either — nothing about the
    // schedule assumes it is called exactly once ever, only once per expiry
    // pass; the boundary itself imposes no such restriction.
    dispatch::checkpoint().await.unwrap();

    dispatch::shutdown().await.unwrap();
  });
}
