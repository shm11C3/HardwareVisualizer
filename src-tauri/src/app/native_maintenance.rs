//! Post-selection maintenance: the checkpoint scheduled after each native
//! expiry pass, and retiring the SQLite source once a later startup has
//! verified the selection.
//!
//! Both are decided in
//! [`docs/development/hardware-archive-duckdb-retention-evidence.md`](../../../../docs/development/hardware-archive-duckdb-retention-evidence.md)
//! and the Design Doc's "Retention and physical allocation" /
//! "Remaining design questions" sections: one explicit `CHECKPOINT` after
//! the daily expiry pass, and a rename in place for retirement rather than a
//! copy or a delete.

use std::path::Path;

use hardviz_core::infrastructure::database::native_database::{
  NativeCancellation, NativeDatabase, NativeDatabaseError,
};

use crate::{log_error, log_info};

/// Checkpoint the native database once, immediately after a daily expiry
/// pass on it.
///
/// This is the whole of the App-owned scheduling decision: Core's
/// [`NativeDatabase::checkpoint`] does the work and documents the measured
/// cost/benefit, and there is exactly one call, exactly once per pass -
/// never per family, never on a timer independent of expiry. See the
/// measured comparison against the engine's own threshold checkpoint on
/// [`NativeDatabase::checkpoint`]'s documentation.
///
/// # Where this is called from
///
/// No call site exists in this App yet: a native expiry pass itself is
/// #2134's dispatch boundary routing scheduled deletion to the native
/// backend, which had not landed when this function was written. This is
/// the seam that pass should call immediately after its last family
/// finishes, the same way [`crate::app::native_conversion`] left the
/// `// #2134 seam:` on `NativeLifecycleOwner` for consumer routing. Fully
/// exercised by this module's own tests in the meantime; see
/// `native_conversion`'s module documentation for why that does not, by
/// itself, satisfy the plain (non-test) build's dead-code analysis.
#[allow(dead_code)]
pub async fn checkpoint_after_expiry(
  database: &NativeDatabase,
) -> Result<(), NativeDatabaseError> {
  let result = database.checkpoint(NativeCancellation::new()).await;
  match &result {
    Ok(()) => log_info!(
      "checkpointed the native database after a daily expiry pass",
      "app::native_maintenance::checkpoint_after_expiry",
      None::<&str>
    ),
    Err(error) => log_error!(
      "failed to checkpoint the native database after a daily expiry pass",
      "app::native_maintenance::checkpoint_after_expiry",
      Some(error.to_string())
    ),
  }
  result
}

/// The renamed name for a retired SQLite source, resolved beside the
/// original path.
fn retired_path(source_database: &Path) -> Option<std::path::PathBuf> {
  let file_name = source_database.file_name()?.to_string_lossy().into_owned();
  Some(source_database.with_file_name(format!("{file_name}.retired")))
}

/// Rename the SQLite source (and its `-wal`/`-shm` sidecars, if present) out
/// of the way, in place, on the same volume.
///
/// Decided 2026-09-13 (Design Doc, "Remaining design questions"): a rename
/// rather than a copy or a delete, because it is atomic and needs no extra
/// disk - the space preflight already budgets for it. Never called on the
/// same startup that produced a selection: the Design Doc calls for
/// retirement after a *later* verified startup, because a rename here would
/// otherwise take away the ordinary SQLite recovery path before any restart
/// has proven the native database opens cleanly on its own.
///
/// Idempotent: a source that is already renamed (or was never there) is not
/// an error, since a later boot may run this again.
///
/// # Precondition this function does not check
///
/// Nothing here may still be writing to `source_database`, since a rename
/// out from under an open SQLite connection is unreliable at best on some
/// platforms. `lib.rs` only calls this on the one startup path where that
/// is true today: when authority is already `NativeSelected` at boot, the
/// SQLite-backed producers (`ArchiveController`, `CoolingRollupController`,
/// `StorageHealthController`, scheduled deletion) are not started for that
/// same boot either - see the code comment where they are started in
/// `lib.rs` - because rerouting them to the selected native database
/// instead is #2134's dispatch boundary, which had not landed when this was
/// written.
pub fn retire_sqlite_source(source_database: &Path) {
  if !source_database.is_file() {
    return;
  }
  let Some(retired) = retired_path(source_database) else {
    log_error!(
      "could not name a retirement path for the SQLite source",
      "app::native_maintenance::retire_sqlite_source",
      Some(source_database.display().to_string())
    );
    return;
  };
  if let Err(error) = std::fs::rename(source_database, &retired) {
    log_error!(
      "failed to retire the SQLite source after a later verified native startup",
      "app::native_maintenance::retire_sqlite_source",
      Some(error.to_string())
    );
    return;
  }
  for suffix in ["-wal", "-shm"] {
    let mut sidecar = source_database.as_os_str().to_os_string();
    sidecar.push(suffix);
    let sidecar = std::path::PathBuf::from(sidecar);
    if !sidecar.is_file() {
      continue;
    }
    let mut retired_sidecar = retired.as_os_str().to_os_string();
    retired_sidecar.push(suffix);
    // Best effort: the database file itself is already safely renamed, and
    // a leftover `-wal`/`-shm` beside the *original* name is debris rather
    // than data loss - nothing opens the retired path expecting them, and
    // nothing reopens the original path expecting the database file that
    // just moved away from under them.
    let _ = std::fs::rename(&sidecar, std::path::PathBuf::from(retired_sidecar));
  }
  log_info!(
    "retired the SQLite source after a later verified native startup",
    "app::native_maintenance::retire_sqlite_source",
    Some(retired.display().to_string())
  );
}

