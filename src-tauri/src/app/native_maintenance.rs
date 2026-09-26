//! Post-selection maintenance: retiring the SQLite source once a later
//! startup has verified the selection.
//!
//! Decided in
//! [`docs/development/hardware-archive-duckdb-retention-evidence.md`](../../../../docs/development/hardware-archive-duckdb-retention-evidence.md)
//! and the Design Doc's "Retention and physical allocation" /
//! "Remaining design questions" sections: a rename in place for retirement
//! rather than a copy or a delete.
//!
//! The other maintenance decision documented there - one explicit
//! `CHECKPOINT` after the daily expiry pass - is no longer this module's:
//! #2134's dispatch boundary is the only thing that still holds a native
//! database open once it has been adopted (see the `// #2134 seam:` on
//! [`crate::app::native_conversion::adopt_selected_database_via_dispatch`]),
//! so the call lives beside the expiry pass itself, in Core's
//! `persistence::archive::cleanup_old_data`, through
//! `hardviz_core::infrastructure::database::dispatch::checkpoint`.

use std::path::Path;

use hardviz_core::infrastructure::database::native_database::AuthorityPaths;

use crate::{log_error, log_info};

/// The renamed name for a retired SQLite source, resolved beside the
/// original path.
fn retired_path(source_database: &Path) -> Option<std::path::PathBuf> {
  let file_name = source_database.file_name()?.to_string_lossy().into_owned();
  Some(source_database.with_file_name(format!("{file_name}.retired")))
}

/// One removal step's failure during
/// [`discard_native_authority_files`]. Carries the path and a short
/// description of what step failed so
/// [`crate::app::startup::reset_database_and_restart`] can show a specific
/// dialog message instead of continuing past a file it could not remove.
#[derive(Debug)]
pub struct DiscardArtifactError {
  path: std::path::PathBuf,
  description: &'static str,
  source: std::io::Error,
}

impl std::fmt::Display for DiscardArtifactError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      formatter,
      "failed to remove {} ({}): {}",
      self.description,
      self.path.display(),
      self.source
    )
  }
}

impl std::error::Error for DiscardArtifactError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    Some(&self.source)
  }
}

