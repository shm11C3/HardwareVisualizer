#![cfg(feature = "duckdb-archive")]
//! Shutdown must close the dispatch boundary to late database consumers.

use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AUTHORITY_MARKER_FILE_NAME, AuthorityPaths, AuthorityState,
  create_empty_native_database,
};
use native_support::app_native_schema;

mod native_support;

#[tokio::test]
async fn dispatch_refuses_late_consumers_without_creating_sqlite_database() {
  let directory = tempfile::tempdir().unwrap();
  let database_path = directory.path().join("hv-database.db");
  assert!(db::init(database_path.clone()));
  assert!(!database_path.exists());

  let paths = AuthorityPaths {
    source_database: database_path.clone(),
    native_database: directory.path().join("hv-database.duckdb"),
    marker: directory.path().join(AUTHORITY_MARKER_FILE_NAME),
  };
  create_empty_native_database(paths.clone(), app_native_schema::get_native_schema())
    .await
    .unwrap();
  assert!(dispatch::init(
    paths,
    app_native_schema::NATIVE_SCHEMA_VERSION
  ));
  assert_eq!(
    dispatch::reobserve_authority().await.unwrap(),
    AuthorityState::NativeSelected
  );

  dispatch::shutdown().await.unwrap();
  dispatch::shutdown().await.unwrap();
  // A refusal that arrives late, such as a failed native hand-off, must not
  // downgrade shutdown to a refusal a later reobservation could reopen.
  dispatch::refuse_consumers("a native hand-off failed during shutdown".to_owned())
    .await
    .unwrap();

  assert!(matches!(
    dispatch::reobserve_authority().await,
    Err(dispatch::DispatchError::Shutdown)
  ));

  let error = dispatch::process_stats::select_process_stats(
    "2026-08-31T00:00:00Z",
    "2026-09-02T00:00:00Z",
    false,
  )
  .await
  .unwrap_err();

  assert!(matches!(error, dispatch::DispatchError::Shutdown));
  assert!(
    !database_path.exists(),
    "a late dispatch must not recreate the SQLite database after shutdown"
  );
}
