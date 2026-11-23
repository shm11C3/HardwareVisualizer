#![cfg(feature = "duckdb-archive")]
//! The native schema upgrade path and native-only tables (ADR 0030), driven
//! through the production conversion against App's own schema definition.
//!
//! Production is at version 2, whose one step adds the native-only
//! `PROCESS_GPU_STATS` (#2720). A database a version 1 build selected is
//! reproduced with [`v1`], the production definition before that step.
//! Nothing here hand-writes a native file.

use super::native_support;

use hardviz_core::infrastructure::database::native_database::{
  AuthorityState, NativeCancellation, NativeDatabase, NativeDatabaseError,
  NativeDatabaseOptions, NativeIdentityMode, NativeSchemaDefinition,
  NativeSchemaUpgradeOutcome, finalize_candidate_database, inspect_authority,
  observe_authority, reconcile_native_database, select_native_database,
  upgrade_selected_native_database,
};
use native_support::{
  NativeFixture, app_migrations, app_native_schema, read_only, read_write,
};
use sqlx::{Executor, SqlitePool};

const NATIVE_ONLY_TABLE: &str = "PROCESS_GPU_STATS";

fn production() -> NativeSchemaDefinition {
  let schema = app_native_schema::get_native_schema();
  assert_eq!(schema.version, 2);
  assert_eq!(schema.native_only_tables, &[NATIVE_ONLY_TABLE]);
  schema
}

/// The production definition as a version 1 build shipped it: the same
/// baseline, without the upgrade step and its native-only identity.
fn v1() -> NativeSchemaDefinition {
  let identities = app_native_schema::IDENTITIES
    .iter()
    .copied()
    .filter(|identity| identity.mode != NativeIdentityMode::NativeOnly)
    .collect::<Vec<_>>();
  NativeSchemaDefinition {
    version: 1,
    native_only_tables: &[],
    identities: Box::leak(identities.into_boxed_slice()),
    upgrades: &[],
    ..production()
  }
}

async fn seed(pool: &SqlitePool) {
  pool
    .execute(
      r#"
      INSERT INTO DATA_ARCHIVE(id,cpu_avg,cpu_max,ram_max,timestamp)
      VALUES (1,10,20,30,'2026-09-01T00:00:00+00:00'),
             (2,11,21,31,'2026-09-01T00:01:00+00:00');
      INSERT INTO PROCESS_STATS(pid,process_name,cpu_usage,memory_usage,execution_sec,timestamp)
      VALUES (4000,'renderer',12.5,4096,60,'2026-09-01T00:00:00+00:00'),
             (4001,'helper',1.5,1024,4,'2026-09-01T00:01:00+00:00'),
             (4002,'removed',0.5,512,2,'2026-09-01T00:02:00+00:00');
      -- The deleted highest row leaves its AUTOINCREMENT mark behind.
      DELETE FROM PROCESS_STATS WHERE pid = 4002;
      "#,
    )
    .await
    .unwrap();
}

async fn seeded_fixture() -> NativeFixture {
  let fixture = NativeFixture::new();
  let pool = fixture.migrated_pool().await;
  seed(&pool).await;
  pool.close().await;
  fixture
}

fn count(path: &std::path::Path, table: &str) -> i64 {
  read_only(path)
    .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
      row.get(0)
    })
    .unwrap()
}

fn table_present(path: &std::path::Path, table: &str) -> bool {
  read_only(path)
    .query_row(
      "SELECT count(*) FROM information_schema.tables WHERE table_name = ?",
      [table],
      |row| row.get::<_, i64>(0),
    )
    .unwrap()
    != 0
}

fn identities(path: &std::path::Path) -> Vec<(String, String, i64)> {
  let connection = read_only(path);
  let mut statement = connection
    .prepare(
      "SELECT table_name, mode, high_water FROM __hv_native_identities \
       ORDER BY table_name",
    )
    .unwrap();
  statement
    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
}

fn recorded_version(path: &std::path::Path) -> i64 {
  read_only(path)
    .query_row(
      "SELECT schema_version FROM __hv_native_metadata",
      [],
      |row| row.get(0),
    )
    .unwrap()
}

async fn next_id(database: &NativeDatabase, table: &'static str) -> i64 {
  database
    .request_write(NativeCancellation::new(), move |context| {
      context.with_transaction(|transaction| transaction.next_id(table))
    })
    .await
    .unwrap()
}