/// Remove every native-database-lifecycle artifact Reset owns: the
/// finalized/selected native file and its write-ahead log, the authority
/// marker, and any interrupted conversion work directories. Called by
/// [`crate::app::startup::reset_database_and_restart`] before it removes the
/// SQLite source itself; see that function for the full order and for why
/// the caller must close the dispatch boundary's native owner first.
///
/// # Per-lifecycle-state behaviour
///
/// This function does not branch on [`crate::app::native_lifecycle::DatabaseLifecycleState`]:
/// Reset is a full, state-independent discard. Every state
/// (`SqliteAuthoritative`, `ConversionRecoverable`, `NativeAuthoritative`,
/// `ActionRequired`) ends up with the native database, its write-ahead log,
/// the marker and any conversion work directories removed - whichever of
/// them happen to exist for that state - so the profile converges on the
/// same clean, artifact-free state every time, and the next
/// `inspect_authority` reports it as a fresh install.
///
/// # Ordering: native database before marker, and stop at the first failure
///
/// The native database is removed **before** the marker, mirroring
/// [`hardviz_core::infrastructure::database::native_database::select_native_database`]'s
/// own write order (native metadata first, marker second) in reverse. If the
/// process dies (or a step fails) between the two removals here, the only
/// state it can leave is "the marker still names a native database that is
/// already gone" - which `inspect_authority` reports as
/// [`hardviz_core::infrastructure::database::native_database::AuthorityInconsistency::MarkerWithoutNativeDatabase`]
/// and refuses to guess at
/// ([`hardviz_core::infrastructure::database::native_database::AuthorityRecovery::StopAndReport`]).
/// The reverse order would be unsafe: removing the marker first while the
/// native database still says `state = 'selected'` recreates exactly the one
/// state `inspect_startup_authority` auto-repairs
/// (`SelectedWithoutMarker`) - a later startup (or, worse, this same reset
/// continuing past the failure to delete the SQLite source next) would
/// silently rewrite the marker from the database's own metadata and undo the
/// reset it was in the middle of performing.
///
/// Every step here therefore stops at its own first failure and returns
/// immediately, rather than logging and continuing: a native file this
/// cannot remove (locked, permissions) must leave the marker - and every
/// step after it - untouched, or the caller's later SQLite removal would run
/// on top of a half-discarded, silently misleading state. The caller
/// ([`crate::app::startup::reset_database_and_restart`]) reports the error
/// and stops before touching the SQLite source or restarting; an interrupted
/// or refused reset is safe to resume, because both startup dialogs keep
/// offering Reset, and a second full pass either finishes the job or fails
/// at the same, now-diagnosable step.
///
/// # What is deliberately not removed
///
/// A `.retired` SQLite copy ([`retired_path`]) is left in place. It already
/// represents history that survived one migration (native conversion); Reset
/// discarding the *currently* authoritative or recoverable data should not
/// also silently discard a separate, older recovery copy on top of that.
/// Leaving it costs only disk space, is fully reversible, and - unlike every
/// path this function does touch - a `.retired`-suffixed name is never read
/// by `observe_authority`/`inspect_authority`, so keeping it can never make a
/// freshly reset profile look anything but clean.
pub fn discard_native_authority_files(
  paths: &AuthorityPaths,
  workspace: &Path,
) -> Result<(), DiscardArtifactError> {
  let native_write_ahead_log = {
    let mut path = paths.native_database.as_os_str().to_os_string();
    path.push(".wal");
    std::path::PathBuf::from(path)
  };
  remove_required_file(&paths.native_database, "the native database file")?;
  remove_required_file(
    &native_write_ahead_log,
    "the native database write-ahead log",
  )?;
  remove_required_file(&paths.marker, "the native authority marker")?;
  discard_conversion_work_directories(workspace)
}

fn remove_required_file(
  path: &Path,
  description: &'static str,
) -> Result<(), DiscardArtifactError> {
  match std::fs::remove_file(path) {
    Ok(()) => Ok(()),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(source) => Err(DiscardArtifactError {
      path: path.to_owned(),
      description,
      source,
    }),
  }
}

