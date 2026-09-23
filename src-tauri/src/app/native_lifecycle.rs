//! The App's ownership of the native database conversion lifecycle:
//! startup authority inspection and the small state vocabulary #2136 will
//! render.
//!
//! Core owns the *facts* a conversion produces (`observe_authority`) and the
//! *decision* they imply (`inspect_authority`), kept deliberately small - see
//! [`hardviz_core::infrastructure::database::native_database::AuthorityState`].
//! This module is the App's single reader of that decision: every other
//! subsystem, including the conversion driver added alongside it, goes
//! through [`inspect_startup_authority`] or [`NativeLifecycleOwner`] rather
//! than calling `observe_authority`/`inspect_authority` a second time from a
//! different place.
//!
//! # The state vocabulary
//!
//! [`DatabaseLifecycleState`] is a lossless translation of Core's
//! `AuthorityState`, widened with the two things a *running* conversion adds
//! that a point-in-time disk inspection cannot express: progress
//! ([`ConversionProgress`]) and a driver failure or cancellation
//! ([`LifecycleIssue`]). It stays small enough to enumerate on purpose - the
//! same reason Core's own authority vocabulary stays at two states - so
//! #2136 can render every value without guessing what an unlisted one might
//! mean.

use std::sync::Mutex;

use hardviz_core::infrastructure::database::native_database::{
  AuthorityInconsistency, AuthorityPaths, AuthorityRecovery, AuthorityState,
  NativeDatabase, inspect_authority, observe_authority, repair_authority_marker,
};

use crate::{log_error, log_info};

/// One step of a running conversion. Reported inside
/// [`DatabaseLifecycleState::Converting`] while the driver is active, and
/// named again by [`LifecycleIssue`] if that step is where it failed or was
/// cancelled.
///
/// No caller constructs one yet: the driver that reports progress through
/// this vocabulary is the next stacked change (#2135). Defining the
/// vocabulary here, ahead of the driver that produces values in it, is what
/// lets #2136 depend on a stable type rather than on the driver's internal
/// shape; the lint is suppressed for the same reason it is on
/// `infrastructure::database::native_schema`.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversionProgress {
  Preflight,
  BuildingCandidate,
  Finalizing,
  PausingProducers,
  Reconciling,
  Selecting,
  ResumingProducers,
}

/// Why the lifecycle owner is not making progress on its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleIssue {
  /// A disagreement `inspect_authority` refused to guess at, named with
  /// Core's own reason rather than re-described here. Every case but one is
  /// reported this way; the one repairable case
  /// (`AuthorityInconsistency::SelectedWithoutMarker`) is closed
  /// automatically by [`inspect_startup_authority`] and never reaches this
  /// variant unless the repair itself failed.
  Authority(AuthorityInconsistency),
  /// `inspect_authority` reported `NativeSelected` - the files agree and
  /// name the native database as authoritative - but opening it failed (the
  /// spill directory, the read/write handle, or an owner thread). The files
  /// are not in question here, unlike `Authority`; the runtime open itself
  /// is. Never silently falls back to SQLite: once selection is durable,
  /// ADR 0022 rejects running on both, so this is reported rather than
  /// treated as "still on SQLite".
  NativeOpenFailed { message: String },
  /// Creating the native database for an otherwise empty profile failed.
  FreshCreationFailed { message: String },
  /// The running conversion failed at a named step. Produced by the
  /// conversion driver (#2135, stacked on this change).
  #[allow(dead_code)]
  ConversionFailed {
    step: ConversionProgress,
    message: String,
  },
  /// The conversion was cancelled at a named step. SQLite stayed
  /// authoritative; a later attempt starts fresh or resumes, depending on
  /// what the step left behind. Produced by the conversion driver.
  #[allow(dead_code)]
  ConversionCancelled { step: ConversionProgress },
}

/// The small, enumerable set of states the native database lifecycle owner
/// can be in. See the module documentation for why this exists and what it
/// is a translation of.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatabaseLifecycleState {
  /// SQLite is authoritative and no conversion has produced anything yet.
  /// Startup may replace this with native authority when the profile is empty
  /// and the fresh-install path creates the database directly.
  SqliteAuthoritative,
  /// A previous conversion left recoverable state and SQLite is still
  /// authoritative. `resumable` says whether a complete finalized file
  /// exists to resume from reconciliation, or whether the copy restarts.
  ConversionRecoverable { resumable: bool },
  /// The conversion driver is running. Produced by the conversion driver.
  #[allow(dead_code)]
  Converting(ConversionProgress),
  /// The native database is authoritative and open.
  NativeAuthoritative,
  /// The lifecycle owner stopped rather than guess. Never produced by
  /// silently falling back to SQLite; see the module documentation.
  ActionRequired(LifecycleIssue),
}