/// The upgrade boot the issue names: a database converted from SQLite and
/// selected by a version 1 build gains `PROCESS_GPU_STATS` and keeps
/// everything it held.
#[tokio::test]
async fn a_converted_and_selected_v1_database_upgrades_to_production() {
  let fixture = seeded_fixture().await;
  fixture.build_candidate().await.unwrap();
  finalize_candidate_database(&fixture.candidate, &fixture.finalized, v1())
    .await
    .unwrap();
  let (_report, verified) = reconcile_native_database(
    &fixture.source,
    &fixture.finalized,
    app_migrations::get_migrations(),
    v1(),
  )
  .await
  .unwrap();
  let paths = fixture.authority_paths();
  let marker = select_native_database(paths.clone(), verified)
    .await
    .unwrap();
  assert_eq!(marker.schema_version, 1);
  assert!(!table_present(&fixture.finalized, NATIVE_ONLY_TABLE));
  let identities_before = identities(&fixture.finalized);
  let process_rows = count(&fixture.finalized, "PROCESS_STATS");
  let archive_rows = count(&fixture.finalized, "DATA_ARCHIVE");

  assert_eq!(
    upgrade_selected_native_database(&paths, production()).unwrap(),
    NativeSchemaUpgradeOutcome::Upgraded { from: 1, to: 2 }
  );

  assert_eq!(recorded_version(&fixture.finalized), 2);
  assert_eq!(
    inspect_authority(&observe_authority(&paths, 2)),
    AuthorityState::NativeSelected
  );
  let upgraded_marker: serde_json::Value =
    serde_json::from_slice(&std::fs::read(&paths.marker).unwrap()).unwrap();
  assert_eq!(upgraded_marker["schema_version"], 2);
  assert_eq!(upgraded_marker["total_rows"], marker.total_rows);
  assert_eq!(count(&fixture.finalized, "PROCESS_STATS"), process_rows);
  assert_eq!(count(&fixture.finalized, "DATA_ARCHIVE"), archive_rows);
  assert_eq!(count(&fixture.finalized, NATIVE_ONLY_TABLE), 0);

  let mut expected = identities_before;
  expected.push((NATIVE_ONLY_TABLE.to_owned(), "autoincrement".to_owned(), 0));
  expected.sort();
  assert_eq!(identities(&fixture.finalized), expected);

  let database = fixture.open().await;
  assert_eq!(next_id(&database, NATIVE_ONLY_TABLE).await, 1);
  // The SQLite-imported mark survived: the deleted highest row stays used.
  assert_eq!(next_id(&database, "PROCESS_STATS").await, 4);
  database.close().await.unwrap();

  assert_eq!(
    upgrade_selected_native_database(&paths, production()).unwrap(),
    NativeSchemaUpgradeOutcome::NotNeeded
  );
}

/// Conversion produces the production version directly: the native-only
/// table is created empty, left out of the copy and the row counts, and
/// given its identity.
#[tokio::test]
async fn converting_creates_process_gpu_stats_empty() {
  let fixture = seeded_fixture().await;

  let report = fixture.finalize().await;

  assert_eq!(report.schema_version, 2);
  assert_eq!(report.tables.len(), app_native_schema::TABLES.len());
  assert!(
    report
      .tables
      .iter()
      .all(|table| table.name != NATIVE_ONLY_TABLE)
  );
  assert_eq!(report.total_rows, 4);
  assert_eq!(recorded_version(&fixture.finalized), 2);
  assert_eq!(count(&fixture.finalized, NATIVE_ONLY_TABLE), 0);
  assert!(identities(&fixture.finalized).contains(&(
    NATIVE_ONLY_TABLE.to_owned(),
    "autoincrement".to_owned(),
    0
  )));
}

/// Reconciliation merges only SQLite-sourced tables. A native-only row is
/// neither deleted for lacking a SQLite source nor counted, and its identity
/// mark follows the native table rather than a SQLite allocator.
#[tokio::test]
async fn reconciliation_leaves_native_only_rows_alone() {
  let fixture = seeded_fixture().await;
  let finalized = fixture.finalize().await;
  read_write(&fixture.finalized)
    .execute_batch(
      "INSERT INTO PROCESS_GPU_STATS VALUES \
       (7, 4000, 'renderer', 12.5, 60.0, '2026-09-01T00:00:00+00:00'); CHECKPOINT",
    )
    .unwrap();

  let (report, _verified) = fixture.try_reconcile().await.unwrap();

  assert_eq!(report.schema_version, 2);
  assert_eq!(report.tables.len(), app_native_schema::TABLES.len());
  assert_eq!(report.total_rows, finalized.total_rows);
  assert_eq!(count(&fixture.finalized, NATIVE_ONLY_TABLE), 1);
  assert!(identities(&fixture.finalized).contains(&(
    NATIVE_ONLY_TABLE.to_owned(),
    "autoincrement".to_owned(),
    7
  )));
}

/// A SQLite-sourced table wrongly marked native-only would be created empty
/// and lose its rows, so the candidate is refused before anything is written.
#[tokio::test]
async fn a_sqlite_sourced_table_listed_as_native_only_is_refused() {
  let fixture = seeded_fixture().await;
  fixture.build_candidate().await.unwrap();
  let mut schema = production();
  let tables = app_native_schema::TABLES
    .iter()
    .copied()
    .filter(|table| *table != "DATA_ARCHIVE")
    .collect::<Vec<_>>();
  schema.tables = Box::leak(tables.into_boxed_slice());
  schema.native_only_tables = &["DATA_ARCHIVE", NATIVE_ONLY_TABLE];

  let error = finalize_candidate_database(&fixture.candidate, &fixture.finalized, schema)
    .await
    .unwrap_err();

  match error {
    NativeDatabaseError::SchemaMismatch { ref table, .. } => {
      assert_eq!(table, "DATA_ARCHIVE");
    }
    other => panic!("unexpected error: {other:?}"),
  }
  assert!(!fixture.finalized.exists());
  assert!(fixture.work_directories().is_empty());
}

/// A finalized but unselected version 1 file keeps its existing rebuild path:
/// the upgrade only ever touches the selected authority.
#[tokio::test]
async fn an_unselected_v1_database_is_not_upgraded() {
  let fixture = seeded_fixture().await;
  fixture.build_candidate().await.unwrap();
  finalize_candidate_database(&fixture.candidate, &fixture.finalized, v1())
    .await
    .unwrap();

  assert_eq!(
    upgrade_selected_native_database(&fixture.authority_paths(), production()).unwrap(),
    NativeSchemaUpgradeOutcome::NotNeeded
  );
  assert_eq!(recorded_version(&fixture.finalized), 1);
  assert!(matches!(
    NativeDatabase::open(&fixture.finalized, NativeDatabaseOptions::new(2)).await,
    Err(NativeDatabaseError::IncompatibleSchema {
      expected: 2,
      actual: 1
    })
  ));
}
