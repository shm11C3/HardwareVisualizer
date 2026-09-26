#![cfg(feature = "duckdb-archive")]
//! A hand-off that expects the selected native database must leave dispatch
//! refusing consumers when the files report anything else, without ever
//! answering from SQLite in between.

use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AUTHORITY_MARKER_FILE_NAME, AuthorityPaths, AuthorityState,
};
use native_support::app_native_schema;

mod native_support;

#[tokio::test]
async fn expecting_selected_refuses_consumers_when_sqlite_is_observed() {
  let directory = tempfile::tempdir().unwrap();
  let database_path = directory.path().join("hv-database.db");
  assert!(db::init(database_path.clone()));

  let paths = AuthorityPaths {
    source_database: database_path.clone(),
    native_database: directory.path().join("hv-database.duckdb"),
    marker: directory.path().join(AUTHORITY_MARKER_FILE_NAME),
  };
  assert!(dispatch::init(
    paths,
    app_native_schema::NATIVE_SCHEMA_VERSION
  ));

  assert_eq!(
    dispatch::reobserve_expecting_selected().await.unwrap(),
    AuthorityState::SqliteAuthoritative
  );

  let error = dispatch::process_stats::select_process_stats(
    "2026-08-31T00:00:00Z",
    "2026-09-02T00:00:00Z",
    false,
  )
  .await
  .unwrap_err();

  assert!(matches!(
    error,
    dispatch::DispatchError::NativeUnavailable { .. }
  ));
  assert!(
    !database_path.exists(),
    "a failed hand-off must not let a consumer create the SQLite database"
  );
}
