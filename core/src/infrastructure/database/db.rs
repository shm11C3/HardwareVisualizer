use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sqlx::sqlite::SqlitePool;

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
  let path = db_path();
  if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(sqlx::Error::Io)?;
  }
  let database_url = format!("sqlite:{}", path.to_string_lossy());
  SqlitePool::connect(&database_url).await
}
