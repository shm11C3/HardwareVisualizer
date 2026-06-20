//! Schema migration runner executed against Core's own connection pool.
//!
//! Core owns the database path and pool ([`super::db`]), so it also owns
//! applying schema migrations. App supplies the ordered migration set
//! (see `src-tauri/src/infrastructure/database/migration.rs`) and calls
//! [`run`] once at startup, before any persistence worker writes.
//!
//! This replaces the previous reliance on `tauri-plugin-sql` to apply
//! migrations, which never ran: the database is never loaded through the
//! plugin (there is no `preload` entry and the frontend never calls
//! `Database.load`), so the registered migrations stayed dormant and any
//! table added by a newer migration (e.g. `storage_devices`) was missing.

use std::future::Future;
use std::pin::Pin;

use sqlx::error::BoxDynError;
use sqlx::migrate::{
  Migration as SqlxMigration, MigrationSource, MigrationType, Migrator,
};
use sqlx::sqlite::SqlitePool;

use super::db;

/// One forward (`Up`) schema migration.
///
/// Deliberately mirrors the historical `tauri-plugin-sql` migration shape
/// so the `sqlx` checksum of already-applied migrations keeps matching:
/// the checksum is derived from `sql` alone, and migrations are resolved
/// as [`MigrationType::ReversibleUp`] exactly as the plugin did. Existing
/// databases carry `_sqlx_migrations` rows written by the plugin, so the
/// SQL text of previously-applied versions must stay byte-identical.
#[derive(Debug, Clone)]
pub struct SchemaMigration {
  pub version: i64,
  pub description: &'static str,
  pub sql: &'static str,
}

#[derive(Debug)]
struct SchemaMigrationSource(Vec<SchemaMigration>);

impl MigrationSource<'static> for SchemaMigrationSource {
  fn resolve(
    self,
  ) -> Pin<
    Box<dyn Future<Output = Result<Vec<SqlxMigration>, BoxDynError>> + Send + 'static>,
  > {
    Box::pin(async move {
      let migrations = self
        .0
        .into_iter()
        .map(|m| {
          SqlxMigration::new(
            m.version,
            m.description.into(),
            MigrationType::ReversibleUp,
            m.sql.into(),
            false,
          )
        })
        .collect();
      Ok(migrations)
    })
  }
}

/// Apply all pending migrations to the configured database.
///
/// Idempotent: `sqlx` records applied versions in `_sqlx_migrations` and
/// skips them on subsequent runs. Must be called after [`db::init`] and
/// before persistence workers start.
pub async fn run(migrations: Vec<SchemaMigration>) -> Result<(), String> {
  let pool = db::get_pool()
    .await
    .map_err(|e| format!("Failed to open database for migration: {e}"))?;
  run_on_pool(&pool, migrations).await
}

/// Apply `migrations` to an already-open pool.
///
/// [`run`] is the production entry point — it opens Core's configured
/// pool via [`db::get_pool`]. This variant lets callers supply their own
/// pool (notably tests that run against a temporary database) without
/// touching the process-wide [`db::init`] path.
pub async fn run_on_pool(
  pool: &SqlitePool,
  migrations: Vec<SchemaMigration>,
) -> Result<(), String> {
  let migrator = Migrator::new(SchemaMigrationSource(migrations))
    .await
    .map_err(|e| format!("Failed to build migrator: {e}"))?;
  migrator
    .run(pool)
    .await
    .map_err(|e| format!("Failed to run migrations: {e}"))?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::Row;
  use sqlx::sqlite::SqlitePool;
  use tempfile::NamedTempFile;

  fn migrations() -> Vec<SchemaMigration> {
    vec![
      SchemaMigration {
        version: 1,
        description: "create_a",
        sql: "CREATE TABLE a (id INTEGER PRIMARY KEY);",
      },
      SchemaMigration {
        version: 2,
        description: "create_b",
        sql: "CREATE TABLE b (id INTEGER PRIMARY KEY);",
      },
    ]
  }

  async fn file_pool() -> (NamedTempFile, SqlitePool) {
    let file = NamedTempFile::new().unwrap();
    let url = format!("sqlite:{}", file.path().to_string_lossy());
    let pool = SqlitePool::connect(&url).await.unwrap();
    (file, pool)
  }

  async fn max_applied_version(pool: &SqlitePool) -> i64 {
    let row: (Option<i64>,) =
      sqlx::query_as("SELECT MAX(version) FROM _sqlx_migrations WHERE success = 1")
        .fetch_one(pool)
        .await
        .unwrap();
    row.0.unwrap_or(0)
  }

  #[tokio::test]
  async fn applies_pending_migrations() {
    let (_file, pool) = file_pool().await;

    run_on_pool(&pool, migrations()).await.unwrap();

    let tables: Vec<String> = sqlx::query(
      "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('a', 'b') ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.get::<String, _>("name"))
    .collect();

    assert_eq!(tables, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(max_applied_version(&pool).await, 2);
  }

  #[tokio::test]
  async fn reapplying_only_runs_new_migrations() {
    let (_file, pool) = file_pool().await;

    // Simulates the real upgrade path: an existing DB already at an older
    // version is brought forward when a newer migration is appended.
    run_on_pool(&pool, migrations()).await.unwrap();

    let mut next = migrations();
    next.push(SchemaMigration {
      version: 3,
      description: "create_c",
      sql: "CREATE TABLE c (id INTEGER PRIMARY KEY);",
    });

    // Re-running with the unchanged v1/v2 SQL must not trip a checksum
    // mismatch, and only v3 is applied.
    run_on_pool(&pool, next).await.unwrap();

    let has_c: (i64,) = sqlx::query_as(
      "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = 'c'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(has_c.0, 1);
    assert_eq!(max_applied_version(&pool).await, 3);
  }
}
