//! The App-owned conversion driver: preflight, candidate capture,
//! finalization, quiescing every database producer, reconciliation and
//! durable selection.
//!
//! Core owns each step ([`build_candidate_database`],
//! [`finalize_candidate_database`], [`reconcile_native_database`],
//! [`select_native_database`]) and the invariants documented on them; this
//! module is the one place in the App that calls them in order, on the
//! paths [`crate::infrastructure::database::native_paths`] resolves, and
//! that pauses and resumes the App's own database producers around the step
//! that requires them quiesced.
//!
//! # Cancellation
//!
//! [`build_candidate_database`], [`finalize_candidate_database`] and
//! [`reconcile_native_database`] accept no cancellation token, and their own
//! documentation warns that dropping the awaited future does not stop the
//! blocking work - it keeps running and keeps writing to the path it was
//! given. Interrupting one mid-call would therefore not produce a clean
//! "nothing happened" state; it would race the driver's own cleanup against
//! a write still in flight. [`ConversionCancellation`] is checked only
//! *between* steps, immediately before starting the next one, never inside
//! an awaited Core call. Every step's own persisted result (a candidate
//! file, a finalized file, a reconciled and verified file) is exactly the
//! state a restart already knows how to resume or discard, so stopping
//! between steps needs no extra bookkeeping here.
//!
//! # What is paused
//!
//! Only the step between finalization and selection needs every SQLite
//! writer stopped: [`reconcile_native_database`] captures a fresh candidate
//! from the *live* source, and selecting a database that a writer changed
//! after that capture would silently drop whatever was written in between
//! (see the proof type's own documentation). Preflight, candidate capture
//! and finalization all run against an already-immutable snapshot or a
//! separate file, so nothing needs to pause for them.

// No caller drives this yet: #2136 adds the explicit user-triggered flow
// that invokes `run_conversion`, the same way `native_schema`'s definition
// waits for the finalizer that reads it. The driver is fully exercised by
// this module's own integration tests in the meantime - see the module for
// why testing it does not, by itself, satisfy the plain (non-test) build's
// dead-code analysis: `app` is a private module tree, so nothing here is
// externally reachable either.
#![allow(dead_code)]

use std::path::PathBuf;

use hardviz_core::infrastructure::database::candidate_database::build_candidate_database;
use hardviz_core::infrastructure::database::native_database::{
  NativeDatabaseError, finalize_candidate_database, plan_conversion_space,
  reconcile_native_database, select_native_database,
};
use hardviz_core::persistence::{
  ArchiveController, CoolingRollupController, StorageHealthController,
};

use crate::app::native_lifecycle::{
  ConversionProgress, DatabaseLifecycleState, LifecycleIssue, NativeLifecycleOwner,
  inspect_startup_authority,
};
use crate::infrastructure::database::{migration, native_schema};
use crate::workers::WorkersState;
use crate::{log_error, log_info};

/// The debris prefix `inspect_authority`'s `work_directory_present` already
/// recognizes; any suffix works as long as it starts with the shared root.
const DRIVER_WORK_PREFIX: &str = ".hardwarevisualizer-duckdb-driver-";