/// Inspect on-disk authority state and apply the one allowed repair.
///
/// This is the App's single entry point onto Core's authority facts: nothing
/// else in the App calls `observe_authority` or `inspect_authority`
/// directly, so every startup decision comes from this function's returned
/// vocabulary rather than a second reading of the same files.
///
/// Never opens the native database, never touches the SQLite source, and
/// never creates anything - the one write this function can perform is
/// [`repair_authority_marker`], which only rewrites a marker file from a
/// native database's own already-committed metadata. Callers still own
/// deciding what to do with the returned state, including whether to open
/// the native database when it reports [`DatabaseLifecycleState::NativeAuthoritative`].
pub fn inspect_startup_authority(
  paths: &AuthorityPaths,
  expected_schema_version: u32,
) -> DatabaseLifecycleState {
  let decision = inspect_authority(&observe_authority(paths, expected_schema_version));
  match decision {
    AuthorityState::Inconsistent {
      reason,
      recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
    } => match repair_authority_marker(paths) {
      Ok(_marker) => {
        log_info!(
          "repaired the native selection marker from the database's own committed metadata",
          "app::native_lifecycle::inspect_startup_authority",
          None::<&str>
        );
        // Re-observe rather than assume the repair closed the gap: the
        // repair only ever rewrites the marker, and re-reading both files is
        // what proves it, not the repair call's own success.
        translate(inspect_authority(&observe_authority(
          paths,
          expected_schema_version,
        )))
      }
      Err(error) => {
        log_error!(
          "failed to repair the native selection marker; a committed selection is not \
           reflected on disk",
          "app::native_lifecycle::inspect_startup_authority",
          Some(error.to_string())
        );
        DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(reason))
      }
    },
    other => translate(other),
  }
}

fn translate(state: AuthorityState) -> DatabaseLifecycleState {
  match state {
    AuthorityState::SqliteAuthoritative => DatabaseLifecycleState::SqliteAuthoritative,
    AuthorityState::FinalizedUnselected => {
      DatabaseLifecycleState::ConversionRecoverable { resumable: true }
    }
    AuthorityState::ConversionInProgress { resumable } => {
      DatabaseLifecycleState::ConversionRecoverable { resumable }
    }
    AuthorityState::NativeSelected => DatabaseLifecycleState::NativeAuthoritative,
    AuthorityState::Inconsistent { reason, .. } => {
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(reason))
    }
  }
}

/// True when none of the three on-disk facts the authority decision reads
/// exist yet: no SQLite source, no native database, and no selection
/// marker.
///
/// [`inspect_startup_authority`] reports [`DatabaseLifecycleState::SqliteAuthoritative`]
/// for this same, empty case, because nothing on disk says otherwise *yet*.
/// That peek is correct about the disk, but a fresh install goes on to
/// create and select a native database directly, moments later, in the
/// same startup (#2203) - so a caller that needs to know which backend the
/// database will actually end up on, before that creation runs, must check
/// this instead of trusting the peek's answer alone. Today the one such
/// caller is the Hardware Archive Retention Period default (#2136): the
/// peek used to be trusted on its own, which defaulted a never-saved
/// retention value to the SQLite-era 30 days on every fresh install.
pub fn is_fresh_profile(paths: &AuthorityPaths) -> bool {
  !paths.source_database.is_file()
    && !paths.native_database.is_file()
    && !paths.marker.is_file()
}

/// The Hardware Archive Retention Period default that applies to a
/// never-saved value, given only the on-disk authority facts (#2136).
///
/// A fresh profile ([`is_fresh_profile`]) always resolves to the native
/// default, even though [`inspect_startup_authority`] itself would still
/// report [`DatabaseLifecycleState::SqliteAuthoritative`] for it - see that
/// function's documentation for why. An existing SQLite source with no
/// native artifacts yet keeps the SQLite-era default, since startup does
/// not touch it.
pub fn default_hardware_archive_retention_days(
  paths: &AuthorityPaths,
  expected_schema_version: u32,
) -> u32 {
  use hardviz_core::settings::HardwareArchiveSettings;

  if is_fresh_profile(paths) {
    return HardwareArchiveSettings::NATIVE_DEFAULT_RETENTION_DAYS;
  }
  match inspect_startup_authority(paths, expected_schema_version) {
    DatabaseLifecycleState::NativeAuthoritative => {
      HardwareArchiveSettings::NATIVE_DEFAULT_RETENTION_DAYS
    }
    _ => HardwareArchiveSettings::SQLITE_DEFAULT_RETENTION_DAYS,
  }
}

