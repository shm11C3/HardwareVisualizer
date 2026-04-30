use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sqlx::sqlite::SqlitePool;

/// Process-wide database location. Set once at App startup via [`init`]
/// (typically `<AppData>/<bundle-id>/hv-database.db`). Core has no way
/// to resolve the bundle identifier on its own — that lives in Tauri
/// configuration — so the path must be threaded in from App.
static DB_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Configure the SQLite file used by every persistence writer. Should be
/// called once at startup, before any archive worker runs. Subsequent
/// calls are silently ignored so re-initialization in tests is harmless.
pub fn init(path: PathBuf) {
  let _ = DB_PATH.set(path);
}

/// Read the configured DB path. Panics if [`init`] hasn't been called —
/// hitting this in normal operation indicates an ordering bug in App
/// startup, not a recoverable runtime condition.
fn db_path() -> &'static Path {
  DB_PATH
    .get()
    .expect(
      "hwviz_core::infrastructure::database::db::init must be called before get_pool",
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
  let path = db_path();
  if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(sqlx::Error::Io)?;
  }
  let database_url = format!("sqlite:{}", path.to_string_lossy());
  SqlitePool::connect(&database_url).await
}