/// A one-shot, driver-wide cancellation flag, checked between steps.
///
/// Unrelated to `hardviz_core`'s `NativeCancellation`, which cancels one
/// request already in flight against an open [`NativeDatabase`][nd] — this
/// flag never reaches into a Core call at all; see the module documentation
/// for why.
///
/// [nd]: hardviz_core::infrastructure::database::native_database::NativeDatabase
#[derive(Clone, Default)]
pub struct ConversionCancellation(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl ConversionCancellation {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn cancel(&self) {
    self.0.store(true, std::sync::atomic::Ordering::Release);
  }

  pub fn is_cancelled(&self) -> bool {
    self.0.load(std::sync::atomic::Ordering::Acquire)
  }
}

/// What starting each paused producer back up needs, gathered once by the
/// caller that originally started them.
///
/// Reconstructing a controller needs the same arguments `lib.rs`'s startup
/// used (the `EventBus`, the ambient sensor registry, the Storage Health
/// identity key and guidance sink), which only that caller still has, so the
/// driver takes them back as closures instead of trying to cache or
/// rediscover them. `hw_archive` and `storage_health` are `None` when the
/// corresponding setting was off at startup and so nothing was running to
/// pause in the first place.
pub struct ProducerResumers {
  pub hw_archive: Option<Box<dyn FnOnce() -> ArchiveController + Send>>,
  pub cooling_rollup: Box<dyn FnOnce() -> CoolingRollupController + Send>,
  pub storage_health: Option<Box<dyn FnOnce() -> StorageHealthController + Send>>,
}

/// Stop every running database producer held in `workers` and wait for each
/// to finish, including a still-running startup retention pass. Producers
/// are gone (the `Mutex`es hold `None`) until [`resume_producers`] restarts
/// them; nothing else may read or write through them meanwhile.
pub async fn pause_and_drain_producers(workers: &WorkersState) {
  let hw_archive = workers.hw_archive.lock().unwrap().take();
  let cooling_rollup = workers.cooling_rollup.lock().unwrap().take();
  let storage_health = workers.storage_health.lock().unwrap().take();
  let scheduled_cleanup = workers.scheduled_cleanup.lock().unwrap().take();

  if let Some(hw_archive) = hw_archive {
    hw_archive.terminate().await;
  }
  if let Some(cooling_rollup) = cooling_rollup {
    cooling_rollup.terminate().await;
  }
  if let Some(storage_health) = storage_health {
    storage_health.terminate().await;
  }
  // Last: it only deletes rows the writers above already wrote, so it
  // cannot still be racing a write once they are gone.
  if let Some(scheduled_cleanup) = scheduled_cleanup {
    let _ = scheduled_cleanup.await;
  }
}

/// Restart every producer [`pause_and_drain_producers`] stopped, using the
/// construction closures gathered in `resumers`.
pub fn resume_producers(workers: &WorkersState, resumers: ProducerResumers) {
  if let Some(make) = resumers.hw_archive {
    workers.hw_archive.lock().unwrap().replace(make());
  }
  // `CoolingRollupController::setup` also returns a first-catch-up receiver
  // meant for startup's retention-cleanup ordering; a resumed rollup has no
  // such caller waiting on it, so `resumers.cooling_rollup`'s closure is
  // expected to discard that receiver itself (see `empty_resumers` in the
  // tests below for the pattern) and hand back just the controller.
  workers
    .cooling_rollup
    .lock()
    .unwrap()
    .replace((resumers.cooling_rollup)());
  if let Some(make) = resumers.storage_health {
    workers.storage_health.lock().unwrap().replace(make());
  }
}

/// What one [`run_conversion`] call ended with.
#[derive(Debug)]
pub enum ConversionOutcome {
  /// The native database was already selected before this call ran; nothing
  /// to do.
  AlreadySelected,
  /// Startup found an authority disagreement `inspect_authority` refused to
  /// guess at. The driver never started; `NativeLifecycleOwner::state`
  /// already reports why.
  ActionRequired,
  /// The conversion selected the native database.
  Selected { total_rows: u64 },
  /// Stopped before starting an irreversible step. SQLite is still
  /// authoritative; a later call resumes from whatever the completed steps
  /// left behind.
  Cancelled { step: ConversionProgress },
}

#[derive(Debug)]
pub enum ConversionError {
  Preflight(NativeDatabaseError),
  Candidate(hardviz_core::infrastructure::database::candidate_database::CandidateError),
  Finalize(NativeDatabaseError),
  Reconcile(NativeDatabaseError),
  Select(NativeDatabaseError),
}

impl std::fmt::Display for ConversionError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Preflight(error) => {
        write!(formatter, "preflight refused the conversion: {error}")
      }
      Self::Candidate(error) => {
        write!(formatter, "building the candidate failed: {error}")
      }
      Self::Finalize(error) => {
        write!(formatter, "finalizing the candidate failed: {error}")
      }
      Self::Reconcile(error) => {
        write!(formatter, "reconciling the native database failed: {error}")
      }
      Self::Select(error) => {
        write!(formatter, "selecting the native database failed: {error}")
      }
    }
  }
}

impl std::error::Error for ConversionError {}