#[cfg(test)]
mod tests {
  use hardviz_core::infrastructure::database::candidate_database::build_candidate_database;
  use hardviz_core::infrastructure::database::migrate;
  use hardviz_core::infrastructure::database::native_database::{
    NativeDatabaseOptions, finalize_candidate_database,
  };
  use sqlx::ConnectOptions;
  use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

  use super::*;
  use crate::infrastructure::database::{migration, native_schema};

  /// A real, finalized native database in a fresh temporary directory -
  /// SQLite through App's own migrations, converted through the production
  /// candidate/finalize calls, matching the fixture philosophy
  /// `core/tests/native_support` and `native_conversion`'s tests already
  /// established. The returned `TempDir` must outlive the database.
  async fn finalized_native_database() -> (tempfile::TempDir, NativeDatabase) {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    let candidate = directory.path().join("candidate.duckdb");
    let native = directory.path().join("hv-database.duckdb");

    let options = SqliteConnectOptions::new()
      .filename(&source)
      .create_if_missing(true)
      .disable_statement_logging();
    let pool = SqlitePoolOptions::new()
      .max_connections(1)
      .connect_with(options)
      .await
      .unwrap();
    migrate::run_on_pool(&pool, migration::get_migrations())
      .await
      .unwrap();
    pool.close().await;

    build_candidate_database(&source, &candidate, migration::get_migrations())
      .await
      .unwrap();
    finalize_candidate_database(&candidate, &native, native_schema::get_native_schema())
      .await
      .unwrap();

    let database = NativeDatabase::open(
      &native,
      NativeDatabaseOptions::new(native_schema::NATIVE_SCHEMA_VERSION),
    )
    .await
    .unwrap();
    (directory, database)
  }

  #[tokio::test]
  async fn checkpoint_after_expiry_checkpoints_the_open_database() {
    let (_directory, database) = finalized_native_database().await;

    // An empty, just-finalized database has nothing pending, so the only
    // claim this proves is that the seam reaches Core's real operation
    // without error - the WAL-flush behavior itself is Core's own
    // `checkpoint_flushes_the_write_ahead_log_and_keeps_the_data` test.
    checkpoint_after_expiry(&database).await.unwrap();

    database.close().await.unwrap();
  }

  #[tokio::test]
  async fn checkpoint_after_expiry_reports_the_underlying_error() {
    let (_directory, database) = finalized_native_database().await;
    database.close().await.unwrap();

    let error = checkpoint_after_expiry(&database).await.unwrap_err();
    assert!(matches!(error, NativeDatabaseError::Closed), "{error:?}");
  }

  #[test]
  fn retiring_an_absent_source_is_a_silent_no_op() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    retire_sqlite_source(&source);
    assert!(!source.exists());
  }

  #[test]
  fn retiring_renames_the_source_and_its_sidecars_in_place() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    std::fs::write(&source, b"sqlite source").unwrap();
    std::fs::write(directory.path().join("hv-database.db-wal"), b"wal").unwrap();
    std::fs::write(directory.path().join("hv-database.db-shm"), b"shm").unwrap();

    retire_sqlite_source(&source);

    assert!(!source.exists());
    assert!(!directory.path().join("hv-database.db-wal").exists());
    assert!(!directory.path().join("hv-database.db-shm").exists());
    let retired = directory.path().join("hv-database.db.retired");
    assert_eq!(std::fs::read(&retired).unwrap(), b"sqlite source");
    assert_eq!(
      std::fs::read(directory.path().join("hv-database.db.retired-wal")).unwrap(),
      b"wal"
    );
    assert_eq!(
      std::fs::read(directory.path().join("hv-database.db.retired-shm")).unwrap(),
      b"shm"
    );
  }

  #[test]
  fn retiring_twice_is_idempotent() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    std::fs::write(&source, b"sqlite source").unwrap();

    retire_sqlite_source(&source);
    assert!(!source.exists());
    // A second call finds nothing to rename and must not error or panic.
    retire_sqlite_source(&source);
    assert_eq!(
      std::fs::read(directory.path().join("hv-database.db.retired")).unwrap(),
      b"sqlite source"
    );
  }

  #[test]
  fn retiring_without_sidecars_renames_only_the_database_file() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    std::fs::write(&source, b"sqlite source").unwrap();

    retire_sqlite_source(&source);

    assert!(!source.exists());
    assert!(directory.path().join("hv-database.db.retired").is_file());
  }
}
