use crate::utils;
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

/// Check if the database schema is compatible with this app version.
///
/// Returns `None` if the database is compatible or doesn't exist yet.
/// Returns `Some(DbStartupError)` if the database schema is incompatible.
pub fn check_db_compatibility() -> Option<DbStartupError> {
  let db_path = utils::file::get_app_data_dir("hv-database.db");
  let app_max_version = super::migration::get_max_migration_version();
  check_db_compatibility_at(&db_path, app_max_version)
}

fn check_db_compatibility_at(
  db_path: &Path,
  app_max_version: i64,
) -> Option<DbStartupError> {
  if !db_path.exists() {
    return None;
  }

  let database_url = format!("sqlite:{}", db_path.to_string_lossy());

  let result = tauri::async_runtime::block_on(async {
    query_max_migration_version(&database_url).await
  });

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
      // Table doesn't exist yet — first migration hasn't run
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

  #[tokio::test]
  async fn query_returns_none_when_no_migration_table() {
    let tmp = NamedTempFile::new().unwrap();
    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    // Create an empty DB (no _sqlx_migrations table)
    let _pool = SqlitePool::connect(&url).await.unwrap();

    let result = query_max_migration_version(&url).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no such table"));
  }

  #[tokio::test]
  async fn query_returns_none_when_table_empty() {
    let tmp = NamedTempFile::new().unwrap();
    let pool = create_test_db(tmp.path()).await;
    pool.close().await;

    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    let result = query_max_migration_version(&url).await.unwrap();
    // MAX() on empty table returns NULL → None
    assert_eq!(result, None);
  }

  #[tokio::test]
  async fn query_returns_max_successful_version() {
    let tmp = NamedTempFile::new().unwrap();
    let pool = create_test_db(tmp.path()).await;
    insert_migration(&pool, 1, true).await;
    insert_migration(&pool, 2, true).await;
    insert_migration(&pool, 3, true).await;
    pool.close().await;

    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    let result = query_max_migration_version(&url).await.unwrap();
    assert_eq!(result, Some(3));
  }

  #[tokio::test]
  async fn query_ignores_failed_migrations() {
    let tmp = NamedTempFile::new().unwrap();
    let pool = create_test_db(tmp.path()).await;
    insert_migration(&pool, 1, true).await;
    insert_migration(&pool, 2, true).await;
    insert_migration(&pool, 3, false).await; // failed
    pool.close().await;

    let url = format!("sqlite:{}", tmp.path().to_string_lossy());
    let result = query_max_migration_version(&url).await.unwrap();
    assert_eq!(result, Some(2));
  }

  #[test]
  fn compatible_when_db_file_does_not_exist() {
    let path = Path::new("/nonexistent/path/to/db.sqlite");
    assert_eq!(check_db_compatibility_at(path, 5), None);
  }

  #[test]
  fn compatible_when_db_version_equals_app_version() {
    let tmp = NamedTempFile::new().unwrap();
    tauri::async_runtime::block_on(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 5, true).await;
      pool.close().await;
    });

    assert_eq!(check_db_compatibility_at(tmp.path(), 5), None);
  }

  #[test]
  fn compatible_when_db_version_lower_than_app() {
    let tmp = NamedTempFile::new().unwrap();
    tauri::async_runtime::block_on(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 3, true).await;
      pool.close().await;
    });

    assert_eq!(check_db_compatibility_at(tmp.path(), 5), None);
  }

  #[test]
  fn incompatible_when_db_version_higher_than_app() {
    let tmp = NamedTempFile::new().unwrap();
    tauri::async_runtime::block_on(async {
      let pool = create_test_db(tmp.path()).await;
      insert_migration(&pool, 1, true).await;
      insert_migration(&pool, 6, true).await;
      pool.close().await;
    });

    assert_eq!(
      check_db_compatibility_at(tmp.path(), 5),
      Some(DbStartupError::IncompatibleVersion {
        db_max_version: 6,
        app_max_version: 5,
      })
    );
  }

  #[test]
  fn compatible_when_no_migration_table() {
    let tmp = NamedTempFile::new().unwrap();
    tauri::async_runtime::block_on(async {
      let url = format!("sqlite:{}", tmp.path().to_string_lossy());
      let _pool = SqlitePool::connect(&url).await.unwrap();
    });

    assert_eq!(check_db_compatibility_at(tmp.path(), 5), None);
  }

  #[test]
  fn compatible_when_migration_table_empty() {
    let tmp = NamedTempFile::new().unwrap();
    tauri::async_runtime::block_on(async {
      let pool = create_test_db(tmp.path()).await;
      pool.close().await;
    });

    assert_eq!(check_db_compatibility_at(tmp.path(), 5), None);
  }
}