/// Run the whole conversion once: preflight, candidate, finalize, pause
/// every database producer, reconcile, select, resume every producer.
///
/// `paths` and `workspace` are the caller's resolved
/// [`crate::infrastructure::database::native_paths::authority_paths`] and
/// [`crate::infrastructure::database::native_paths::database_directory`] -
/// taken as parameters, not resolved here, so this function never depends on
/// the real app-data directory and a test can point it at a temporary one.
///
/// Safe to call again after any outcome, including a failure: entry is
/// always decided by re-inspecting the on-disk authority state (the same
/// call startup uses), never by trusting what a previous call thinks it
/// left behind. An already-selected database or an unresolved authority
/// disagreement both return immediately without touching anything.
pub async fn run_conversion(
  paths: hardviz_core::infrastructure::database::native_database::AuthorityPaths,
  workspace: PathBuf,
  expected_schema_version: u32,
  owner: &NativeLifecycleOwner,
  workers: &WorkersState,
  resumers: ProducerResumers,
  cancellation: &ConversionCancellation,
) -> Result<ConversionOutcome, ConversionError> {
  // Checked before anything touches disk, and before `entry` is even
  // computed: `observe_authority`'s own documentation requires the native
  // file to have no live `NativeDatabase` owner, because reading its
  // metadata means opening it as a second DuckDB instance and DuckDB
  // refuses a second instance on a file another one holds. Once this or a
  // startup call has opened the selected database into the `// #2134 seam:`,
  // inspecting authority again would misreport it as unreadable instead of
  // reporting the true, healthy `NativeSelected` state.
  if owner.selected_database().is_some() {
    owner.set_state(DatabaseLifecycleState::NativeAuthoritative);
    return Ok(ConversionOutcome::AlreadySelected);
  }

  let entry = inspect_startup_authority(&paths, expected_schema_version);
  owner.set_state(entry.clone());
  let resume_from_reconciliation = match entry {
    DatabaseLifecycleState::NativeAuthoritative => {
      return Ok(ConversionOutcome::AlreadySelected);
    }
    DatabaseLifecycleState::ActionRequired(_) => {
      return Ok(ConversionOutcome::ActionRequired);
    }
    // `inspect_startup_authority` never returns this; it only ever reports a
    // point-in-time disk read, and nothing has started converting yet.
    DatabaseLifecycleState::Converting(_) => unreachable!(
      "inspect_startup_authority returned a running-conversion state at entry"
    ),
    DatabaseLifecycleState::SqliteAuthoritative => false,
    DatabaseLifecycleState::ConversionRecoverable { resumable } => resumable,
  };

  owner.set_state(DatabaseLifecycleState::Converting(
    ConversionProgress::Preflight,
  ));
  // Covers the whole conversion's peak, not just this call's first step:
  // reconciliation (below) captures a second candidate while the finalized
  // file it is updating is still on disk, which is the largest simultaneous
  // footprint either path reaches - see `plan_conversion_space`'s own
  // documentation. Measuring the source fresh here, immediately before any
  // copy, is also more accurate than trusting an earlier estimate on a
  // resumed run.
  plan_conversion_space(&paths.source_database, &workspace, None).map_err(|error| {
    fail(owner, ConversionProgress::Preflight, &error);
    ConversionError::Preflight(error)
  })?;

  if let Some(outcome) =
    check_cancelled(owner, cancellation, ConversionProgress::Preflight)
  {
    return Ok(outcome);
  }

  if !resume_from_reconciliation {
    owner.set_state(DatabaseLifecycleState::Converting(
      ConversionProgress::BuildingCandidate,
    ));
    let work = tempfile::Builder::new()
      .prefix(DRIVER_WORK_PREFIX)
      .tempdir_in(&workspace)
      .map_err(|error| {
        let error =
          hardviz_core::infrastructure::database::candidate_database::CandidateError::Worker {
            message: format!("failed to reserve a conversion work directory: {error}"),
          };
        fail(owner, ConversionProgress::BuildingCandidate, &error);
        ConversionError::Candidate(error)
      })?;
    let candidate_path: PathBuf = work.path().join("candidate.duckdb");
    build_candidate_database(
      &paths.source_database,
      &candidate_path,
      migration::get_migrations(),
    )
    .await
    .map_err(|error| {
      fail(owner, ConversionProgress::BuildingCandidate, &error);
      ConversionError::Candidate(error)
    })?;

    if let Some(outcome) =
      check_cancelled(owner, cancellation, ConversionProgress::BuildingCandidate)
    {
      return Ok(outcome);
    }

    owner.set_state(DatabaseLifecycleState::Converting(
      ConversionProgress::Finalizing,
    ));
    finalize_candidate_database(
      &candidate_path,
      &paths.native_database,
      native_schema::get_native_schema(),
    )
    .await
    .map_err(|error| {
      fail(owner, ConversionProgress::Finalizing, &error);
      ConversionError::Finalize(error)
    })?;
    // The candidate is only ever a finalization input; the finalized file is
    // the durable artifact from here on.
    drop(work);
  }

  if let Some(outcome) =
    check_cancelled(owner, cancellation, ConversionProgress::Finalizing)
  {
    return Ok(outcome);
  }

  owner.set_state(DatabaseLifecycleState::Converting(
    ConversionProgress::PausingProducers,
  ));
  if let Some(outcome) =
    check_cancelled(owner, cancellation, ConversionProgress::PausingProducers)
  {
    return Ok(outcome);
  }
  pause_and_drain_producers(workers).await;
  log_info!(
    "database producers paused for native conversion reconciliation",
    "app::native_conversion::run_conversion",
    None::<&str>
  );

  // From here every exit path - success, cancellation or failure - must
  // resume the producers before returning, so the pause/reconcile/select
  // sequence runs as one block whose result is handled only after resuming.
  let result =
    reconcile_and_select(owner, &paths, expected_schema_version, cancellation).await;
  resume_producers(workers, resumers);
  log_info!(
    "database producers resumed after native conversion reconciliation",
    "app::native_conversion::run_conversion",
    None::<&str>
  );

  result
}

