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

use crate::{log_error, log_info};

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
/// is true today: when authority is already `NativeSelected` at boot, and
/// only *before* the database-backed producers (`ArchiveController`,
/// `CoolingRollupController`, `StorageHealthController`, scheduled
/// deletion) are started for that same boot - see the code comment where
/// they are started in `lib.rs`. Migrations are also skipped on that same
/// path (nothing else recreates `source_database`), and #2134's dispatch
/// boundary - adopted earlier, in `resolve_native_authority` - opens only
/// the native `.duckdb` file, never the SQLite source, so nothing holds
/// `source_database` open by the time this runs.
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
}
