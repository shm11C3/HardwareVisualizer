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

use std::path::{Path, PathBuf};

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

/// The shared root every conversion work directory's prefix starts with -
/// this driver's own [`DRIVER_WORK_PREFIX`], and Core's own
/// `.hardwarevisualizer-duckdb-finalize-`/`-reconcile-`/`-runtime-`
/// directories. The same root `inspect_authority`'s `work_directory_present`
/// check recognizes.
///
/// `pub(crate)` so `app::native_maintenance::discard_native_authority_files`
/// (Reset) matches the same debris without a second, drifting copy of the
/// literal prefix.
pub(crate) const WORK_DEBRIS_PREFIX: &str = ".hardwarevisualizer-duckdb-";

/// Best-effort removal of `.hardwarevisualizer-duckdb-*` directories already
/// in `workspace`, left behind by an attempt that crashed before producing
/// a usable finalized file.
///
/// A directory this cannot remove (still held open by another process, or a
/// permissions issue) is left in place and logged; it cannot block a fresh
/// attempt, which reserves its own differently-named directory, only make
/// this attempt's own preflight estimate more conservative than necessary.
/// Best-effort here is deliberate and unrelated to Reset's own removal,
/// which must not continue past a failure - see
/// `app::native_maintenance::discard_native_authority_files`, which matches
/// the same [`WORK_DEBRIS_PREFIX`] with its own fail-stop sweep rather than
/// calling this function.
fn discard_stale_conversion_work(workspace: &Path) {
  let Ok(entries) = std::fs::read_dir(workspace) else {
    return;
  };
  for entry in entries.filter_map(Result::ok) {
    if !entry
      .file_name()
      .to_string_lossy()
      .starts_with(WORK_DEBRIS_PREFIX)
    {
      continue;
    }
    if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
      continue;
    }
    let path = entry.path();
    match std::fs::remove_dir_all(&path) {
      Ok(()) => log_info!(
        "discarded stale conversion work directory before starting a new attempt",
        "app::native_conversion::discard_stale_conversion_work",
        Some(path.display().to_string())
      ),
      Err(error) => log_error!(
        "failed to discard a stale conversion work directory",
        "app::native_conversion::discard_stale_conversion_work",
        Some(format!("{}: {error}", path.display()))
      ),
    }
  }
}

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

/// Whether `state` is one `begin_attempt_marking_converting` starts a fresh
/// attempt from.
///
/// - [`DatabaseLifecycleState::SqliteAuthoritative`] and
///   [`DatabaseLifecycleState::ConversionRecoverable`]: the ordinary case -
///   no attempt has produced a durable selection yet.
/// - [`LifecycleIssue::ConversionFailed`] and
///   [`LifecycleIssue::ConversionCancelled`]: SQLite is still authoritative
///   (see [`fail`] and [`check_cancelled`]); the UI's "Try Again" restarts
///   from here, resuming from whatever completed steps left on disk.
/// - [`LifecycleIssue::NativeOpenFailed`]: the native database is already
///   durably selected, but this process could not open it or hand it to
///   dispatch (see [`open_selected_database`],
///   [`adopt_selected_database_via_dispatch`]). A retry re-enters
///   `run_conversion`, whose entry inspection reads the disk fresh, finds
///   `NativeAuthoritative` there, and retries the open - it does not redo
///   the conversion itself. Refusing this state instead would make "Try
///   Again" a permanent no-op for the one issue it exists to recover from.
///
/// Every other state refuses: [`DatabaseLifecycleState::NativeAuthoritative`]
/// and [`DatabaseLifecycleState::Converting`] because nothing needs
/// starting or one already is, and every other [`LifecycleIssue`]
/// (`Authority`, `FreshCreationFailed`) because those name a files-level
/// disagreement or a fresh-profile failure a plain restart of this same
/// flow cannot resolve.
fn is_startable_state(state: &DatabaseLifecycleState) -> bool {
  matches!(
    state,
    DatabaseLifecycleState::SqliteAuthoritative
      | DatabaseLifecycleState::ConversionRecoverable { .. }
      | DatabaseLifecycleState::ActionRequired(
        LifecycleIssue::ConversionFailed { .. }
          | LifecycleIssue::ConversionCancelled { .. }
          | LifecycleIssue::NativeOpenFailed { .. }
      )
  )
}

/// What #2136's explicit-user-intent conversion command needs that only
/// `lib::run`'s own setup closure otherwise holds: the `EventBus` every
/// database producer subscribes to, and a per-attempt cancellation flag.
///
/// Startup itself never pauses producers, so it never needed a way to
/// rebuild them; the explicit conversion flow added here does, because it
/// can run at any point in a session that is already producing rows. This
/// is a thin coordination point, not a second lifecycle owner:
/// [`NativeLifecycleOwner`] still owns the state vocabulary and the
/// selected database.
#[derive(Default)]
pub struct ConversionRuntime {
  bus: std::sync::Mutex<Option<hardviz_core::event_bus::EventBus>>,
  cancellation: std::sync::Mutex<Option<ConversionCancellation>>,
  in_progress: std::sync::atomic::AtomicBool,
}

impl ConversionRuntime {
  /// Record the process's one `EventBus`, once, right after `lib::run`
  /// creates it - the same bus `WindowAdapter`, `TrayAdapter` and every
  /// database producer subscribe to.
  pub fn set_bus(&self, bus: hardviz_core::event_bus::EventBus) {
    self.bus.lock().unwrap().replace(bus);
  }

  pub fn bus(&self) -> Option<hardviz_core::event_bus::EventBus> {
    self.bus.lock().unwrap().clone()
  }