async fn reconcile_and_select(
  owner: &NativeLifecycleOwner,
  paths: &hardviz_core::infrastructure::database::native_database::AuthorityPaths,
  expected_schema_version: u32,
  cancellation: &ConversionCancellation,
) -> Result<ConversionOutcome, ConversionError> {
  if let Some(outcome) =
    check_cancelled(owner, cancellation, ConversionProgress::PausingProducers)
  {
    return Ok(outcome);
  }

  owner.set_state(DatabaseLifecycleState::Converting(
    ConversionProgress::Reconciling,
  ));
  let (report, verified) = reconcile_native_database(
    &paths.source_database,
    &paths.native_database,
    migration::get_migrations(),
    native_schema::get_native_schema(),
  )
  .await
  .map_err(|error| {
    fail(owner, ConversionProgress::Reconciling, &error);
    ConversionError::Reconcile(error)
  })?;

  if let Some(outcome) =
    check_cancelled(owner, cancellation, ConversionProgress::Reconciling)
  {
    return Ok(outcome);
  }

  owner.set_state(DatabaseLifecycleState::Converting(
    ConversionProgress::Selecting,
  ));
  select_native_database(paths.clone(), verified)
    .await
    .map_err(|error| {
      fail(owner, ConversionProgress::Selecting, &error);
      ConversionError::Select(error)
    })?;

  let total_rows = report.total_rows;
  match open_selected_database(owner, paths, expected_schema_version).await {
    Ok(()) => {}
    Err(error) => {
      // Selection already committed durably; a failure to *open* it here
      // does not change authority, it only means the seam is empty until a
      // later successful startup or another call opens it. Reported, not
      // escalated: `ConversionOutcome::Selected` below is still accurate.
      log_error!(
        "native database selected but could not be opened for the #2134 seam",
        "app::native_conversion::reconcile_and_select",
        Some(error.to_string())
      );
    }
  }
  Ok(ConversionOutcome::Selected { total_rows })
}

/// Hand the freshly selected database to the `// #2134 seam:` the same way
/// startup does, so a caller that just converted does not have to restart
/// the process to see it.
async fn open_selected_database(
  owner: &NativeLifecycleOwner,
  paths: &hardviz_core::infrastructure::database::native_database::AuthorityPaths,
  expected_schema_version: u32,
) -> Result<(), NativeDatabaseError> {
  use hardviz_core::infrastructure::database::native_database::{
    NativeDatabase, NativeDatabaseOptions,
  };
  let database = NativeDatabase::open(
    &paths.native_database,
    NativeDatabaseOptions::new(expected_schema_version),
  )
  .await?;
  owner.set_selected_database(database);
  owner.set_state(DatabaseLifecycleState::NativeAuthoritative);
  Ok(())
}

