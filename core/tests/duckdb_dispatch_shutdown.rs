#![cfg(feature = "duckdb-archive")]
//! Shutdown must close the dispatch boundary to late database consumers.

use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;

#[tokio::test]
async fn dispatch_refuses_late_consumers_without_creating_sqlite_database() {
  let directory = tempfile::tempdir().unwrap();
  let database_path = directory.path().join("hv-database.db");
  assert!(db::init(database_path.clone()));
  assert!(!database_path.exists());

  dispatch::shutdown().await.unwrap();
  dispatch::shutdown().await.unwrap();

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