/// Whether `hv-database.db` is still the live source this state allows
/// touching - creating, migrating, or writing through an on-demand command
/// outside the paused/resumed background producers.
///
/// True only for [`DatabaseLifecycleState::SqliteAuthoritative`] and
/// [`DatabaseLifecycleState::ConversionRecoverable`]. Every other state
/// means either a conversion is actively quiescing producers to capture a
/// consistent snapshot ([`DatabaseLifecycleState::Converting`]), or native
/// authority is already selected and the source may already have been
/// renamed away by [`crate::app::native_maintenance::retire_sqlite_source`]
/// ([`DatabaseLifecycleState::NativeAuthoritative`]), or startup refused to
/// guess at a disagreement ([`DatabaseLifecycleState::ActionRequired`]).
/// Touching SQLite in any of those states risks exactly the bug this
/// function was added to close: recreating a retired source out from under
/// its own retirement, or silently dropping a write reconciliation already
/// stopped looking for.
pub fn sqlite_source_is_authoritative(state: &DatabaseLifecycleState) -> bool {
  matches!(
    state,
    DatabaseLifecycleState::SqliteAuthoritative
      | DatabaseLifecycleState::ConversionRecoverable { .. }
  )
}

/// Whether an on-demand write issued *through dispatch* (#2134) is safe to
/// answer right now, for a producer dispatch does not already pause/drain
/// around reconciliation (see `native_conversion::pause_and_drain_producers`
/// and its one known gap, `commands::hardware::refresh_storage_devices`).
///
/// Broader than [`sqlite_source_is_authoritative`]: once dispatch is
/// actually routing consumers, a write is fine in
/// [`DatabaseLifecycleState::NativeAuthoritative`] too - dispatch answers it
/// from the native database, not a possibly-retired SQLite file - so only
/// [`DatabaseLifecycleState::Converting`] (reconciliation is capturing the
/// snapshot a write outside the paused producers could otherwise race) and
/// [`DatabaseLifecycleState::ActionRequired`] (dispatch itself refuses,
/// `DispatchError::NativeUnavailable`) refuse here.
pub fn database_writable(state: &DatabaseLifecycleState) -> bool {
  !matches!(
    state,
    DatabaseLifecycleState::Converting(_) | DatabaseLifecycleState::ActionRequired(_)
  )
}

/// The App's single owner of native database lifecycle state and the
/// selected database instance. Managed as Tauri state so both the startup
/// path and the (later) conversion driver read and write through one place.
pub struct NativeLifecycleOwner {
  state: Mutex<DatabaseLifecycleState>,
  selected: Mutex<Option<NativeDatabase>>,
}

impl Default for NativeLifecycleOwner {
  fn default() -> Self {
    Self::new()
  }
}

impl NativeLifecycleOwner {
  pub fn new() -> Self {
    Self {
      state: Mutex::new(DatabaseLifecycleState::SqliteAuthoritative),
      selected: Mutex::new(None),
    }
  }

  /// Not read anywhere yet outside this module's own tests: the #2136
  /// command that exposes it to the frontend, and the conversion driver that
  /// needs to know the current state before transitioning out of it, both
  /// land in stacked changes.
  #[allow(dead_code)]
  pub fn state(&self) -> DatabaseLifecycleState {
    self.state.lock().unwrap().clone()
  }

  pub fn set_state(&self, state: DatabaseLifecycleState) {
    *self.state.lock().unwrap() = state;
  }

  /// Check `predicate` against the current state and, only if it holds,
  /// replace it with `next` - both under the same lock acquisition, so no
  /// concurrent [`Self::set_state`] (from a different in-flight attempt
  /// reaching a terminal state, for example) can land between the check and
  /// the write. Returns whether the transition happened.
  ///
  /// Exists for callers like
  /// `ConversionRuntime::begin_attempt_marking_converting` that must decide
  /// whether to start from the current state and, if so, mark it in one
  /// indivisible step - reading [`Self::state`] and calling
  /// [`Self::set_state`] separately would leave a window where a concurrent
  /// write could be silently overwritten.
  pub fn set_state_if(
    &self,
    predicate: impl FnOnce(&DatabaseLifecycleState) -> bool,
    next: DatabaseLifecycleState,
  ) -> bool {
    let mut guard = self.state.lock().unwrap();
    if predicate(&guard) {
      *guard = next;
      true
    } else {
      false
    }
  }