/// Record a step failure both in the log and on the lifecycle owner, so a
/// caller reading `NativeLifecycleOwner::state` afterward sees *why*
/// without re-deriving it from the returned `ConversionError`.
fn fail(
  owner: &NativeLifecycleOwner,
  step: ConversionProgress,
  error: &impl std::fmt::Display,
) {
  let message = error.to_string();
  log_error!(
    "native database conversion step failed",
    "app::native_conversion",
    Some(message.clone())
  );
  owner.set_state(DatabaseLifecycleState::ActionRequired(
    LifecycleIssue::ConversionFailed { step, message },
  ));
}

fn check_cancelled(
  owner: &NativeLifecycleOwner,
  cancellation: &ConversionCancellation,
  step: ConversionProgress,
) -> Option<ConversionOutcome> {
  if !cancellation.is_cancelled() {
    return None;
  }
  log_info!(
    "native database conversion cancelled",
    "app::native_conversion::check_cancelled",
    None::<&str>
  );
  owner.set_state(DatabaseLifecycleState::ActionRequired(
    LifecycleIssue::ConversionCancelled { step },
  ));
  Some(ConversionOutcome::Cancelled { step })
}

#[cfg(test)]
mod tests {
  use hardviz_core::event_bus::EventBus;
  use hardviz_core::infrastructure::database::migrate;
  use hardviz_core::infrastructure::database::native_database::{
    AUTHORITY_MARKER_FILE_NAME, AuthorityPaths,
  };
  use sqlx::ConnectOptions;
  use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
  use tempfile::TempDir;

  use super::*;

  /// A real SQLite source, migrated through App's own ordered migrations -
  /// nothing here hand-writes a database, matching the fixture philosophy
  /// `core/tests/native_support` already established.
  struct Fixture {
    directory: TempDir,
  }

  impl Fixture {
    async fn new() -> Self {
      let directory = tempfile::tempdir().unwrap();
      let options = SqliteConnectOptions::new()
        .filename(directory.path().join("hv-database.db"))
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
      Self { directory }
    }

    fn paths(&self) -> AuthorityPaths {
      AuthorityPaths {
        source_database: self.directory.path().join("hv-database.db"),
        native_database: self.directory.path().join("hv-database.duckdb"),
        marker: self.directory.path().join(AUTHORITY_MARKER_FILE_NAME),
      }
    }

    fn workspace(&self) -> PathBuf {
      self.directory.path().to_path_buf()
    }
  }

  /// A `ProducerResumers` that starts nothing except the cooling rollup,
  /// which always restarts regardless of settings - see `resume_producers`.
  fn empty_resumers(runtime: tokio::runtime::Handle) -> ProducerResumers {
    ProducerResumers {
      hw_archive: None,
      cooling_rollup: Box::new(move || CoolingRollupController::setup(runtime).0),
      storage_health: None,
    }
  }

  #[tokio::test]
  async fn a_fresh_conversion_selects_the_native_database() {
    let fixture = Fixture::new().await;
    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();

    let outcome = run_conversion(
      fixture.paths(),
      fixture.workspace(),
      native_schema::NATIVE_SCHEMA_VERSION,
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
    )
    .await
    .unwrap();

    assert!(matches!(outcome, ConversionOutcome::Selected { .. }));
    assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
    assert!(
      owner.selected_database().is_some(),
      "the #2134 seam must hold the opened database after a successful conversion"
    );
    assert!(fixture.paths().marker.is_file());
    assert!(fixture.paths().native_database.is_file());
    // The cooling rollup was restarted after the pause; the archive/storage
    // health resumers were `None` because nothing was running to begin with.
    assert!(workers.cooling_rollup.lock().unwrap().is_some());
    assert!(workers.hw_archive.lock().unwrap().is_none());
  }

