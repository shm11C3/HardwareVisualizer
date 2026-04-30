use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbStartupError {
  IncompatibleVersion {
    db_max_version: i64,
    app_max_version: i64,
  },
  Other(String),
}

/// Decide whether the on-disk SQLite database is compatible with the
/// running app's migration set.
///
/// Returns `None` when the database is compatible (or doesn't exist /
/// hasn't been migrated yet) and `Some(DbStartupError)` otherwise.
///
/// This function is intentionally Tauri-independent: callers in the App
/// crate pass the resolved DB path and the highest migration version
/// the binary is built with.
pub fn check_db_compatibility(
  db_path: &Path,
  app_max_version: i64,
) -> Option<DbStartupError> {
  if !db_path.exists() {
    return None;
  }

  let database_url = format!("sqlite:{}", db_path.to_string_lossy());

  // Core has no Tauri runtime, so spin up a single-threaded tokio
  // runtime for this synchronous one-shot query. This function runs
  // before the Tauri builder starts, so there is no enclosing runtime
  // to clash with.
  let runtime = match tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
  {
    Ok(rt) => rt,
    Err(e) => {
      return Some(DbStartupError::Other(format!(
        "Failed to build runtime for DB preflight: {e}"
      )));
    }
  };

  let result = runtime.block_on(query_max_migration_version(&database_url));

  match result {
    Ok(Some(db_max_version)) if db_max_version > app_max_version => {
      Some(DbStartupError::IncompatibleVersion {
        db_max_version,
        app_max_version,
      })
    }
    Ok(_) => None,
    Err(e) => {
      let err_msg = e.to_string();
      // The first migration hasn't run yet — the table simply doesn't exist
      // in this database. That's compatible.
      if err_msg.contains("no such table") {
        None
      } else {
        Some(DbStartupError::Other(err_msg))
      }
    }
  }
}

async fn query_max_migration_version(
  database_url: &str,
) -> Result<Option<i64>, sqlx::Error> {
  let pool = SqlitePool::connect(database_url).await?;
  let row = sqlx::query(
    "SELECT MAX(version) as max_version FROM _sqlx_migrations WHERE success = 1",
  )
  .fetch_optional(&pool)
  .await;
  pool.close().await;

  match row {
    Ok(Some(r)) => Ok(r.get("max_version")),
    Ok(None) => Ok(None),
    Err(e) => Err(e),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::sqlite::SqlitePool;
  use tempfile::NamedTempFile;

  async fn create_test_db(path: &Path) -> SqlitePool {
    let url = format!("sqlite:{}", path.to_string_lossy());
    let pool = SqlitePool::connect(&url).await.unwrap();
    sqlx::query(
      "CREATE TABLE _sqlx_migrations (
        version BIGINT PRIMARY KEY,
        description TEXT NOT NULL,
        installed_on TEXT NOT NULL DEFAULT (datetime('now')),
        success BOOLEAN NOT NULL,
        checksum BLOB NOT NULL,
        execution_time BIGINT NOT NULL
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
  }

  async fn insert_migration(pool: &SqlitePool, version: i64, success: bool) {
    sqlx::query(
      "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
       VALUES (?, 'test', ?, X'00', 0)",
    )
    .bind(version)
    .bind(success)
    .execute(pool)
    .await
    .unwrap();
  }

  fn run<F: std::future::Future<Output = T>, T>(f: F) -> T {
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(f)
  }

  #[test]
  fn query_returns_err_when_no_migration_table() {
    let tmp = NamedTempFile::new().unwrap();
    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    run(async {
      let _pool = SqlitePool::connect(&url).await.unwrap();
    });

    let result = run(query_max_migration_version(&url));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no such table"));
  }

  #[test]
  fn query_returns_none_when_table_empty() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let pool = create_test_db(tmp.path()).await;
      pool.close().await;
    });

    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    let result = run(query_max_migration_version(&url)).unwrap();
    assert_eq!(result, None);
  }

  #[test]
  fn query_returns_max_successful_version() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 2, true).await;
      insert_migration(&pool, 3, true).await;
      pool.close().await;
    });

    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    let result = run(query_max_migration_version(&url)).unwrap();
    assert_eq!(result, Some(3));
  }

  #[test]
  fn query_ignores_failed_migrations() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 2, true).await;
      insert_migration(&pool, 3, false).await;
      pool.close().await;
    });

    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    let result = run(query_max_migration_version(&url)).unwrap();
    assert_eq!(result, Some(2));
  }

  #[test]
  fn compatible_when_db_file_does_not_exist() {
    let path = Path::new("/nonexistent/path/to/db.sqlite");
    assert_eq!(check_db_compatibility(path, 5), None);
  }

  #[test]
  fn compatible_when_db_version_equals_app_version() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 5, true).await;
      pool.close().await;
    });

    assert_eq!(check_db_compatibility(tmp.path(), 5), None);
  }

  #[test]
  fn compatible_when_db_version_lower_than_app() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 3, true).await;
      pool.close().await;
    });

    assert_eq!(check_db_compatibility(tmp.path(), 5), None);
  }

  #[test]
  fn incompatible_when_db_version_higher_than_app() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 6, true).await;
      pool.close().await;
    });

    assert_eq!(
      check_db_compatibility(tmp.path(), 5),
      Some(DbStartupError::IncompatibleVersion {
        db_max_version: 6,
        app_max_version: 5,
      })
    );
  }

  #[test]
  fn compatible_when_no_migration_table() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let url = format!("sqlite:{}", tmp.path().to_string_lossy());
      let _pool = SqlitePool::connect(&url).await.unwrap();
    });

    assert_eq!(check_db_compatibility(tmp.path(), 5), None);
  }

  #[test]
  fn compatible_when_migration_table_empty() {
    let tmp = NamedTempFile::new().unwrap();
    run(async {
      let pool = create_test_db(tmp.path()).await;
      pool.close().await;
    });

    assert_eq!(check_db_compatibility(tmp.path(), 5), None);
  }
}