/// Remove every `.hardwarevisualizer-duckdb-*` directory already in
/// `workspace`, stopping at the first one this cannot remove.
///
/// Deliberately not [`crate::app::native_conversion::discard_stale_conversion_work`]:
/// that sweep is best-effort by design (a directory it cannot remove is
/// logged and left in place so a fresh conversion attempt is not blocked by
/// debris it does not even read) - see its own documentation. Reset needs
/// the opposite policy, matching every other step in
/// [`discard_native_authority_files`]: stop and report rather than continue
/// past a removal this process could not perform.
fn discard_conversion_work_directories(
  workspace: &Path,
) -> Result<(), DiscardArtifactError> {
  let entries = match std::fs::read_dir(workspace) {
    Ok(entries) => entries,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
    Err(source) => {
      return Err(DiscardArtifactError {
        path: workspace.to_owned(),
        description: "the database directory",
        source,
      });
    }
  };
  for entry in entries {
    let entry = entry.map_err(|source| DiscardArtifactError {
      path: workspace.to_owned(),
      description: "the database directory",
      source,
    })?;
    let file_name = entry.file_name();
    let file_name = file_name.to_string_lossy();
    if !file_name.starts_with(crate::app::native_conversion::WORK_DEBRIS_PREFIX)
      || file_name.starts_with(
        hardviz_core::infrastructure::database::native_database::LEGACY_RUNTIME_SPILL_DIRECTORY_PREFIX,
      )
      || !entry.file_type().is_ok_and(|kind| kind.is_dir())
    {
      continue;
    }
    let path = entry.path();
    std::fs::remove_dir_all(&path).map_err(|source| DiscardArtifactError {
      path,
      description: "an interrupted conversion work directory",
      source,
    })?;
  }
  Ok(())
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
/// is true today: when authority is already `NativeSelected` at boot, and
/// only *before* the database-backed producers (`ArchiveController`,
/// `CoolingRollupController`, `StorageHealthController`, scheduled
/// deletion) are started for that same boot - see the code comment where
/// they are started in `lib.rs`. Migrations are also skipped on that same
/// path (nothing else recreates `source_database`), and #2134's dispatch
/// boundary - adopted earlier, in `resolve_native_authority` - opens only
/// the native `.duckdb` file, never the SQLite source, so nothing holds
/// `source_database` open by the time this runs.
///
/// # The write-ahead log is part of the operation
///
/// A SQLite `-wal` file can hold committed pages the main file does not have
/// yet (Core closes its pool after every operation, so this is normally only
/// true after a crash mid-write). A retired main file without it would be an
/// incomplete recovery copy that still looks complete, so the log moves
/// *first*, and a failure to move the main file afterwards puts it back.
/// That order is also the crash-safe one: a process killed between the two
/// renames leaves the source in place, and the next boot's call finishes the
/// job. The `-shm` file is only an index SQLite rebuilds, so it stays best
/// effort.
pub fn retire_sqlite_source(source_database: &Path) {
  retire_with(source_database, |from, to| std::fs::rename(from, to));
}

fn sidecar_path(database: &Path, suffix: &str) -> std::path::PathBuf {
  let mut path = database.as_os_str().to_os_string();
  path.push(suffix);
  std::path::PathBuf::from(path)
}

/// [`retire_sqlite_source`] with the rename injected, so a test can make one
/// specific rename fail.
fn retire_with(
  source_database: &Path,
  rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) {
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
  // Defense in depth beside the caller's own ordering fix (lib.rs skips
  // migrations, which is what could recreate `source_database` here, once
  // authority is already `NativeAuthoritative`): `std::fs::rename` silently
  // overwrites an existing destination, and the retired file is the only
  // remaining copy of history predating the conversion. Refusing to
  // overwrite it is cheap insurance against a caller ever running this
  // twice on data that regenerated the source in between.
  if retired.exists() {
    log_error!(
      "refusing to retire the SQLite source: a retired copy already exists",
      "app::native_maintenance::retire_sqlite_source",
      Some(retired.display().to_string())
    );
    return;
  }
  let write_ahead_log = sidecar_path(source_database, "-wal");
  let retired_write_ahead_log = sidecar_path(&retired, "-wal");
  let moved_write_ahead_log = write_ahead_log.is_file();
  if moved_write_ahead_log {
    if retired_write_ahead_log.exists() {
      log_error!(
        "refusing to retire the SQLite source: a retired write-ahead log already exists",
        "app::native_maintenance::retire_sqlite_source",
        Some(retired_write_ahead_log.display().to_string())
      );
      return;
    }
    if let Err(error) = rename(&write_ahead_log, &retired_write_ahead_log) {
      log_error!(
        "failed to retire the SQLite write-ahead log; the source was left in place",
        "app::native_maintenance::retire_sqlite_source",
        Some(error.to_string())
      );
      return;
    }
  }
  if let Err(error) = rename(source_database, &retired) {
    let restored = !moved_write_ahead_log
      || rename(&retired_write_ahead_log, &write_ahead_log).is_ok();
    log_error!(
      "failed to retire the SQLite source after a later verified native startup",
      "app::native_maintenance::retire_sqlite_source",
      Some(format!(
        "{error}; write-ahead log back beside the source: {restored}"
      ))
    );
    return;
  }
  let shared_memory = sidecar_path(source_database, "-shm");
  if shared_memory.is_file() {
    // Best effort: SQLite rebuilds this index from the write-ahead log, so a
    // leftover beside the original name is debris rather than data loss.
    let _ = rename(&shared_memory, &sidecar_path(&retired, "-shm"));
  }
  log_info!(
    "retired the SQLite source after a later verified native startup",
    "app::native_maintenance::retire_sqlite_source",
    Some(retired.display().to_string())
  );
}

#[cfg(test)]
mod tests {
  use super::*;

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

  /// The scenario the caller's own ordering fix (skipping migrations once
  /// native authority is already selected) is meant to prevent from ever
  /// happening: something recreated `source_database` after it was already
  /// retired once. Refusing to overwrite the retired file is the second
  /// line of defense.
  #[test]
  fn retiring_never_overwrites_an_existing_retired_copy() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    std::fs::write(&source, b"original history").unwrap();

    retire_sqlite_source(&source);
    assert!(!source.exists());

    // Something (a bug, a migration that should not have run) recreated an
    // empty source at the same path.
    std::fs::write(&source, b"freshly recreated, empty").unwrap();

    retire_sqlite_source(&source);

    // The recreated file is left in place - retirement refused rather than
    // silently overwriting the real history.
    assert_eq!(std::fs::read(&source).unwrap(), b"freshly recreated, empty");
    assert_eq!(
      std::fs::read(directory.path().join("hv-database.db.retired")).unwrap(),
      b"original history"
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

  /// A real rename for every path except `failing`, which errors.
  fn rename_failing_for(
    failing: std::path::PathBuf,
  ) -> impl Fn(&Path, &Path) -> std::io::Result<()> {
    move |from, to| {
      if from == failing {
        Err(std::io::Error::other("forced rename failure"))
      } else {
        std::fs::rename(from, to)
      }
    }
  }

  #[test]
  fn a_failed_database_rename_puts_the_write_ahead_log_back() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    let write_ahead_log = directory.path().join("hv-database.db-wal");
    std::fs::write(&source, b"sqlite source").unwrap();
    std::fs::write(&write_ahead_log, b"committed pages").unwrap();

    retire_with(&source, rename_failing_for(source.clone()));

    // Nothing is retired, and the source still has the log that completes it.
    assert_eq!(std::fs::read(&source).unwrap(), b"sqlite source");
    assert_eq!(std::fs::read(&write_ahead_log).unwrap(), b"committed pages");
    assert!(!directory.path().join("hv-database.db.retired").exists());
    assert!(!directory.path().join("hv-database.db.retired-wal").exists());
  }

  #[test]
  fn a_failed_write_ahead_log_rename_leaves_the_source_in_place() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    let write_ahead_log = directory.path().join("hv-database.db-wal");
    std::fs::write(&source, b"sqlite source").unwrap();
    std::fs::write(&write_ahead_log, b"committed pages").unwrap();

    retire_with(&source, rename_failing_for(write_ahead_log.clone()));

    assert_eq!(std::fs::read(&source).unwrap(), b"sqlite source");
    assert_eq!(std::fs::read(&write_ahead_log).unwrap(), b"committed pages");
    assert!(!directory.path().join("hv-database.db.retired").exists());
  }

  #[test]
  fn a_retirement_interrupted_after_the_log_moved_finishes_on_the_next_call() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    std::fs::write(&source, b"sqlite source").unwrap();
    // What a process killed between the two renames leaves behind.
    std::fs::write(
      directory.path().join("hv-database.db.retired-wal"),
      b"committed pages",
    )
    .unwrap();

    retire_sqlite_source(&source);

    assert!(!source.exists());
    assert_eq!(
      std::fs::read(directory.path().join("hv-database.db.retired")).unwrap(),
      b"sqlite source"
    );
    assert_eq!(
      std::fs::read(directory.path().join("hv-database.db.retired-wal")).unwrap(),
      b"committed pages"
    );
  }

  mod reset {
    use hardviz_core::infrastructure::database::candidate_database::build_candidate_database;
    use hardviz_core::infrastructure::database::migrate;
    use hardviz_core::infrastructure::database::native_database::{
      AUTHORITY_MARKER_FILE_NAME, AuthorityInconsistency, AuthorityRecovery,
      AuthorityState, finalize_candidate_database, inspect_authority, observe_authority,
      reconcile_native_database, select_native_database,
    };
    use sqlx::ConnectOptions;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;

    fn paths(directory: &Path) -> AuthorityPaths {
      AuthorityPaths {
        source_database: directory.join("hv-database.db"),
        native_database: directory.join("hv-database.duckdb"),
        marker: directory.join(AUTHORITY_MARKER_FILE_NAME),
      }
    }

    /// A real migrated SQLite source, matching the fixture philosophy used
    /// throughout the native-database tests: nothing here hand-writes a
    /// database file.
    async fn write_sqlite_source(paths: &AuthorityPaths) {
      let options = SqliteConnectOptions::new()
        .filename(&paths.source_database)
        .create_if_missing(true)
        .disable_statement_logging();
      let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
      migrate::run_on_pool(
        &pool,
        crate::infrastructure::database::migration::get_migrations(),
      )
      .await
      .unwrap();
      pool.close().await;
    }

    /// A durably selected native database beside its SQLite source, built
    /// through the real candidate/finalize/reconcile/select pipeline.
    async fn select_native(directory: &Path) -> AuthorityPaths {
      let paths = paths(directory);
      write_sqlite_source(&paths).await;
      let candidate = directory.join("candidate.duckdb");
      build_candidate_database(
        &paths.source_database,
        &candidate,
        crate::infrastructure::database::migration::get_migrations(),
      )
      .await
      .unwrap();
      finalize_candidate_database(
        &candidate,
        &paths.native_database,
        crate::infrastructure::database::native_schema::get_native_schema(),
      )
      .await
      .unwrap();
      let (_report, verified) = reconcile_native_database(
        &paths.source_database,
        &paths.native_database,
        crate::infrastructure::database::migration::get_migrations(),
        crate::infrastructure::database::native_schema::get_native_schema(),
      )
      .await
      .unwrap();
      select_native_database(paths.clone(), verified)
        .await
        .unwrap();
      paths
    }

    fn schema_version() -> u32 {
      crate::infrastructure::database::native_schema::NATIVE_SCHEMA_VERSION
    }

    fn assert_fresh(paths: &AuthorityPaths) {
      assert_eq!(
        inspect_authority(&observe_authority(paths, schema_version())),
        AuthorityState::SqliteAuthoritative,
        "a reset profile must read as a fresh install with no artifacts left"
      );
      assert!(!paths.native_database.is_file());
      assert!(!paths.marker.is_file());
    }

    #[tokio::test]
    async fn reset_from_native_authoritative_leaves_a_clean_fresh_profile() {
      let directory = tempfile::tempdir().unwrap();
      let paths = select_native(directory.path()).await;
      assert_eq!(
        inspect_authority(&observe_authority(&paths, schema_version())),
        AuthorityState::NativeSelected
      );

      discard_native_authority_files(&paths, directory.path()).unwrap();
      std::fs::remove_file(&paths.source_database).unwrap();

      assert_fresh(&paths);
    }

    #[tokio::test]
    async fn reset_from_conversion_recoverable_leaves_a_clean_fresh_profile() {
      let directory = tempfile::tempdir().unwrap();
      let paths = paths(directory.path());
      write_sqlite_source(&paths).await;
      let candidate = directory.path().join("candidate.duckdb");
      build_candidate_database(
        &paths.source_database,
        &candidate,
        crate::infrastructure::database::migration::get_migrations(),
      )
      .await
      .unwrap();
      finalize_candidate_database(
        &candidate,
        &paths.native_database,
        crate::infrastructure::database::native_schema::get_native_schema(),
      )
      .await
      .unwrap();
      assert_eq!(
        inspect_authority(&observe_authority(&paths, schema_version())),
        AuthorityState::FinalizedUnselected
      );

      discard_native_authority_files(&paths, directory.path()).unwrap();
      std::fs::remove_file(&paths.source_database).unwrap();

      assert_fresh(&paths);
    }

    #[test]
    fn reset_from_sqlite_authoritative_leaves_a_clean_fresh_profile() {
      let directory = tempfile::tempdir().unwrap();
      let paths = paths(directory.path());
      std::fs::write(&paths.source_database, b"sqlite source").unwrap();
      assert_eq!(
        inspect_authority(&observe_authority(&paths, schema_version())),
        AuthorityState::SqliteAuthoritative
      );

      discard_native_authority_files(&paths, directory.path()).unwrap();
      std::fs::remove_file(&paths.source_database).unwrap();

      assert_fresh(&paths);
    }

    #[tokio::test]
    async fn reset_from_action_required_leaves_a_clean_fresh_profile() {
      let directory = tempfile::tempdir().unwrap();
      let paths = select_native(directory.path()).await;
      // Corrupt the marker so startup reports an unresolved disagreement
      // (`MarkerNamesAnotherDatabase`) rather than a clean selection - the
      // `ActionRequired` case reset must also resolve.
      let mut marker: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&paths.marker).unwrap()).unwrap();
      marker["native_database_file_name"] = serde_json::json!("other.duckdb");
      std::fs::write(&paths.marker, serde_json::to_vec(&marker).unwrap()).unwrap();
      assert!(matches!(
        inspect_authority(&observe_authority(&paths, schema_version())),
        AuthorityState::Inconsistent {
          reason: AuthorityInconsistency::MarkerNamesAnotherDatabase,
          ..
        }
      ));

      discard_native_authority_files(&paths, directory.path()).unwrap();
      std::fs::remove_file(&paths.source_database).unwrap();

      assert_fresh(&paths);
    }

    #[tokio::test]
    async fn reset_discards_interrupted_conversion_work_directories() {
      let directory = tempfile::tempdir().unwrap();
      let paths = paths(directory.path());
      write_sqlite_source(&paths).await;
      let work = directory
        .path()
        .join(".hardwarevisualizer-duckdb-driver-leftover");
      std::fs::create_dir(&work).unwrap();
      assert_eq!(
        inspect_authority(&observe_authority(&paths, schema_version())),
        AuthorityState::ConversionInProgress { resumable: false }
      );

      discard_native_authority_files(&paths, directory.path()).unwrap();
      std::fs::remove_file(&paths.source_database).unwrap();

      assert!(!work.exists());
      assert_fresh(&paths);
    }

    #[test]
    fn reset_preserves_legacy_runtime_spill_directories() {
      let directory = tempfile::tempdir().unwrap();
      let paths = paths(directory.path());
      std::fs::write(&paths.source_database, b"sqlite source").unwrap();
      let legacy_spill = directory.path().join(format!(
        "{}crashed",
        hardviz_core::infrastructure::database::native_database::LEGACY_RUNTIME_SPILL_DIRECTORY_PREFIX
      ));
      std::fs::create_dir(&legacy_spill).unwrap();
      let spill_artifact = legacy_spill.join("spill-file");
      std::fs::write(&spill_artifact, b"owned by another native database").unwrap();

      discard_native_authority_files(&paths, directory.path()).unwrap();
      std::fs::remove_file(&paths.source_database).unwrap();

      assert!(legacy_spill.is_dir());
      assert_eq!(
        std::fs::read(spill_artifact).unwrap(),
        b"owned by another native database"
      );
      assert_fresh(&paths);
    }

    /// The crash-safety proof: removing the native database file (step 1 of
    /// [`discard_native_authority_files`]) without reaching the marker
    /// removal (step 2) must never be silently repaired back into a
    /// selection - see the function's own "Ordering" documentation.
    #[tokio::test]
    async fn an_interrupted_reset_stopped_after_the_native_database_is_removed_is_reported_not_repaired()
     {
      let directory = tempfile::tempdir().unwrap();
      let paths = select_native(directory.path()).await;

      std::fs::remove_file(&paths.native_database).unwrap();
      // The marker removal (and everything after it) never ran.
      assert!(paths.marker.is_file());

      assert_eq!(
        inspect_authority(&observe_authority(&paths, schema_version())),
        AuthorityState::Inconsistent {
          reason: AuthorityInconsistency::MarkerWithoutNativeDatabase,
          recovery: AuthorityRecovery::StopAndReport,
        },
        "a reset interrupted between removing the native database and its marker must stop \
         and report, never guess or resurrect a selection"
      );
    }

    #[tokio::test]
    async fn reset_leaves_a_retired_sqlite_copy_in_place() {
      let directory = tempfile::tempdir().unwrap();
      let paths = select_native(directory.path()).await;
      let retired = directory.path().join("hv-database.db.retired");
      let retired_wal = directory.path().join("hv-database.db.retired-wal");
      std::fs::write(&retired, b"pre-conversion history").unwrap();
      std::fs::write(&retired_wal, b"pre-conversion wal").unwrap();

      discard_native_authority_files(&paths, directory.path()).unwrap();
      std::fs::remove_file(&paths.source_database).unwrap();

      assert_eq!(std::fs::read(&retired).unwrap(), b"pre-conversion history");
      assert_eq!(std::fs::read(&retired_wal).unwrap(), b"pre-conversion wal");
      // Leaving it in place must not make the reset profile look anything
      // but clean: `.retired` is never read by `observe_authority`.
      assert_fresh(&paths);
    }

    /// A native file this process cannot remove must stop the whole reset,
    /// not just log and continue - see `discard_native_authority_files`'s
    /// own "stop at the first failure" documentation. A non-empty directory
    /// at the native database's path is the deterministic, platform-
    /// independent stand-in: `std::fs::remove_file` reliably refuses to
    /// remove a directory on every target this crate builds for, unlike a
    /// locked-file failure, whose exact semantics differ by OS (see the
    /// Windows-only test below for that case too).
    #[test]
    fn reset_stops_at_the_first_unremovable_artifact_leaving_the_marker_and_source_intact()
     {
      let directory = tempfile::tempdir().unwrap();
      let paths = paths(directory.path());
      std::fs::write(&paths.source_database, b"sqlite source").unwrap();
      std::fs::create_dir(&paths.native_database).unwrap();
      std::fs::write(paths.native_database.join("not-a-database"), b"x").unwrap();
      std::fs::write(&paths.marker, b"marker artifact").unwrap();

      let error = discard_native_authority_files(&paths, directory.path()).unwrap_err();
      assert!(
        error.to_string().contains("native database file"),
        "{error}"
      );

      // Nothing after the failed step ran: the marker and SQLite source are
      // untouched, so a caller that stops here - rather than continuing on
      // to delete the SQLite source - never leaves a half-discarded,
      // misleading state on disk.
      assert!(paths.marker.is_file());
      assert!(paths.source_database.is_file());
      assert!(paths.native_database.is_dir());
    }

    /// The same failure mode as above, forced the way it would actually
    /// happen on Windows - a file another handle still has open - rather
    /// than the platform-independent directory stand-in.
    #[cfg(windows)]
    #[test]
    fn reset_stops_when_the_native_database_file_is_locked_open_on_windows() {
      use std::os::windows::fs::OpenOptionsExt;

      let directory = tempfile::tempdir().unwrap();
      let paths = paths(directory.path());
      std::fs::write(&paths.source_database, b"sqlite source").unwrap();
      std::fs::write(&paths.native_database, b"duckdb artifact").unwrap();
      std::fs::write(&paths.marker, b"marker artifact").unwrap();

      // `share_mode` narrowed to read-only sharing (no `FILE_SHARE_DELETE`)
      // is what actually forces `remove_file` to fail on Windows; a plain
      // `File::open` shares delete access by default and would not
      // reproduce the failure this test exists to cover.
      const FILE_SHARE_READ: u32 = 0x0000_0001;
      let _lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&paths.native_database)
        .unwrap();

      let error = discard_native_authority_files(&paths, directory.path()).unwrap_err();
      assert!(
        error.to_string().contains("native database file"),
        "{error}"
      );
      assert!(paths.marker.is_file());
      assert!(paths.source_database.is_file());
    }
  }
}