  #[tokio::test]
  async fn a_real_producer_is_drained_before_reconciliation_and_running_again_after() {
    let fixture = Fixture::new().await;
    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();
    let bus = EventBus::new();
    let runtime = tokio::runtime::Handle::current();

    workers
      .hw_archive
      .lock()
      .unwrap()
      .replace(ArchiveController::setup(&bus, runtime.clone()));

    let resumers = ProducerResumers {
      hw_archive: Some(Box::new({
        let bus = bus.clone();
        let runtime = runtime.clone();
        move || ArchiveController::setup(&bus, runtime)
      })),
      cooling_rollup: Box::new(move || CoolingRollupController::setup(runtime).0),
      storage_health: None,
    };

    let outcome = run_conversion(
      fixture.paths(),
      fixture.workspace(),
      native_schema::NATIVE_SCHEMA_VERSION,
      &owner,
      &workers,
      resumers,
      &ConversionCancellation::new(),
    )
    .await
    .unwrap();

    assert!(matches!(outcome, ConversionOutcome::Selected { .. }));
    // `pause_and_drain_producers` awaits `ArchiveController::terminate`,
    // which only returns once its task has actually stopped, so a new
    // controller being present here means the old one was fully joined
    // before reconciliation ran, not merely replaced.
    assert!(workers.hw_archive.lock().unwrap().is_some());
  }

  #[tokio::test]
  async fn a_resumable_conversion_skips_candidate_and_finalize() {
    let fixture = Fixture::new().await;
    // Matches what an earlier, interrupted driver run would have left
    // behind: a complete finalized file, no marker, no leftover work
    // directory - `AuthorityState::FinalizedUnselected`.
    let candidate = fixture.directory.path().join("candidate.duckdb");
    build_candidate_database(
      &fixture.paths().source_database,
      &candidate,
      migration::get_migrations(),
    )
    .await
    .unwrap();
    finalize_candidate_database(
      &candidate,
      &fixture.paths().native_database,
      native_schema::get_native_schema(),
    )
    .await
    .unwrap();
    std::fs::remove_file(&candidate).unwrap();
    assert_eq!(
      inspect_startup_authority(&fixture.paths(), native_schema::NATIVE_SCHEMA_VERSION),
      DatabaseLifecycleState::ConversionRecoverable { resumable: true }
    );

    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();
    let outcome = run_conversion(
      fixture.paths(),
      fixture.workspace(),
      native_schema::NATIVE_SCHEMA_VERSION,
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
    )
    .await
    .unwrap();

    assert!(matches!(outcome, ConversionOutcome::Selected { .. }));
  }

  #[tokio::test]
  async fn cancelling_before_any_copy_leaves_sqlite_authoritative_and_writes_nothing() {
    let fixture = Fixture::new().await;
    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();
    let cancellation = ConversionCancellation::new();
    cancellation.cancel();

    let outcome = run_conversion(
      fixture.paths(),
      fixture.workspace(),
      native_schema::NATIVE_SCHEMA_VERSION,
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &cancellation,
    )
    .await
    .unwrap();

    assert!(matches!(
      outcome,
      ConversionOutcome::Cancelled {
        step: ConversionProgress::Preflight
      }
    ));
    assert!(matches!(
      owner.state(),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::ConversionCancelled {
        step: ConversionProgress::Preflight
      })
    ));
    assert!(!fixture.paths().native_database.exists());

    // A restart resolves cleanly: nothing was produced, so authority is
    // exactly what it was before this call.
    assert_eq!(
      inspect_startup_authority(&fixture.paths(), native_schema::NATIVE_SCHEMA_VERSION),
      DatabaseLifecycleState::SqliteAuthoritative
    );
  }

  #[tokio::test]
  async fn a_missing_source_database_fails_at_preflight_without_side_effects() {
    let fixture = Fixture::new().await;
    std::fs::remove_file(fixture.paths().source_database).unwrap();

    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();

    let error = run_conversion(
      fixture.paths(),
      fixture.workspace(),
      native_schema::NATIVE_SCHEMA_VERSION,
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, ConversionError::Preflight(_)));
    assert!(matches!(
      owner.state(),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::ConversionFailed {
        step: ConversionProgress::Preflight,
        ..
      })
    ));
    assert!(!fixture.paths().native_database.exists());
  }

  #[tokio::test]
  async fn a_second_call_after_selection_is_a_no_op() {
    let fixture = Fixture::new().await;
    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();

    run_conversion(
      fixture.paths(),
      fixture.workspace(),
      native_schema::NATIVE_SCHEMA_VERSION,
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
    )
    .await
    .unwrap();

    let second = run_conversion(
      fixture.paths(),
      fixture.workspace(),
      native_schema::NATIVE_SCHEMA_VERSION,
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
    )
    .await
    .unwrap();

    assert!(matches!(second, ConversionOutcome::AlreadySelected));
  }
}