  // #2134 seam: the selected native database, once startup
  // (`inspect_startup_authority` resolving to
  // `DatabaseLifecycleState::NativeAuthoritative`) or a completed conversion
  // has opened one. #2134's dispatch boundary is the intended caller; today
  // nothing routes reads or writes through the value this returns, and the
  // existing SQLite-backed producers keep running unchanged until that
  // boundary lands.
  #[allow(dead_code)]
  pub fn selected_database(&self) -> Option<NativeDatabase> {
    self.selected.lock().unwrap().clone()
  }

  pub fn set_selected_database(&self, database: NativeDatabase) {
    self.selected.lock().unwrap().replace(database);
  }

  /// Take (and clear) whatever this owner currently holds, so a caller can
  /// hand it over - closing it - before something else (#2134's dispatch
  /// boundary) opens its own instance on the same file. See
  /// `native_conversion::adopt_selected_database_via_dispatch`, the `//
  /// #2134 seam:`.
  pub fn take_selected_database(&self) -> Option<NativeDatabase> {
    self.selected.lock().unwrap().take()
  }
}

#[cfg(test)]
mod tests {
  use hardviz_core::infrastructure::database::native_database::AUTHORITY_MARKER_FILE_NAME;

  use super::*;

  fn paths(directory: &std::path::Path) -> AuthorityPaths {
    AuthorityPaths {
      source_database: directory.join("hv-database.db"),
      native_database: directory.join("hv-database.duckdb"),
      marker: directory.join(AUTHORITY_MARKER_FILE_NAME),
    }
  }