  /// Claim the right to run one conversion attempt now, returning a fresh
  /// cancellation flag. `None` if an attempt already claimed it and has
  /// not called [`Self::end_attempt`] yet - callers use this to refuse a
  /// second concurrent `start_database_conversion` rather than run two
  /// drivers against the same files.
  pub fn begin_attempt(&self) -> Option<ConversionCancellation> {
    if self
      .in_progress
      .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
      return None;
    }
    let cancellation = ConversionCancellation::new();
    self
      .cancellation
      .lock()
      .unwrap()
      .replace(cancellation.clone());
    Some(cancellation)
  }

  /// [`Self::begin_attempt`], but only after [`Self::bus`] resolves -
  /// checked first and atomically with the claim, so a caller that also
  /// needs the bus (`commands::database_conversion::start_database_conversion`)
  /// can never claim the in-progress flag and then fail on a still-missing
  /// bus, which would strand every later call as a silent no-op
  /// ("another attempt already claimed it") until the process restarts.
  ///
  /// Returns the same `Err` [`Self::bus`]'s absence implies when the bus
  /// is not ready yet - no claim is taken - `Ok(None)` when another
  /// attempt already holds the claim (not an error), and `Ok(Some(..))`
  /// with a fresh cancellation flag and the resolved bus otherwise.
  pub fn begin_attempt_with_bus(
    &self,
  ) -> Result<Option<(ConversionCancellation, hardviz_core::event_bus::EventBus)>, String>
  {
    let bus = self
      .bus()
      .ok_or_else(|| "the database producer event bus is not ready yet".to_string())?;
    Ok(self.begin_attempt().map(|cancellation| (cancellation, bus)))
  }

  /// [`Self::begin_attempt_with_bus`], but also marks `owner` as
  /// `Converting(Preflight)` synchronously, in the same call, before the
  /// caller spawns the task that runs [`run_conversion`] - and only when
  /// `owner`'s *current* state, read and written atomically (see
  /// [`is_startable_state`] and [`NativeLifecycleOwner::set_state_if`]), is
  /// one a start actually begins from.
  ///
  /// `commands::database_conversion::start_database_conversion` used to
  /// leave `owner` at whatever it already reported until the spawned task's
  /// own point-in-time disk inspection ran - work that can take tens of ms
  /// (e.g. opening a finalized file to read its metadata on a
  /// resume/retry). A caller that polled `get_database_conversion` in that
  /// window read the pre-start state and could conclude nothing was
  /// running. Marking `owner` here, before this call returns to the
  /// command, closes that window: `run_conversion` itself is written not to
  /// regress this value back to a pre-start read - see its own entry-point
  /// documentation - so every path out of a successful claim already
  /// reports `Converting` for as long as it takes to become true.
  ///
  /// Only [`is_startable_state`] transitions to `Converting(Preflight)` -
  /// every other state (`NativeAuthoritative`, `Converting`, and every
  /// `ActionRequired` reason that function does not list) is left exactly
  /// as it is and this returns `Ok(None)` without claiming anything or
  /// spawning a task. This is a regression fix: the two Settings and
  /// startup-prompt UI surfaces keep independent hook state, so a stale
  /// "Convert Now" control can still call this after another surface (or a
  /// previous attempt) already reached `NativeAuthoritative`. Unconditionally
  /// marking `Converting` there would erase that terminal state, make
  /// `run_conversion`'s own already-selected guard miss (it only checks
  /// `owner.selected_database().is_some() || state == NativeAuthoritative`),
  /// and send the spawned task to re-inspect files while dispatch's own
  /// `NativeDatabase` still holds the file open - refused on Windows,
  /// unsafe to read concurrently elsewhere.
  ///
  /// The claim is taken first, then the state check-and-write happens under
  /// one lock via `set_state_if` - not a separate `owner.state()` read
  /// followed by a separate `owner.set_state(...)` write, which would leave
  /// a window for a concurrent completion (a different in-flight attempt
  /// reaching `NativeAuthoritative`, for example) to land in between and be
  /// silently overwritten. If the state check refuses after the claim
  /// already succeeded, the claim is released via [`Self::end_attempt`]
  /// before returning `Ok(None)`, so a doomed request never strands the
  /// in-progress flag for a real one behind it.
  pub fn begin_attempt_marking_converting(
    &self,
    owner: &NativeLifecycleOwner,
  ) -> Result<Option<(ConversionCancellation, hardviz_core::event_bus::EventBus)>, String>
  {
    let Some((cancellation, bus)) = self.begin_attempt_with_bus()? else {
      return Ok(None);
    };
    let marked = owner.set_state_if(
      is_startable_state,
      DatabaseLifecycleState::Converting(ConversionProgress::Preflight),
    );
    if !marked {
      self.end_attempt();
      return Ok(None);
    }
    Ok(Some((cancellation, bus)))
  }

  /// Release the claim [`Self::begin_attempt`] took, once `run_conversion`
  /// has returned - whatever the outcome.
  pub fn end_attempt(&self) {
    self.cancellation.lock().unwrap().take();
    self
      .in_progress
      .store(false, std::sync::atomic::Ordering::SeqCst);
  }

  /// Signal cancellation to whichever attempt is currently running.
  /// Returns `false` (and signals nothing) if no attempt is in flight.
  pub fn cancel_current(&self) -> bool {
    match self.cancellation.lock().unwrap().as_ref() {
      Some(cancellation) => {
        cancellation.cancel();
        true
      }
      None => false,
    }
  }
}

/// Build the real [`ProducerResumers`] the driver uses to restart database
/// producers it paused for reconciliation, mirroring the construction
/// `lib::run`'s own startup performs for the same three producers. Reads
/// `core_settings` fresh from [`crate::commands::settings::AppState`]
/// rather than trusting a value captured at startup, since the user may
/// have toggled Hardware Archive or Storage Health in this same session.
pub fn build_producer_resumers(
  app: &tauri::AppHandle,
  bus: hardviz_core::event_bus::EventBus,
  runtime: tokio::runtime::Handle,
) -> ProducerResumers {
  use tauri::Manager;

  let core_settings = app
    .state::<crate::commands::settings::AppState>()
    .core_settings
    .lock()
    .unwrap()
    .clone();

  let hw_archive: Option<Box<dyn FnOnce() -> ArchiveController + Send>> =
    if core_settings.hardware_archive.enabled {
      let app_handle = app.clone();
      let core_settings = core_settings.clone();
      let runtime = runtime.clone();
      Some(Box::new(move || {
        let environmental_sensors =
          crate::setup_environmental_sensors(&app_handle, &core_settings, &runtime);
        ArchiveController::setup_with_environmental_sensors(
          &bus,
          runtime,
          environmental_sensors,
        )
      }))
    } else {
      None
    };

  let cooling_rollup: Box<dyn FnOnce() -> CoolingRollupController + Send> = {
    let runtime = runtime.clone();
    Box::new(move || {
      // No first-catch-up caller waiting on a resumed rollup - see
      // `resume_producers`'s own documentation.
      CoolingRollupController::setup(runtime).0
    })
  };

  let storage_health: Option<Box<dyn FnOnce() -> StorageHealthController + Send>> =
    if core_settings.storage_health.enabled {
      match core_settings.storage_health_identity.hash_key_bytes() {
        Ok(identity_hash_key) => {
          let retention_days = core_settings.storage_health.retention_days;
          let guidance_state = std::sync::Arc::clone(
            &app.state::<std::sync::Arc<
              crate::services::external_component_guidance_service::ExternalComponentGuidanceState,
            >>(),
          );
          let runtime = runtime.clone();
          Some(Box::new(move || {
            let sink: hardviz_core::persistence::ExternalComponentGuidanceSink =
              std::sync::Arc::new(move |candidates| {
                guidance_state.record_candidates(candidates);
              });
            StorageHealthController::setup_with_guidance_sink(
              runtime,
              retention_days,
              identity_hash_key,
              Some(sink),
            )
          }))
        }
        Err(error) => {
          log_error!(
            "Storage Health producer was not resumed after conversion because the \
             identity key is invalid",
            "app::native_conversion::build_producer_resumers",
            Some(error)
          );
          None
        }
      }
    } else {
      None
    };

  ProducerResumers {
    hw_archive,
    cooling_rollup,
    storage_health,
  }
}

/// Which databases one conversion works on. App resolves all three
/// ([`crate::infrastructure::database::native_paths`] and
/// [`native_schema::NATIVE_SCHEMA_VERSION`]); they travel together so a test
/// can point the whole driver at a temporary directory.
pub struct ConversionTarget {
  pub paths: hardviz_core::infrastructure::database::native_database::AuthorityPaths,
  /// Where conversion work directories are created and the space preflight
  /// measures; the directory that holds both databases.
  pub workspace: PathBuf,
  pub expected_schema_version: u32,
}

