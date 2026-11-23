use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use sqlx::sqlite::{
  SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous,
};

/// Process-wide database location. Set once at App startup via [`init`]
/// (typically `<AppData>/<bundle-id>/hv-database.db`). Core has no way
/// to resolve the bundle identifier on its own — that lives in Tauri
/// configuration — so the path must be threaded in from App.
static DB_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Configure the SQLite file used by every persistence writer. Should
/// be called once at startup, before any archive worker runs.
///
/// Returns `true` if this call set the process-wide database path,
/// `false` if it was already set by an earlier call. The path is held
/// in a [`OnceLock`], so a second call with a different path does
/// **not** replace the original — callers that need to know whether
/// their requested path actually took effect should check the return
/// value (e.g. test fixtures running in the same process).
pub fn init(path: PathBuf) -> bool {
  DB_PATH.set(path).is_ok()
}

/// Read the configured DB path. Panics if [`init`] hasn't been called —
/// hitting this in normal operation indicates an ordering bug in App
/// startup, not a recoverable runtime condition.
fn db_path() -> &'static Path {
  DB_PATH
    .get()
    .expect(
      "hardviz_core::infrastructure::database::db::init must be called before get_pool",
    )
    .as_path()
}

/// Open a fresh `SqlitePool` against the configured database file.
///
/// Each writer call gets its own pool today (matching the pre-Phase-4
/// behavior in `src-tauri/src/infrastructure/database`). Pool caching
/// is intentionally out of scope for #1407 — see "Out of Scope" in the
/// issue ("Schema redesigns or migrations.").
pub async fn get_pool() -> Result<SqlitePool, sqlx::Error> {
  open_pool(db_path()).await
}

async fn open_pool(path: &Path) -> Result<SqlitePool, sqlx::Error> {
  ensure_parent_dir(path).await?;
  let options = SqliteConnectOptions::new()
    .filename(path)
    .create_if_missing(true)
    .journal_mode(SqliteJournalMode::Wal)
    .busy_timeout(Duration::from_secs(5))
    .synchronous(SqliteSynchronous::Normal);
  SqlitePool::connect_with(options).await
}

async fn ensure_parent_dir(path: &Path) -> Result<(), sqlx::Error> {
  if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(sqlx::Error::Io)?;
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[tokio::test]
  async fn open_pool_creates_missing_database_file() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("missing").join("hv-database.db");

    assert!(!db_path.exists());

    let pool = open_pool(&db_path).await.unwrap();
    sqlx::query("CREATE TABLE smoke (id INTEGER PRIMARY KEY)")
      .execute(&pool)
      .await
      .unwrap();
    pool.close().await;

    assert!(db_path.exists());
  }

  #[tokio::test]
  async fn open_pool_configures_sqlite_for_concurrent_access() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("hv-database.db");

    let pool = open_pool(&db_path).await.unwrap();
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
      .fetch_one(&pool)
      .await
      .unwrap();
    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
      .fetch_one(&pool)
      .await
      .unwrap();
    let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
      .fetch_one(&pool)
      .await
      .unwrap();
    pool.close().await;

    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(busy_timeout, 5_000);
    // SQLite represents NORMAL as the integer value 1.
    assert_eq!(synchronous, 1);
  }

  #[tokio::test]
  async fn ensure_parent_dir_allows_filename_only_database_paths() {
    ensure_parent_dir(Path::new("hv-database.db"))
      .await
      .unwrap();
  }
}