  #[test]
  fn an_empty_profile_is_sqlite_authoritative_before_fresh_creation() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
      inspect_startup_authority(&paths(directory.path()), 1),
      DatabaseLifecycleState::SqliteAuthoritative
    );
  }

  /// #2136 regression: an empty profile directory must default the
  /// Hardware Archive Retention Period to the native value, because
  /// startup creates and selects a native database for it moments later
  /// (#2203) even though `inspect_startup_authority` alone still reports
  /// `SqliteAuthoritative` for the same, empty directory (see the test
  /// above).
  #[test]
  fn an_empty_profile_directory_is_fresh() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());

    assert!(is_fresh_profile(&paths));
    assert_eq!(
      default_hardware_archive_retention_days(&paths, 1),
      hardviz_core::settings::HardwareArchiveSettings::NATIVE_DEFAULT_RETENTION_DAYS
    );
  }

  /// A directory with only a pre-existing SQLite source (a plain SQLite
  /// installation from before native databases existed, or one just past
  /// its first migration) is not fresh, and keeps the SQLite-era default.
  #[test]
  fn a_directory_with_only_a_sqlite_source_is_not_fresh() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    std::fs::write(&paths.source_database, b"sqlite artifact").unwrap();

    assert!(!is_fresh_profile(&paths));
    assert_eq!(
      default_hardware_archive_retention_days(&paths, 1),
      hardviz_core::settings::HardwareArchiveSettings::SQLITE_DEFAULT_RETENTION_DAYS
    );
  }

  /// A profile that already selected a native database - the state a fresh
  /// install reaches moments after the empty-directory case above - is not
  /// "fresh" by this helper's own definition, but still resolves to the
  /// native default through `inspect_startup_authority` itself.
  #[tokio::test]
  async fn a_selected_native_profile_is_not_fresh_and_still_uses_the_native_default() {
    use hardviz_core::infrastructure::database::native_database::create_empty_native_database;

    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    create_empty_native_database(
      paths.clone(),
      crate::infrastructure::database::native_schema::get_native_schema(),
    )
    .await
    .unwrap();

    assert!(!is_fresh_profile(&paths));
    assert_eq!(
      default_hardware_archive_retention_days(
        &paths,
        crate::infrastructure::database::native_schema::NATIVE_SCHEMA_VERSION
      ),
      hardviz_core::settings::HardwareArchiveSettings::NATIVE_DEFAULT_RETENTION_DAYS
    );
  }

  #[test]
  fn sqlite_is_authoritative_only_while_still_the_live_source() {
    assert!(sqlite_source_is_authoritative(
      &DatabaseLifecycleState::SqliteAuthoritative
    ));
    assert!(sqlite_source_is_authoritative(
      &DatabaseLifecycleState::ConversionRecoverable { resumable: true }
    ));
    assert!(sqlite_source_is_authoritative(
      &DatabaseLifecycleState::ConversionRecoverable { resumable: false }
    ));

    assert!(!sqlite_source_is_authoritative(
      &DatabaseLifecycleState::NativeAuthoritative
    ));
    assert!(!sqlite_source_is_authoritative(
      &DatabaseLifecycleState::Converting(ConversionProgress::Reconciling)
    ));
    assert!(!sqlite_source_is_authoritative(
      &DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(
        AuthorityInconsistency::SourceDatabaseMissing
      ))
    ));
  }

  #[test]
  fn database_is_writable_through_dispatch_except_mid_conversion_or_blocked() {
    for state in [
      DatabaseLifecycleState::SqliteAuthoritative,
      DatabaseLifecycleState::ConversionRecoverable { resumable: true },
      DatabaseLifecycleState::ConversionRecoverable { resumable: false },
      // Unlike `sqlite_source_is_authoritative`, writable: dispatch answers
      // it from the native database.
      DatabaseLifecycleState::NativeAuthoritative,
    ] {
      assert!(database_writable(&state), "{state:?}");
    }

    for state in [
      DatabaseLifecycleState::Converting(ConversionProgress::Reconciling),
      DatabaseLifecycleState::Converting(ConversionProgress::PausingProducers),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(
        AuthorityInconsistency::SourceDatabaseMissing,
      )),
    ] {
      assert!(!database_writable(&state), "{state:?}");
    }
  }

  #[test]
  fn native_lifecycle_owner_starts_sqlite_authoritative_with_no_database() {
    let owner = NativeLifecycleOwner::new();
    assert_eq!(owner.state(), DatabaseLifecycleState::SqliteAuthoritative);
    assert!(owner.selected_database().is_none());
  }

  #[test]
  fn native_lifecycle_owner_round_trips_state() {
    let owner = NativeLifecycleOwner::new();
    owner.set_state(DatabaseLifecycleState::NativeAuthoritative);
    assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
  }

  /// A real, production-built "selected, marker missing" database: SQLite
  /// through App's own migrations, converted through the same candidate,
  /// finalize, reconcile and select calls the driver uses, with the marker
  /// deleted afterward to simulate the one interruption window
  /// `select_native_database`'s own documentation names. Nothing here
  /// hand-writes a native file.
  async fn selected_database_with_marker_removed(
    directory: &std::path::Path,
  ) -> AuthorityPaths {
    use hardviz_core::infrastructure::database::candidate_database::build_candidate_database;
    use hardviz_core::infrastructure::database::migrate;
    use hardviz_core::infrastructure::database::native_database::{
      finalize_candidate_database, reconcile_native_database, select_native_database,
    };
    use sqlx::ConnectOptions;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    let paths = paths(directory);
    let candidate = directory.join("candidate.duckdb");

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

    // Simulate the one interruption window: the database committed
    // `selected`, but the marker write never landed.
    std::fs::remove_file(&paths.marker).unwrap();
    paths
  }

  #[tokio::test]
  async fn a_committed_selection_without_its_marker_is_repaired_rather_than_reported() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_database_with_marker_removed(directory.path()).await;

    let state = inspect_startup_authority(
      &paths,
      crate::infrastructure::database::native_schema::NATIVE_SCHEMA_VERSION,
    );

    assert_eq!(state, DatabaseLifecycleState::NativeAuthoritative);
    assert!(
      paths.marker.is_file(),
      "the repair must have rewritten the marker"
    );
  }

  #[test]
  fn set_state_if_writes_only_when_the_predicate_holds_for_the_current_state() {
    let owner = NativeLifecycleOwner::new();

    let refused = owner.set_state_if(
      |state| matches!(state, DatabaseLifecycleState::NativeAuthoritative),
      DatabaseLifecycleState::Converting(ConversionProgress::Preflight),
    );
    assert!(!refused);
    assert_eq!(owner.state(), DatabaseLifecycleState::SqliteAuthoritative);

    let applied = owner.set_state_if(
      |state| matches!(state, DatabaseLifecycleState::SqliteAuthoritative),
      DatabaseLifecycleState::Converting(ConversionProgress::Preflight),
    );
    assert!(applied);
    assert_eq!(
      owner.state(),
      DatabaseLifecycleState::Converting(ConversionProgress::Preflight)
    );
  }

  #[test]
  fn set_state_if_evaluates_the_predicate_against_the_state_at_call_time_not_a_cached_read()
   {
    let owner = NativeLifecycleOwner::new();
    // A state change between an earlier `state()` read and this call must
    // still be what the predicate sees - this is the whole point of
    // `set_state_if` over a separate read-then-write.
    owner.set_state(DatabaseLifecycleState::NativeAuthoritative);

    let applied = owner.set_state_if(
      |state| matches!(state, DatabaseLifecycleState::SqliteAuthoritative),
      DatabaseLifecycleState::Converting(ConversionProgress::Preflight),
    );

    assert!(!applied);
    assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
  }
}