/// Who answers database consumers once the selection is durable.
///
/// An explicit argument rather than a check of whether Core's dispatch
/// boundary happens to be initialised: `dispatch::init` is process-wide (a
/// `OnceLock`), so this module's own tests must be able to say they are not
/// using it instead of racing each other for that configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionHandoff {
  /// The App's real path. `dispatch::init` has already run at startup, and
  /// the selected database is handed to the dispatch boundary
  /// ([`adopt_selected_database_via_dispatch`]) while the producers are
  /// still paused, so nothing can write to the stale SQLite source between
  /// the final reconciliation and the boundary switching backends.
  ThroughDispatch,
  /// Keep the opened database on the [`NativeLifecycleOwner`] only and never
  /// touch the dispatch boundary.
  OwnerOnly,
}

/// What one [`run_conversion`] call ended with.
#[derive(Debug)]
pub enum ConversionOutcome {
  /// The native database was already selected before this call ran; nothing
  /// to do.
  AlreadySelected,
  /// The call stopped rather than guess, and `NativeLifecycleOwner::state`
  /// already reports why: either entry found an authority disagreement
  /// `inspect_authority` refused to guess at and the driver never started,
  /// or the selection became durable but could not be finished, opened or
  /// handed over, and the producers were left paused.
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
  target: ConversionTarget,
  owner: &NativeLifecycleOwner,
  workers: &WorkersState,
  resumers: ProducerResumers,
  cancellation: &ConversionCancellation,
  handoff: SelectionHandoff,
) -> Result<ConversionOutcome, ConversionError> {
  let ConversionTarget {
    paths,
    workspace,
    expected_schema_version,
  } = target;

  // Checked before anything touches disk, and before `entry` is even
  // computed: `observe_authority`'s own documentation requires the native
  // file to have no live `NativeDatabase` owner, because reading its
  // metadata means opening it as a second DuckDB instance and DuckDB
  // refuses a second instance on a file another one holds. Once this or a
  // startup call has opened the selected database - into the `// #2134
  // seam:`, or into the dispatch boundary, which leaves the owner holding
  // no handle of its own and only its `NativeAuthoritative` state to show
  // for it - inspecting authority again would misreport it as unreadable
  // instead of reporting the true, healthy `NativeSelected` state.
  if owner.selected_database().is_some()
    || matches!(owner.state(), DatabaseLifecycleState::NativeAuthoritative)
  {
    owner.set_state(DatabaseLifecycleState::NativeAuthoritative);
    return Ok(ConversionOutcome::AlreadySelected);
  }

  // Deliberately not `owner.set_state(entry.clone())` unconditionally here:
  // `commands::database_conversion::start_database_conversion` may already
  // have set `owner` to `Converting(Preflight)` synchronously, before this
  // task even ran, so a subsequent caller polling `get_database_conversion`
  // never reads a stale pre-start state - see that command's own
  // documentation. Writing `entry` back here unconditionally would regress
  // that optimistic state to whatever `entry` reports whenever it is
  // `SqliteAuthoritative` or `ConversionRecoverable`, reopening the same
  // invisible-conversion window a few lines below closes for good (this
  // function sets `Converting(Preflight)` itself either way). Only the
  // branches below that do not reach that line - `NativeAuthoritative` and
  // `ActionRequired`, both terminal for this call - still record `entry`
  // themselves.
  let entry = inspect_startup_authority(&paths, expected_schema_version);
  let resume_from_reconciliation = match entry {
    DatabaseLifecycleState::NativeAuthoritative => {
      owner.set_state(DatabaseLifecycleState::NativeAuthoritative);
      // Durably selected on disk, but the guard above already established
      // this process holds no open handle - either a fresh call after
      // another process's selection, or a retry after this same process's
      // own post-selection open failed (see `reconcile_and_select`).
      // Attempting the open here, rather than assuming there is nothing
      // left to do, is what makes that failure recoverable without a
      // process restart.
      return Ok(
        match open_selected_database(owner, &paths, expected_schema_version).await {
          Ok(()) => {
            let outcome =
              hand_off(owner, handoff, ConversionOutcome::AlreadySelected).await;
            // This is also the way back from a hand-over that failed earlier
            // and left the producers paused (see the end of this function),
            // and from a startup that refused to start them. The cooling
            // rollup restarts unconditionally, so its absence is what says
            // nothing is running; never start a second set beside a live one.
            let producers_stopped = workers.cooling_rollup.lock().unwrap().is_none();
            if matches!(outcome, ConversionOutcome::AlreadySelected) && producers_stopped
            {
              resume_producers(workers, resumers);
            }
            outcome
          }
          Err(error) => {
            fail_open(owner, &error);
            ConversionOutcome::ActionRequired
          }
        },
      );
    }
    DatabaseLifecycleState::ActionRequired(_) => {
      owner.set_state(entry);
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

  // Before measuring available space: any `.hardwarevisualizer-duckdb-*`
  // directory already in `workspace` is debris from an attempt that
  // crashed - `inspect_authority`'s own `work_directory_present` check is
  // what flagged this entry as `ConversionInProgress` in the first place.
  // Nothing will ever read it again (a resumable attempt's own valid
  // finalized file lives at `paths.native_database`, not in a work
  // directory), and left in place it can make the preflight measurement
  // below fail for lack of space it would otherwise have found.
  discard_stale_conversion_work(&workspace);

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
  //
  // `resume_from_reconciliation` is passed as `finalized_already_exists`:
  // when resuming, the finalized file is already on disk and this budget
  // must not also reserve room for creating a second one - the caller's
  // available-space measurement already counts the existing file as not
  // free.
  plan_conversion_space(
    &paths.source_database,
    &workspace,
    None,
    resume_from_reconciliation,
  )
  .map_err(|error| {
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

  // The pause/reconcile/select sequence runs as one block whose result is
  // handled only after the producers have been dealt with - see below for
  // the one outcome that leaves them stopped.
  //
  // The hand-over to the dispatch boundary belongs inside that block too. A
  // producer resumed while dispatch still routes to SQLite would write after
  // the final reconciliation into a source that is no longer authoritative,
  // and that row would be absent from the selected database for good.
  let result = match reconcile_and_select(
    owner,
    &paths,
    expected_schema_version,
    cancellation,
    handoff,
  )
  .await
  {
    Ok(outcome @ ConversionOutcome::Selected { .. }) => {
      Ok(hand_off(owner, handoff, outcome).await)
    }
    other => other,
  };

  // Producers resume only onto a backend that is both authoritative and
  // being served. Inside this block `ActionRequired` has one meaning: the
  // selection is already durable, but finishing it (see
  // `settle_failed_selection`), opening the selected database or handing it
  // to dispatch failed. SQLite is then a stale recovery copy, and
  // dispatch either still routes to it or refuses every consumer, so a
  // resumed producer would write rows the native database never gets, or
  // collect rows only to have them refused and logged. They stay stopped,
  // the same way a startup that ends in `ActionRequired` never starts them,
  // until a later call completes the hand-over (the entry path above).
  // Cancellation and a failure before selection leave SQLite authoritative,
  // so those resume exactly as before.
  if matches!(result, Ok(ConversionOutcome::ActionRequired)) {
    log_error!(
      "database producers left paused: the native database is selected but not served",
      "app::native_conversion::run_conversion",
      None::<&str>
    );
  } else {
    resume_producers(workers, resumers);
    log_info!(
      "database producers resumed after native conversion reconciliation",
      "app::native_conversion::run_conversion",
      None::<&str>
    );
  }

  result
}

async fn reconcile_and_select(
  owner: &NativeLifecycleOwner,
  paths: &hardviz_core::infrastructure::database::native_database::AuthorityPaths,
  expected_schema_version: u32,
  cancellation: &ConversionCancellation,
  handoff: SelectionHandoff,
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
  if let Err(error) = select_native_database(paths.clone(), verified).await
    && let Some(outcome) =
      settle_failed_selection(owner, paths, expected_schema_version, handoff, error)
        .await?
  {
    return Ok(outcome);
  }

  let total_rows = report.total_rows;
  // Selection already committed durably; a failure to *open* it here does
  // not change authority - the file is still the selected one - but the
  // owner must not claim `NativeAuthoritative` without holding it open (see
  // `open_selected_database`'s contract). Recorded as `ActionRequired`
  // rather than folded into a still-`Selected` outcome, because the retry
  // is the entry check at the top of `run_conversion`: a later call sees
  // `NativeAuthoritative` on disk, finds no open database in the owner, and
  // attempts the open again there - the same path a fresh process's startup
  // uses - rather than requiring a process restart to recover.
  match open_selected_database(owner, paths, expected_schema_version).await {
    Ok(()) => Ok(ConversionOutcome::Selected { total_rows }),
    Err(error) => {
      fail_open(owner, &error);
      Ok(ConversionOutcome::ActionRequired)
    }
  }
}

/// Decide what a failed [`select_native_database`] left on disk.
///
/// `select_native_database` commits `selected` into the native file before
/// it checkpoints, syncs and publishes the marker (Core's selection module
/// explains why that order, and not the reverse, is the repairable one), so
/// its error alone does not say whether SQLite is still authoritative. The
/// files are inspected again, through the same function startup uses:
///
/// - The files positively show no selection ([`failed_before_commit`]): the
///   failure came before the commit, and it is an ordinary, retryable
///   conversion failure.
/// - `NativeAuthoritative`: the commit landed, and
///   [`inspect_startup_authority`] closed the gap by rewriting the marker
///   from the committed metadata. Returns `Ok(None)` so the caller finishes
///   the selection normally. The producers are still paused and the file was
///   reconciled just now, so this is the one moment the repair cannot leave
///   a SQLite row behind.
/// - Anything else - the repair itself failed, or the native metadata cannot
///   be read right now (the same I/O or `.wal` problem that failed the
///   checkpoint can hide a committed `selected`): the commit may have
///   landed, so SQLite must not be written again. Returns `ActionRequired`,
///   which keeps the producers paused; under
///   [`SelectionHandoff::ThroughDispatch`] the boundary is also told to
///   refuse every consumer rather than keep routing on-demand writes to
///   SQLite. A later [`run_conversion`] retries from its entry path once the
///   cause is gone.
///
/// Treating every error as retryable resumed the producers on SQLite, and
/// the next startup repaired the marker and retired SQLite without the rows
/// they wrote in between (#2238).
async fn settle_failed_selection(
  owner: &NativeLifecycleOwner,
  paths: &hardviz_core::infrastructure::database::native_database::AuthorityPaths,
  expected_schema_version: u32,
  handoff: SelectionHandoff,
  error: NativeDatabaseError,
) -> Result<Option<ConversionOutcome>, ConversionError> {
  use hardviz_core::infrastructure::database::native_database::AuthorityInconsistency;

  let on_disk = inspect_startup_authority(paths, expected_schema_version);
  let native_file = std::fs::symlink_metadata(&paths.native_database);
  if failed_before_commit(&on_disk, &native_file) {
    fail(owner, ConversionProgress::Selecting, &error);
    return Err(ConversionError::Select(error));
  }
  log_error!(
    "selecting the native database failed and the selection may have committed",
    "app::native_conversion::settle_failed_selection",
    Some(format!("{error}; the files now read as {on_disk:?}"))
  );
  let issue = match on_disk {
    DatabaseLifecycleState::NativeAuthoritative => return Ok(None),
    DatabaseLifecycleState::ActionRequired(issue) => issue,
    _ => LifecycleIssue::Authority(AuthorityInconsistency::NativeMetadataUnreadable),
  };
  if handoff == SelectionHandoff::ThroughDispatch {
    // `refuse_consumers` leaves the boundary unavailable on every path;
    // `reobserve_authority` would not, because unreadable metadata inspects
    // as an interrupted conversion and it would answer from SQLite.
    if let Err(close_error) =
      hardviz_core::infrastructure::database::dispatch::refuse_consumers(format!(
        "the native selection could not be completed: {error}"
      ))
      .await
    {
      log_error!(
        "closing the dispatch boundary's native owner failed while refusing consumers",
        "app::native_conversion::settle_failed_selection",
        Some(close_error.to_string())
      );
    }
  }
  owner.set_state(DatabaseLifecycleState::ActionRequired(issue));
  Ok(Some(ConversionOutcome::ActionRequired))
}

/// Whether the re-inspection after a failed selection positively shows that
/// `selected` never committed. `native_file` is `symlink_metadata` of the
/// native database path.
///
/// Only two facts count as proof: native metadata read back as finalized and
/// unselected (`ConversionRecoverable { resumable: true }`), or no native file
/// at all. The observer derives "no native file" from `is_file`, which a
/// metadata error also makes false, so `SqliteAuthoritative` and
/// `ConversionRecoverable { resumable: false }` count only when the probe
/// reports `NotFound`. An inaccessible or unreadable file is uncertainty, not
/// absence. (The marker needs no probe: the observer already reports it absent
/// only on `NotFound`.)
fn failed_before_commit(
  on_disk: &DatabaseLifecycleState,
  native_file: &std::io::Result<std::fs::Metadata>,
) -> bool {
  let native_absent = matches!(
    native_file,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound
  );
  match on_disk {
    DatabaseLifecycleState::ConversionRecoverable { resumable: true } => true,
    DatabaseLifecycleState::SqliteAuthoritative
    | DatabaseLifecycleState::ConversionRecoverable { resumable: false } => native_absent,
    _ => false,
  }
}

/// Open the freshly selected database into `owner`'s own bookkeeping, the
/// same way startup does, so a caller that just converted does not have to
/// restart the process to see `NativeAuthoritative` reflected. This is
/// `owner`'s *own* copy for the driver's own use (retry/idempotency
/// checks - see `run_conversion`'s entry) and is deliberately not the one
/// #2134's dispatch boundary answers consumers from; see
/// [`adopt_selected_database_via_dispatch`] for that hand-over.
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

/// `// #2134 seam:` hand `owner`'s selected database over to Core's dispatch
/// boundary, so every consumer routed through
/// [`hardviz_core::infrastructure::database::dispatch`] answers from it -
/// not from the copy [`open_selected_database`] opened above for `owner`'s
/// own bookkeeping.
///
/// [`run_conversion`] calls this itself under
/// [`SelectionHandoff::ThroughDispatch`], while the producers are still
/// paused. Dispatch's `init`/`reobserve_authority` are process-wide (a
/// `OnceLock`-backed configuration - see their own documentation), so
/// `dispatch::init` must already have run with the same `paths` this driver
/// was given; in this App that is `lib.rs`'s startup
/// (`resolve_native_authority`). This module's own tests pass
/// [`SelectionHandoff::OwnerOnly`] instead and never reach this function, so
/// none of them can corrupt each other's configuration by racing for the
/// same `OnceLock`. See the App-level integration tests under
/// `src-tauri/tests/` for the proof this actually wires together end to end.
///
/// Only [`AuthorityState::NativeSelected`] counts as a successful hand-over.
/// `reobserve_authority` deliberately returns `Ok` for an inconsistent
/// marker, metadata or file state while leaving the boundary refusing every
/// consumer, so treating any `Ok` as success would report
/// `NativeAuthoritative` over a backend that answers nothing.
///
/// Respects the single-owner rule
/// [`hardviz_core::infrastructure::database::dispatch::reobserve_authority`]'s
/// own documentation requires: DuckDB refuses a second instance on a file
/// another one already holds, and `reobserve_authority` is the only thing
/// that ever opens dispatch's own instance - so `owner`'s copy is closed
/// ("handed over") first, never left open beside it.
pub async fn adopt_selected_database_via_dispatch(
  owner: &NativeLifecycleOwner,
) -> Result<(), NativeDatabaseError> {
  use hardviz_core::infrastructure::database::dispatch;
  use hardviz_core::infrastructure::database::native_database::AuthorityState;

  if let Some(database) = owner.take_selected_database() {
    database.close().await?;
  }
  match dispatch::reobserve_authority().await {
    Ok(AuthorityState::NativeSelected) => {
      owner.set_state(DatabaseLifecycleState::NativeAuthoritative);
      Ok(())
    }
    Ok(other) => {
      let error = NativeDatabaseError::Worker {
        message: format!(
          "the dispatch boundary observed {other:?} instead of the selected native \
           database and is refusing every consumer"
        ),
      };
      fail_open(owner, &error);
      Err(error)
    }
    Err(error) => {
      fail_open(owner, &error);
      Err(error)
    }
  }
}

/// Apply `handoff` to a call that ended with the native database selected
/// and open on `owner`. A failed hand-over is reported as
/// [`ConversionOutcome::ActionRequired`]: the selection itself is durable, so
/// it is not a conversion failure, and `owner`'s state already says why.
async fn hand_off(
  owner: &NativeLifecycleOwner,
  handoff: SelectionHandoff,
  selected: ConversionOutcome,
) -> ConversionOutcome {
  match handoff {
    SelectionHandoff::OwnerOnly => selected,
    SelectionHandoff::ThroughDispatch => {
      match adopt_selected_database_via_dispatch(owner).await {
        Ok(()) => selected,
        Err(_) => ConversionOutcome::ActionRequired,
      }
    }
  }
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

/// Record a failure to open an already, durably selected database - see
/// `open_selected_database`'s own callers for why this is distinct from
/// [`fail`]: the selection itself did not fail, so naming it a conversion
/// step failure would be wrong, and unlike `LifecycleIssue::Authority` the
/// files are not in question, only the runtime open.
fn fail_open(owner: &NativeLifecycleOwner, error: &impl std::fmt::Display) {
  let message = error.to_string();
  log_error!(
    "native database selected but could not be opened",
    "app::native_conversion",
    Some(message.clone())
  );
  owner.set_state(DatabaseLifecycleState::ActionRequired(
    LifecycleIssue::NativeOpenFailed { message },
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
mod conversion_runtime_tests {
  use super::ConversionRuntime;
  use crate::app::native_lifecycle::{
    ConversionProgress, DatabaseLifecycleState, LifecycleIssue, NativeLifecycleOwner,
  };

  #[test]
  fn a_second_attempt_is_refused_while_one_is_in_progress() {
    let runtime = ConversionRuntime::default();

    assert!(runtime.begin_attempt().is_some());
    assert!(
      runtime.begin_attempt().is_none(),
      "a second concurrent attempt must not get its own cancellation flag"
    );
  }

  #[test]
  fn ending_an_attempt_allows_a_new_one_to_start() {
    let runtime = ConversionRuntime::default();

    let first = runtime.begin_attempt().unwrap();
    runtime.end_attempt();

    let second = runtime.begin_attempt();
    assert!(second.is_some());
    assert!(
      !first.is_cancelled(),
      "ending an attempt must not cancel it retroactively"
    );
  }

  #[test]
  fn cancel_current_signals_the_in_flight_attempt() {
    let runtime = ConversionRuntime::default();
    let cancellation = runtime.begin_attempt().unwrap();

    assert!(runtime.cancel_current());
    assert!(cancellation.is_cancelled());
  }

  #[test]
  fn cancel_current_is_a_no_op_when_nothing_is_running() {
    let runtime = ConversionRuntime::default();
    assert!(!runtime.cancel_current());
  }

  #[test]
  fn the_bus_survives_after_being_set() {
    let runtime = ConversionRuntime::default();
    assert!(runtime.bus().is_none());

    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    assert!(runtime.bus().is_some());
  }

  /// Regression for the #2220 review finding: a missing bus must never
  /// strand the in-progress claim. Before `begin_attempt_with_bus`
  /// existed, the caller resolved the bus *after* `begin_attempt()`, so a
  /// missing bus left `in_progress` set with nothing to call
  /// `end_attempt()` - every later `start_database_conversion` would then
  /// find `begin_attempt` already claimed and silently do nothing, forever.
  #[test]
  fn a_missing_bus_never_strands_the_in_progress_claim() {
    let runtime = ConversionRuntime::default();

    match runtime.begin_attempt_with_bus() {
      Err(error) => assert!(error.contains("event bus")),
      Ok(_) => panic!("expected an error while the bus is not set"),
    }

    // If the claim had been taken before the bus check, this would
    // observe `Ok(None)` ("another attempt already claimed it") instead.
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    match runtime.begin_attempt_with_bus() {
      Ok(Some(_)) => {}
      other => panic!("expected a fresh claim once the bus is set, got {}", {
        match &other {
          Ok(None) => "Ok(None)",
          Err(_) => "Err(_)",
          Ok(Some(_)) => unreachable!(),
        }
      }),
    }
  }

  #[test]
  fn begin_attempt_with_bus_returns_none_when_another_attempt_is_in_progress() {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());

    assert!(runtime.begin_attempt_with_bus().unwrap().is_some());
    assert!(runtime.begin_attempt_with_bus().unwrap().is_none());
  }

  /// Regression for #2245: before
  /// `begin_attempt_marking_converting` existed,
  /// `commands::database_conversion::start_database_conversion` returned as
  /// soon as it spawned the driver task, and the first `Converting` write
  /// happened only inside that task, after its own point-in-time disk
  /// inspection ran - work that can take tens of ms (e.g. opening a
  /// finalized file to read its metadata on a resume/retry). A caller that
  /// polled `get_database_conversion` in that window read whatever
  /// pre-start state `owner` already had and could conclude nothing was
  /// running. A successful claim must leave `owner` reporting `Converting`
  /// synchronously, before this call returns - not later, once some task
  /// eventually gets scheduled.
  #[test]
  fn a_successful_claim_marks_the_owner_converting_synchronously() {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    let owner = NativeLifecycleOwner::new();

    assert_eq!(
      owner.state(),
      DatabaseLifecycleState::SqliteAuthoritative,
      "the fixture must start from the same pre-start state a real profile \
       does, or this test would not exercise the race at all"
    );

    assert!(
      runtime
        .begin_attempt_marking_converting(&owner)
        .unwrap()
        .is_some()
    );

    assert_eq!(
      owner.state(),
      DatabaseLifecycleState::Converting(ConversionProgress::Preflight),
      "the owner must already report Converting the instant a claim \
       succeeds, before any task spawned around it has had a chance to run"
    );
  }

  /// A refused claim (another attempt already in progress) must leave
  /// `owner` exactly as it found it - it is not this call's place to
  /// report anything when it is not the one that will drive the
  /// conversion forward.
  #[test]
  fn a_refused_claim_does_not_touch_the_owner() {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    let owner = NativeLifecycleOwner::new();

    assert!(
      runtime
        .begin_attempt_marking_converting(&owner)
        .unwrap()
        .is_some()
    );
    // A second, independent owner stands in for what a second concurrent
    // caller of the command would read: this call must not touch it.
    let second_owner = NativeLifecycleOwner::new();
    assert!(
      runtime
        .begin_attempt_marking_converting(&second_owner)
        .unwrap()
        .is_none()
    );
    assert_eq!(
      second_owner.state(),
      DatabaseLifecycleState::SqliteAuthoritative
    );
  }

  /// A missing bus must leave `owner` untouched too - the same "claim and
  /// mark must not be separate fallible steps" guarantee
  /// `begin_attempt_with_bus` documents, extended to the state write this
  /// method adds.
  #[test]
  fn a_missing_bus_does_not_mark_the_owner_converting() {
    let runtime = ConversionRuntime::default();
    let owner = NativeLifecycleOwner::new();

    assert!(runtime.begin_attempt_marking_converting(&owner).is_err());
    assert_eq!(owner.state(), DatabaseLifecycleState::SqliteAuthoritative);
  }

  /// Regression for a PR #2252 review finding: the Settings section and the
  /// startup prompt keep independent hook state, so a stale "Convert Now"
  /// control can still call `start_database_conversion` after a *different*
  /// call already reached `NativeAuthoritative` (e.g. through a
  /// `SelectionHandoff::ThroughDispatch` hand-off, which clears
  /// `selected_database()` but keeps `state()` at `NativeAuthoritative` -
  /// see `adopt_selected_database_via_dispatch`). Unconditionally marking
  /// `Converting` there would erase that terminal state, make
  /// `run_conversion`'s own already-selected guard miss, and send a task to
  /// re-inspect files while dispatch's own `NativeDatabase` still holds the
  /// file open.
  #[test]
  fn starting_after_a_dispatch_hand_off_leaves_native_authoritative_untouched_and_spawns_nothing()
   {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    let owner = NativeLifecycleOwner::new();
    // Mirrors `adopt_selected_database_via_dispatch` after a successful
    // hand-off: `state()` is `NativeAuthoritative` but `selected_database()`
    // is empty, because dispatch (not `owner`) holds the open file.
    owner.set_state(DatabaseLifecycleState::NativeAuthoritative);

    let claim = runtime.begin_attempt_marking_converting(&owner).unwrap();

    assert!(
      claim.is_none(),
      "nothing to start once the native database is already selected"
    );
    assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
    // The refusal must not have claimed the in-progress flag either - a
    // later, legitimate start (e.g. after Reset) must still be able to.
    assert!(
      runtime.begin_attempt().is_some(),
      "a refused claim above must not strand the in-progress flag"
    );
  }

  /// And for an already-running `Converting` attempt observed through a
  /// different owner instance (or a state a caller set directly): a second
  /// Start must not reset its progress back to `Preflight`.
  #[test]
  fn starting_while_already_converting_leaves_its_progress_untouched() {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    let owner = NativeLifecycleOwner::new();
    owner.set_state(DatabaseLifecycleState::Converting(
      ConversionProgress::Reconciling,
    ));

    let claim = runtime.begin_attempt_marking_converting(&owner).unwrap();

    assert!(claim.is_none());
    assert_eq!(
      owner.state(),
      DatabaseLifecycleState::Converting(ConversionProgress::Reconciling)
    );
  }

  /// Regression for a PR #2252 review finding: `ActionRequired(Authority(_)
  /// | FreshCreationFailed)` names a files-level disagreement or a
  /// fresh-profile failure a plain restart of this flow cannot resolve, so
  /// a stale Start control must not paper over it with an optimistic
  /// `Converting`.
  #[test]
  fn starting_while_a_files_level_action_required_leaves_it_untouched_and_spawns_nothing()
  {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    let owner = NativeLifecycleOwner::new();
    let issue = LifecycleIssue::Authority(
      hardviz_core::infrastructure::database::native_database::AuthorityInconsistency::MarkerUnreadable,
    );
    owner.set_state(DatabaseLifecycleState::ActionRequired(issue.clone()));

    let claim = runtime.begin_attempt_marking_converting(&owner).unwrap();

    assert!(claim.is_none());
    assert_eq!(owner.state(), DatabaseLifecycleState::ActionRequired(issue));
  }

  #[test]
  fn starting_while_fresh_creation_failed_leaves_it_untouched_and_spawns_nothing() {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    let owner = NativeLifecycleOwner::new();
    let issue = LifecycleIssue::FreshCreationFailed {
      message: "test fixture".to_string(),
    };
    owner.set_state(DatabaseLifecycleState::ActionRequired(issue.clone()));

    let claim = runtime.begin_attempt_marking_converting(&owner).unwrap();

    assert!(claim.is_none());
    assert_eq!(owner.state(), DatabaseLifecycleState::ActionRequired(issue));
  }

  /// Regression for a second PR #2252 review finding: refusing every
  /// `ActionRequired` made the UI's "Try Again" a permanent no-op after a
  /// failed or cancelled conversion attempt, and broke the
  /// `NativeOpenFailed` recovery path #2277 adds (that retry re-enters
  /// `run_conversion`, which re-reads the already-durable disk selection and
  /// retries the open - see `is_startable_state`'s own documentation). These
  /// three reasons must still admit a fresh claim and mark `Converting`.
  #[test]
  fn starting_after_a_failed_cancelled_or_open_failed_attempt_is_admitted() {
    let recoverable_issues = [
      LifecycleIssue::ConversionFailed {
        step: ConversionProgress::Reconciling,
        message: "test fixture".to_string(),
      },
      LifecycleIssue::ConversionCancelled {
        step: ConversionProgress::BuildingCandidate,
      },
      LifecycleIssue::NativeOpenFailed {
        message: "test fixture".to_string(),
      },
    ];

    for issue in recoverable_issues {
      let runtime = ConversionRuntime::default();
      runtime.set_bus(hardviz_core::event_bus::EventBus::new());
      let owner = NativeLifecycleOwner::new();
      owner.set_state(DatabaseLifecycleState::ActionRequired(issue.clone()));

      let claim = runtime.begin_attempt_marking_converting(&owner).unwrap();

      assert!(claim.is_some(), "{issue:?} must be admitted");
      assert_eq!(
        owner.state(),
        DatabaseLifecycleState::Converting(ConversionProgress::Preflight),
        "{issue:?} must mark Converting(Preflight)"
      );
    }
  }

  /// Regression for a PR #2252 review finding on atomicity: the eligibility
  /// check and the `Converting` write must happen as one operation under
  /// the owner's own lock (see `set_state_if`), not a separate read
  /// followed by a separate write - otherwise a concurrent completion
  /// landing in between would be silently overwritten. This cannot force a
  /// true multi-threaded interleaving deterministically, so it instead
  /// proves the read is never stale: a state change made *after* the first
  /// admission (simulating a different in-flight attempt reaching a
  /// terminal state while this one was still marked `Converting`) must be
  /// exactly what the next admission's check observes.
  #[test]
  fn a_state_change_between_two_admissions_is_observed_by_the_next_ones_check() {
    let runtime = ConversionRuntime::default();
    runtime.set_bus(hardviz_core::event_bus::EventBus::new());
    let owner = NativeLifecycleOwner::new();

    let first = runtime.begin_attempt_marking_converting(&owner).unwrap();
    assert!(first.is_some());
    // Simulates a concurrent completion: the in-flight attempt this claim
    // was for finishes and reaches a terminal state, without this test
    // going through the full driver.
    runtime.end_attempt();
    owner.set_state(DatabaseLifecycleState::NativeAuthoritative);

    let second = runtime.begin_attempt_marking_converting(&owner).unwrap();

    assert!(
      second.is_none(),
      "the second admission must see the state the first left behind, not \
       one cached from before it ran"
    );
    assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
  }
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

    fn target(&self) -> ConversionTarget {
      ConversionTarget {
        paths: self.paths(),
        workspace: self.workspace(),
        expected_schema_version: native_schema::NATIVE_SCHEMA_VERSION,
      }
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
      fixture.target(),
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
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
      fixture.target(),
      &owner,
      &workers,
      resumers,
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
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

  #[test]
  fn discard_stale_conversion_work_removes_only_recognized_debris_directories() {
    let directory = tempfile::tempdir().unwrap();
    let stale_driver = directory
      .path()
      .join(".hardwarevisualizer-duckdb-driver-abc123");
    let stale_finalize = directory
      .path()
      .join(".hardwarevisualizer-duckdb-finalize-def456");
    let unrelated_dir = directory.path().join("not-debris");
    let unrelated_file = directory
      .path()
      .join(".hardwarevisualizer-duckdb-not-a-directory");

    std::fs::create_dir(&stale_driver).unwrap();
    std::fs::write(stale_driver.join("candidate.duckdb"), b"partial").unwrap();
    std::fs::create_dir(&stale_finalize).unwrap();
    std::fs::create_dir(&unrelated_dir).unwrap();
    std::fs::write(&unrelated_file, b"not a directory, must survive").unwrap();

    discard_stale_conversion_work(directory.path());

    assert!(!stale_driver.exists());
    assert!(!stale_finalize.exists());
    assert!(unrelated_dir.exists());
    assert!(unrelated_file.exists());
  }

  #[test]
  fn discard_stale_conversion_work_on_a_clean_workspace_is_a_no_op() {
    let directory = tempfile::tempdir().unwrap();
    // Must not panic or error when there is nothing to discard.
    discard_stale_conversion_work(directory.path());
    discard_stale_conversion_work(&directory.path().join("does-not-exist"));
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
      fixture.target(),
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
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
      fixture.target(),
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &cancellation,
      SelectionHandoff::OwnerOnly,
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
      fixture.target(),
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
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

  /// Regression for #2238. The selection commits into the native file
  /// before the marker is written, so a marker write that fails leaves a
  /// durable selection behind. Treating that as a retryable failure resumed
  /// the producers on SQLite, and the next startup repaired the marker and
  /// retired SQLite without the rows they wrote in between.
  #[tokio::test]
  async fn a_failure_after_the_selection_commit_keeps_producers_off_sqlite() {
    let fixture = Fixture::new().await;
    // The marker's directory does not exist: entry reads the marker as
    // absent, the conversion runs, the selection commits, and publishing
    // the marker (and repairing it) fails.
    let marker_directory = fixture.directory.path().join("marker");
    let mut target = fixture.target();
    target.paths.marker = marker_directory.join(AUTHORITY_MARKER_FILE_NAME);
    let paths = target.paths.clone();
    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();

    let outcome = run_conversion(
      target,
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await;

    assert!(
      matches!(outcome, Ok(ConversionOutcome::ActionRequired)),
      "a failure after the commit is not a retryable failure: {outcome:?}"
    );
    assert!(
      !matches!(
        owner.state(),
        DatabaseLifecycleState::ActionRequired(LifecycleIssue::ConversionFailed { .. })
      ),
      "{:?}",
      owner.state()
    );
    assert!(
      workers.cooling_rollup.lock().unwrap().is_none(),
      "producers must not resume while SQLite is no longer authoritative"
    );

    // Once the cause is gone, a retry finishes the selection and only then
    // starts the producers again.
    std::fs::create_dir(&marker_directory).unwrap();
    let retry = run_conversion(
      ConversionTarget {
        paths: paths.clone(),
        workspace: fixture.workspace(),
        expected_schema_version: native_schema::NATIVE_SCHEMA_VERSION,
      },
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await
    .unwrap();

    assert!(
      matches!(retry, ConversionOutcome::AlreadySelected),
      "{retry:?}"
    );
    assert_eq!(owner.state(), DatabaseLifecycleState::NativeAuthoritative);
    assert!(paths.marker.is_file());
    assert!(workers.cooling_rollup.lock().unwrap().is_some());
  }

  /// Candidate, finalize and reconcile against the fixture, returning the
  /// proof `select_native_database` takes.
  async fn reconciled(
    fixture: &Fixture,
  ) -> hardviz_core::infrastructure::database::native_database::VerifiedNativeDatabase {
    let paths = fixture.paths();
    let candidate = fixture.directory.path().join("candidate.duckdb");
    build_candidate_database(
      &paths.source_database,
      &candidate,
      migration::get_migrations(),
    )
    .await
    .unwrap();
    finalize_candidate_database(
      &candidate,
      &paths.native_database,
      native_schema::get_native_schema(),
    )
    .await
    .unwrap();
    std::fs::remove_file(&candidate).unwrap();
    reconcile_native_database(
      &paths.source_database,
      &paths.native_database,
      migration::get_migrations(),
      native_schema::get_native_schema(),
    )
    .await
    .unwrap()
    .1
  }

  fn injected_failure() -> NativeDatabaseError {
    NativeDatabaseError::Worker {
      message: "injected failure".to_owned(),
    }
  }

  #[tokio::test]
  async fn a_failure_after_the_commit_finishes_the_selection_when_the_marker_can_be_repaired()
   {
    // The most common post-commit failures (a checkpoint, `.wal` or sync
    // error) leave the marker path usable, so the gap closes right away and
    // the conversion carries on to open and hand over the database.
    let fixture = Fixture::new().await;
    let verified = reconciled(&fixture).await;
    select_native_database(fixture.paths(), verified)
      .await
      .unwrap();
    std::fs::remove_file(fixture.paths().marker).unwrap();
    let owner = NativeLifecycleOwner::new();

    let settled = settle_failed_selection(
      &owner,
      &fixture.paths(),
      native_schema::NATIVE_SCHEMA_VERSION,
      SelectionHandoff::OwnerOnly,
      injected_failure(),
    )
    .await;

    assert!(matches!(settled, Ok(None)), "{settled:?}");
    assert!(fixture.paths().marker.is_file());
    assert_eq!(
      inspect_startup_authority(&fixture.paths(), native_schema::NATIVE_SCHEMA_VERSION),
      DatabaseLifecycleState::NativeAuthoritative
    );
  }

  #[tokio::test]
  async fn a_failure_whose_native_metadata_cannot_be_read_keeps_producers_paused() {
    // The I/O or `.wal` problem that failed the checkpoint can also hide a
    // committed `selected` from the re-inspection, which then reads as an
    // interrupted conversion. That is not proof the commit did not land, so
    // the failure must not be treated as retryable with SQLite authoritative.
    // Unreadable bytes stand in for "cannot be read right now".
    let fixture = Fixture::new().await;
    let verified = reconciled(&fixture).await;
    select_native_database(fixture.paths(), verified)
      .await
      .unwrap();
    std::fs::remove_file(fixture.paths().marker).unwrap();
    std::fs::write(fixture.paths().native_database, b"not a database").unwrap();
    assert_eq!(
      inspect_startup_authority(&fixture.paths(), native_schema::NATIVE_SCHEMA_VERSION),
      DatabaseLifecycleState::ConversionRecoverable { resumable: false }
    );
    let owner = NativeLifecycleOwner::new();

    let settled = settle_failed_selection(
      &owner,
      &fixture.paths(),
      native_schema::NATIVE_SCHEMA_VERSION,
      SelectionHandoff::OwnerOnly,
      injected_failure(),
    )
    .await;

    // `run_conversion` leaves the producers paused for exactly this outcome.
    assert!(
      matches!(settled, Ok(Some(ConversionOutcome::ActionRequired))),
      "{settled:?}"
    );
    assert_eq!(
      owner.state(),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(
        hardviz_core::infrastructure::database::native_database::AuthorityInconsistency::NativeMetadataUnreadable
      ))
    );
  }

  #[test]
  fn only_a_positive_absence_or_unselected_read_counts_as_before_the_commit() {
    use std::io::{Error, ErrorKind};

    let missing = || Err(Error::from(ErrorKind::NotFound));
    let denied = || Err(Error::from(ErrorKind::PermissionDenied));
    let other = || Err(Error::other("device not ready"));
    let directory = tempfile::tempdir().unwrap();
    let present = || std::fs::symlink_metadata(directory.path());

    let sqlite = DatabaseLifecycleState::SqliteAuthoritative;
    let unreadable = DatabaseLifecycleState::ConversionRecoverable { resumable: false };
    let unselected = DatabaseLifecycleState::ConversionRecoverable { resumable: true };

    // Absence is proven only by `NotFound`; any other probe result is doubt.
    assert!(failed_before_commit(&sqlite, &missing()));
    assert!(failed_before_commit(&unreadable, &missing()));
    for probe in [denied(), other(), present()] {
      assert!(!failed_before_commit(&sqlite, &probe), "{probe:?}");
      assert!(!failed_before_commit(&unreadable, &probe), "{probe:?}");
    }
    // Metadata already read back as unselected needs no probe.
    assert!(failed_before_commit(&unselected, &present()));
    // A selection the files show, or a disagreement, is never "before".
    for state in [
      DatabaseLifecycleState::NativeAuthoritative,
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(
        hardviz_core::infrastructure::database::native_database::AuthorityInconsistency::SelectedWithoutMarker,
      )),
    ] {
      assert!(!failed_before_commit(&state, &missing()), "{state:?}");
    }
  }

  #[tokio::test]
  async fn a_failure_before_the_commit_stays_a_retryable_conversion_failure() {
    let fixture = Fixture::new().await;
    reconciled(&fixture).await;
    let owner = NativeLifecycleOwner::new();

    let settled = settle_failed_selection(
      &owner,
      &fixture.paths(),
      native_schema::NATIVE_SCHEMA_VERSION,
      SelectionHandoff::OwnerOnly,
      injected_failure(),
    )
    .await;

    assert!(
      matches!(settled, Err(ConversionError::Select(_))),
      "{settled:?}"
    );
    assert!(matches!(
      owner.state(),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::ConversionFailed {
        step: ConversionProgress::Selecting,
        ..
      })
    ));
    assert!(!fixture.paths().marker.exists());
  }

  #[tokio::test]
  async fn a_second_call_after_selection_is_a_no_op() {
    let fixture = Fixture::new().await;
    let owner = NativeLifecycleOwner::new();
    let workers = WorkersState::default();

    run_conversion(
      fixture.target(),
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await
    .unwrap();

    let second = run_conversion(
      fixture.target(),
      &owner,
      &workers,
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await
    .unwrap();

    assert!(matches!(second, ConversionOutcome::AlreadySelected));
  }

  #[tokio::test]
  async fn a_fresh_owner_opens_an_already_selected_database_instead_of_assuming_it_is_open()
   {
    // Before the fix this closes over, `run_conversion` mapped a disk-level
    // `NativeAuthoritative` straight to `AlreadySelected` without ever
    // calling `open_selected_database` - correct only for the owner that
    // did the selecting itself (caught by the guard at the very top of this
    // function). A second, independent `NativeLifecycleOwner` - a later
    // call in the same process, or #2136 constructing a fresh one - has an
    // empty seam and disk-level `NativeAuthoritative` at the same time, and
    // needs the entry path to actually open the database rather than
    // reporting success over nothing.
    let fixture = Fixture::new().await;
    let first_owner = NativeLifecycleOwner::new();

    let outcome = run_conversion(
      fixture.target(),
      &first_owner,
      &WorkersState::default(),
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await
    .unwrap();
    assert!(matches!(outcome, ConversionOutcome::Selected { .. }));
    // Release the handle so the second owner's open below is not blocked by
    // DuckDB's single-instance rule - a separate case, covered next.
    first_owner
      .selected_database()
      .unwrap()
      .close()
      .await
      .unwrap();

    let second_owner = NativeLifecycleOwner::new();
    assert!(second_owner.selected_database().is_none());
    let outcome = run_conversion(
      fixture.target(),
      &second_owner,
      &WorkersState::default(),
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, ConversionOutcome::AlreadySelected));
    assert_eq!(
      second_owner.state(),
      DatabaseLifecycleState::NativeAuthoritative
    );
    assert!(
      second_owner.selected_database().is_some(),
      "the entry path must open the database, not just report success"
    );
  }

  #[tokio::test]
  async fn a_concurrently_held_database_is_reported_as_action_required_not_silently_skipped()
   {
    // Another owner in this process already holds the selected database. The
    // second owner must be refused and record why, never treat it as done.
    // Which layer refuses depends on the platform: on Windows DuckDB's own
    // lock makes `observe_authority` fail to read the metadata, so entry
    // reports `ActionRequired(Authority(NativeMetadataUnreadable))`. On Linux
    // and macOS DuckDB's `fcntl` lock is process-scoped and lets that read
    // through; entry then reaches the open-and-retry branch, where
    // `NativeDatabase::open`'s in-process claim refuses the second owner with
    // `AlreadyOpen`, and the owner records `ActionRequired(NativeOpenFailed)`.
    // Both converge on the guarantee pinned here.
    let fixture = Fixture::new().await;
    let first_owner = NativeLifecycleOwner::new();

    run_conversion(
      fixture.target(),
      &first_owner,
      &WorkersState::default(),
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await
    .unwrap();
    // `first_owner` deliberately keeps its handle open.

    let second_owner = NativeLifecycleOwner::new();
    let outcome = run_conversion(
      fixture.target(),
      &second_owner,
      &WorkersState::default(),
      empty_resumers(tokio::runtime::Handle::current()),
      &ConversionCancellation::new(),
      SelectionHandoff::OwnerOnly,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, ConversionOutcome::ActionRequired));
    assert!(
      matches!(
        second_owner.state(),
        DatabaseLifecycleState::ActionRequired(_)
      ),
      "{:?}",
      second_owner.state()
    );
    assert!(second_owner.selected_database().is_none());
  }
}
