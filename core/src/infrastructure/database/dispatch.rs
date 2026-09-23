//! The single boundary every database consumer is answered through.
//!
//! # What decides the answer
//!
//! The boundary holds the *durable selection state* recorded by
//! [`super::native_database::selection`] - never a per-call flag, and never a
//! copy taken once at startup that could go stale the moment a later
//! selection is recorded. Concretely: a live [`NativeDatabase`] owner, or the
//! absence of one, sitting behind an [`RwLock`] that every dispatch function
//! reads at call time.
//!
//! The boundary does **not** poll [`observe_authority`] per call to decide.
//! Doing so would violate the ordering contract documented there - reading the
//! native metadata opens a second DuckDB instance beside this boundary's own
//! live owner, which Windows refuses and Linux and macOS let through unsafely.
//! Instead the boundary's answer
//! changes only when something tells it to: once at App startup ([`init`] +
//! [`reobserve_authority`]), and again whenever the App lifecycle owner records
//! a new selection (see "Seam for #2135" below). Between those calls the
//! `RwLock` is the single source of truth, so every consumer - including one
//! called in the instant after [`super::native_database::select_native_database`]
//! has committed but before this boundary has been told - keeps getting a
//! consistent answer instead of racing a filesystem check.
//! Process shutdown also closes the boundary after its database-backed workers
//! drain, so later consumers are refused without reopening either backend.
//!
//! # Feature-disabled behaviour
//!
//! With `duckdb-archive` disabled, every function in this module is a direct,
//! zero-overhead call into the existing SQLite function it replaces at the
//! call site: no lock, no native types, nothing new on the wire. The two
//! builds of this module intentionally do not share a body - see the
//! per-family submodules below.
//!
//! # Consumers routed here
//!
//! Process Stats, the four raw archive families (`DATA_ARCHIVE`,
//! `GPU_DATA_ARCHIVE`, Ambient, Fan), Storage Health, Cooling's six
//! projections (daily, hourly, fan, thermal delta, and both co-variate
//! shapes), both cooling baselines and the cooling rollup persistence are
//! all routed. The rollup write stays one native transaction on the
//! selected path, matching the single SQLite transaction
//! `persistence::cooling_rollup::persist_day_rollup_from_pool` uses - see
//! [`cooling_rollup::persist_day_rollup`].
//!
//! # Seam for #2135
//!
//! The App-side conversion lifecycle (#2135) owns resolving paths, driving
//! candidate/finalize/reconcile/select, and quiescing SQLite writers around
//! the final reconciliation and selection. This boundary exposes exactly two
//! entry points for that owner, both `#[cfg(feature = "duckdb-archive")]`:
//!
//! - [`init(paths, expected_schema_version)`](init) - called once at App
//!   startup, next to `db::init`, before any consumer runs. Tells the boundary
//!   where to look; does not open anything.
//! - [`reobserve_authority()`](reobserve_authority) - called after
//!   [`super::native_database::select_native_database`] returns (or at startup,
//!   after `init`, to establish the first answer). Closes whatever live owner
//!   this boundary currently holds (a no-op the first time), re-runs
//!   [`observe_authority`]/[`inspect_authority`], and - only if the durable
//!   state is [`AuthorityState::NativeSelected`] - opens exactly one new
//!   [`NativeDatabase`] and adopts it. The three states where SQLite is
//!   genuinely still authoritative
//!   ([`AuthorityState::SqliteAuthoritative`],
//!   [`AuthorityState::ConversionInProgress`],
//!   [`AuthorityState::FinalizedUnselected`]) leave the boundary on SQLite.
//!   Anything else - a failed open, or [`AuthorityState::Inconsistent`] -
//!   leaves every consumer refused with
//!   [`DispatchError::NativeUnavailable`] until a later call reports
//!   something else: once the durable record says native is or may be
//!   selected, answering from SQLite would silently diverge from the
//!   authoritative backend, so the boundary answers nothing rather than
//!   guess. See `boundary::reobserve_authority`'s own doc for the exact
//!   three-way split.
//! - [`shutdown()`](shutdown) - close the live native owner and move the
//!   boundary to a closed state without deciding a new backend. Idempotent;
//!   call it during process shutdown, after database-backed workers have
//!   drained and before the process exits, so the two DuckDB lane threads are
//!   joined deliberately instead of torn down with the process. Any later
//!   consumer is refused before it can reopen SQLite or native storage.
//!
//! `reobserve_authority` was chosen over a "hand me an already-open database"
//! entry point because it is the one that cannot race a second instance
//! opening the file: it always closes its own current owner (if any) before it
//! looks at disk again, and it is the only thing that ever opens a
//! [`NativeDatabase`] on this boundary's behalf. A lifecycle owner that has
//! just called `select_native_database` (which opens and closes its own
//! scoped connection to record the selection, and is not this boundary's
//! owner) simply calls `reobserve_authority()` next; nothing else is needed.

use chrono::{DateTime, NaiveDate, Utc};

use super::archive_queries::{
  AmbientArchiveSeries, ArchiveBucketTimestamp, ArchiveSeriesError, ArchiveSeriesPoint,
  DataArchiveColumn, FanArchiveSeries, GpuArchiveColumn, ProcessStatRecord,
};
use crate::models::hardware::{
  StorageDeviceRecord, StorageHealthRecord, StorageHealthRecordDraft,
};
use crate::persistence::archive_data::{
  AmbientData, FanArchiveRow, GpuData, HardwareArchiveRow, ProcessStatData,
};
use crate::persistence::cooling_baseline::{DailyIdleSample, EstablishedBaseline};
use crate::persistence::cooling_covariate_rollup::{
  CovariateDailySummary, CovariateDaySummary, FanCovariateDailySummary,
};
use crate::persistence::cooling_delta_baseline::EstablishedDeltaBaseline;
use crate::persistence::cooling_fan_rollup::{FanArchiveMinuteSample, FanDailySummary};
use crate::persistence::cooling_hourly_rollup::HourlyCoolingSummary;
use crate::persistence::cooling_rollup::{ArchiveMinuteSample, DailyCoolingSummary};
use crate::persistence::cooling_thermal_delta_rollup::{
  ThermalDeltaDailySummary, ThermalDeltaMinuteSample,
};

/// The one error type every dispatch function returns, regardless of which
/// backend answered. Callers that only need `{e}` (every current call site)
/// need not match on it.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
  #[error(transparent)]
  Sqlite(#[from] sqlx::Error),
  #[error(transparent)]
  ArchiveSeries(#[from] ArchiveSeriesError),
  #[cfg(feature = "duckdb-archive")]
  #[error(transparent)]
  Native(#[from] super::native_database::NativeDatabaseError),
  /// The durable authority state says a native database is (or may be)
  /// selected, but this boundary cannot currently serve it - a failed
  /// `NativeDatabase::open`, or any `AuthorityState::Inconsistent` finding.
  /// Falling through to SQLite here would answer from a backend the durable
  /// record no longer treats as authoritative, silently diverging from
  /// whatever was written natively; refusing is the only response that
  /// cannot lose history. See `boundary::reobserve_authority`.
  #[cfg(feature = "duckdb-archive")]
  #[error(
    "the native database is not safely servable ({reason}); refusing to answer from SQLite \
     because the durable authority record no longer treats it as authoritative"
  )]
  NativeUnavailable { reason: String },
  /// The App has drained database-backed workers and shut down this boundary.
  /// A late consumer must not reopen SQLite or the native database.
  #[cfg(feature = "duckdb-archive")]
  #[error("the database dispatch boundary has shut down; refusing database access")]
  Shutdown,
}

#[cfg(feature = "duckdb-archive")]
mod boundary {
  use std::sync::{Arc, OnceLock};

  use tokio::sync::RwLock;

  use super::super::native_database::{
    AuthorityPaths, AuthorityState, NativeDatabase, NativeDatabaseError,
    NativeDatabaseOptions, inspect_authority, observe_authority,
  };
  use super::DispatchError;

  struct Config {
    paths: AuthorityPaths,
    expected_schema_version: u32,
  }

  /// Which backend this boundary currently answers from. `Unavailable`
  /// refuses a consumer rather than silently reading stale SQLite rows - see
  /// the module doc's "What decides the answer" and
  /// [`DispatchError::NativeUnavailable`]. `Shutdown` refuses any consumer
  /// after the App has drained its database-backed workers.
  enum Active {
    Sqlite,
    Native(NativeDatabase),
    /// Non-serving state installed before the first reobserve await. Recovery
    /// runs in a detached task while holding the write lock, so readers cannot
    /// reach this state or serve stale SQLite during the handoff.
    Reobserving,
    Unavailable(String),
    Shutdown,
  }

  /// What a dispatch function actually dispatches to. Not `Unavailable`:
  /// that case is resolved into an `Err` by [`resolve_backend`] before any
  /// caller sees it.
  pub(super) enum Backend {
    Sqlite,
    Native(NativeDatabase),
  }

  static CONFIG: OnceLock<Config> = OnceLock::new();
  static ACTIVE: OnceLock<Arc<RwLock<Active>>> = OnceLock::new();

  /// The ordinary state before [`reobserve_authority`] has ever run - a
  /// fresh install, or a consumer called before App startup wiring reaches
  /// it (every current test does this deliberately, to prove the
  /// not-selected path needs no configuration).
  fn active() -> Arc<RwLock<Active>> {
    Arc::clone(ACTIVE.get_or_init(|| Arc::new(RwLock::new(Active::Sqlite))))
  }

  fn config() -> &'static Config {
    CONFIG.get().expect(
      "hardviz_core::infrastructure::database::dispatch::init must be called \
       before any dispatch consumer runs with duckdb-archive enabled",
    )
  }

  /// App-owned initialisation. App resolves paths (Core never resolves
  /// app-data paths itself) and hands them here once at startup, next to
  /// `db::init`. Returns `true` if this call set the process-wide
  /// configuration, `false` if an earlier call already had - mirroring
  /// `db::init`'s contract, for the same reason: a `OnceLock` cannot be
  /// replaced, so a caller that needs to know whether its paths took effect
  /// (test fixtures sharing a process) should check the return value.
  ///
  /// Does not open the native database; call [`reobserve_authority`]
  /// afterwards to establish the first answer.
  pub fn init(paths: AuthorityPaths, expected_schema_version: u32) -> bool {
    CONFIG
      .set(Config {
        paths,
        expected_schema_version,
      })
      .is_ok()
  }

  /// Close whatever native owner this boundary currently holds, then look at
  /// disk again and decide the next answer. See the module doc's "Seam for
  /// #2135" for when to call this. Once [`shutdown`] has run, reobservation
  /// returns [`DispatchError::Shutdown`] without changing the terminal state.
  ///
  /// Three outcomes, by durable state:
  /// - [`AuthorityState::SqliteAuthoritative`],
  ///   [`AuthorityState::ConversionInProgress`] or
  ///   [`AuthorityState::FinalizedUnselected`]: SQLite is genuinely still
  ///   authoritative, so the boundary answers from it.
  /// - [`AuthorityState::NativeSelected`]: the boundary opens exactly one new
  ///   [`NativeDatabase`] and answers from it. If the open itself fails -
  ///   the durable record says selected, but the file cannot be opened - the
  ///   boundary is left [`Active::Unavailable`] and this call returns `Err`:
  ///   the caller must not silently keep going on SQLite, because SQLite is
  ///   a stale recovery copy the instant a selection is durable.
  /// - [`AuthorityState::Inconsistent`]: same refusal. The state is still
  ///   returned as `Ok` (it already carries the `reason` and `recovery`
  ///   [`super::super::native_database::AuthorityRecovery`] a caller needs to
  ///   decide whether repair is safe - see
  ///   [`super::super::native_database::repair_authority_marker`]), but the
  ///   boundary answers nothing until a later `reobserve_authority` call
  ///   reports something else.
  pub async fn reobserve_authority() -> Result<AuthorityState, DispatchError> {
    let active = active();
    tokio::spawn(async move {
      let mut guard = active.write().await;
      reobserve_authority_locked(&mut guard, config()).await
    })
    .await
    .map_err(|error| DispatchError::NativeUnavailable {
      reason: format!("the native authority reobservation task failed: {error}"),
    })?
  }

  /// Reobserve while the caller holds the active-state write lock. Keeping
  /// the lock across close and reopen preserves the one-owner handoff.
  async fn reobserve_authority_locked(
    guard: &mut Active,
    config: &Config,
  ) -> Result<AuthorityState, DispatchError> {
    if matches!(&*guard, Active::Shutdown) {
      return Err(DispatchError::Shutdown);
    }
    if matches!(&*guard, Active::Reobserving) {
      return Err(DispatchError::NativeUnavailable {
        reason: "a previous native authority reobservation was interrupted; restart the app to establish a new owner".to_owned(),
      });
    }
    // Set a non-serving state before close/open can yield. If this future is
    // cancelled at either await, the write guard may be released, but SQLite
    // must not answer while native remains durably selected.
    let previous = std::mem::replace(guard, Active::Reobserving);
    if let Active::Native(database) = previous {
      close_or_refuse(guard, database).await?;
    }
    let facts = observe_authority(&config.paths, config.expected_schema_version);
    let state = inspect_authority(&facts);
    let next = match state {
      AuthorityState::SqliteAuthoritative
      | AuthorityState::ConversionInProgress { .. }
      | AuthorityState::FinalizedUnselected => Active::Sqlite,
      AuthorityState::NativeSelected => {
        match NativeDatabase::open(
          &config.paths.native_database,
          NativeDatabaseOptions::new(config.expected_schema_version),
        )
        .await
        {
          Ok(database) => Active::Native(database),
          Err(error) => {
            // The durable record says selected, but the file cannot be
            // opened. Leaving the boundary unavailable - never falling back
            // to `Active::Sqlite` - is the point of this whole change: SQLite
            // is a stale recovery copy the instant a selection is durable.
            *guard = Active::Unavailable(format!(
              "the durable state says selected, but opening it failed: {error}"
            ));
            return Err(DispatchError::Native(error));
          }
        }
      }
      AuthorityState::Inconsistent { reason, recovery } => {
        Active::Unavailable(format!("{reason:?} ({recovery:?})"))
      }
    };
    *guard = next;
    Ok(state)
  }

  /// Close the live native owner, if any, without deciding a new backend, and
  /// refuse every later consumer. The App calls this after database-backed
  /// workers have drained; returning to SQLite here could let a late consumer
  /// recreate the database file while the process is shutting down.
  pub async fn shutdown() -> Result<(), NativeDatabaseError> {
    let active = active();
    let mut guard = active.write().await;
    if let Active::Native(database) = std::mem::replace(&mut *guard, Active::Shutdown) {
      close_or_refuse(&mut guard, database).await?;
    }
    Ok(())
  }

  /// Close a native owner that was just taken out of the boundary.
  ///
  /// The caller has already replaced the owner with a non-serving transition
  /// state. If close fails, the durable state may still say native is selected,
  /// so the boundary is left [`Active::Unavailable`] rather than answering
  /// from a stale SQLite copy.
  async fn close_or_refuse(
    guard: &mut Active,
    database: NativeDatabase,
  ) -> Result<(), NativeDatabaseError> {
    if let Err(error) = database.close().await {
      mark_unavailable_after_close_failure(
        guard,
        format!("closing the previous native owner failed: {error}"),
      );
      return Err(error);
    }
    Ok(())
  }

  fn mark_unavailable_after_close_failure(guard: &mut Active, reason: String) {
    if !matches!(&*guard, Active::Shutdown) {
      *guard = Active::Unavailable(reason);
    }
  }

  /// The backend to dispatch to, or the typed refusal if the durable state
  /// says native is selected (or ambiguous) and this boundary cannot safely
  /// serve either engine. A failed DuckDB checkpoint marks the current owner
  /// invalid; the next consumer reuses the same close-and-reobserve handoff
  /// before it can issue another request.
  pub(super) async fn resolve_backend() -> Result<Backend, DispatchError> {
    resolve_backend_with(active(), None).await
  }

  async fn resolve_backend_with(
    active: Arc<RwLock<Active>>,
    configured: Option<Arc<Config>>,
  ) -> Result<Backend, DispatchError> {
    {
      let guard = active.read().await;
      match &*guard {
        Active::Native(database) if database.is_invalidated() => {}
        state => return backend_from_active(state),
      }
    }

    // Another request can observe the same invalidation. Recheck after taking
    // the write lock so only the first task closes and reopens the owner.
    // Detaching the handoff means cancelling this consumer only drops its join
    // handle; it cannot interrupt close/open or expose stale SQLite.
    let recovery = tokio::spawn(async move {
      let mut guard = active.write().await;
      let should_reobserve =
        matches!(&*guard, Active::Native(database) if database.is_invalidated());
      if should_reobserve {
        let config = match configured.as_deref() {
          Some(config) => config,
          None => config(),
        };
        reobserve_authority_locked(&mut guard, config).await?;
      }
      backend_from_active(&guard)
    });
    recovery
      .await
      .map_err(|error| DispatchError::NativeUnavailable {
        reason: format!("the native database recovery task failed: {error}"),
      })?
  }

  fn backend_from_active(active: &Active) -> Result<Backend, DispatchError> {
    match active {
      Active::Sqlite => Ok(Backend::Sqlite),
      // Cloning is an `Arc` bump, not a new connection.
      Active::Native(database) => Ok(Backend::Native(database.clone())),
      Active::Unavailable(reason) => Err(DispatchError::NativeUnavailable {
        reason: reason.clone(),
      }),
      Active::Reobserving => Err(DispatchError::NativeUnavailable {
        reason: "native authority reobservation did not finish; restart the app to establish a new owner".to_owned(),
      }),
      Active::Shutdown => Err(DispatchError::Shutdown),
    }
  }

  #[cfg(test)]
  mod tests {
    use std::sync::{
      Arc,
      atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    use tokio::sync::RwLock;

    use super::{
      Active, Backend, Config, DispatchError, NativeDatabase, NativeDatabaseError,
      NativeDatabaseOptions, backend_from_active, mark_unavailable_after_close_failure,
      resolve_backend_with,
    };
    use crate::infrastructure::database::native_database::NativeCancellation;
    use crate::infrastructure::database::native_database::{
      AuthorityPaths, NativeIdentity, NativeIdentityMode, NativeSchemaDefinition,
      create_empty_native_database,
    };

    fn schema() -> NativeSchemaDefinition {
      static IDENTITIES: &[NativeIdentity] = &[NativeIdentity {
        table: "probe",
        column: "id",
        mode: NativeIdentityMode::RowId,
      }];
      NativeSchemaDefinition {
        version: 7,
        sql: "CREATE TABLE probe (id BIGINT PRIMARY KEY)",
        tables: &["probe"],
        timestamp_columns: &[],
        identities: IDENTITIES,
      }
    }

    async fn selected_boundary(
      directory: &std::path::Path,
      expected_schema_version: u32,
    ) -> (Arc<RwLock<Active>>, Arc<Config>) {
      let paths =
        AuthorityPaths::in_directory(directory, "source.sqlite3", "native.duckdb");
      create_empty_native_database(paths.clone(), schema())
        .await
        .unwrap();
      let database =
        NativeDatabase::open(&paths.native_database, NativeDatabaseOptions::new(7))
          .await
          .unwrap();
      (
        Arc::new(RwLock::new(Active::Native(database))),
        Arc::new(Config {
          paths,
          expected_schema_version,
        }),
      )
    }

    async fn insert_probe(database: &NativeDatabase, id: i64) {
      database
        .request_write(NativeCancellation::new(), move |context| {
          context
            .connection()
            .execute("INSERT INTO probe VALUES (?)", duckdb::params![id])
            .map(|_| ())
            .map_err(|error| NativeDatabaseError::duckdb("insert test probe", error))
        })
        .await
        .unwrap();
    }

    async fn read_probe(database: &NativeDatabase) -> i64 {
      database
        .request_read(NativeCancellation::new(), |context| {
          context
            .connection()
            .query_row("SELECT id FROM probe", [], |row| row.get::<_, i64>(0))
            .map_err(|error| NativeDatabaseError::duckdb("read test probe", error))
        })
        .await
        .unwrap()
    }

    #[test]
    fn native_close_failure_preserves_shutdown() {
      let mut active = Active::Shutdown;

      mark_unavailable_after_close_failure(
        &mut active,
        "closing the previous native owner failed".to_owned(),
      );

      assert!(matches!(active, Active::Shutdown));
    }

    #[test]
    fn native_close_failure_refuses_consumers_before_shutdown() {
      let mut active = Active::Sqlite;

      mark_unavailable_after_close_failure(
        &mut active,
        "closing the previous native owner failed".to_owned(),
      );

      assert!(matches!(active, Active::Unavailable(_)));
    }

    #[tokio::test]
    async fn sqlite_dispatch_does_not_require_native_configuration() {
      let active = Arc::new(RwLock::new(Active::Sqlite));

      assert!(matches!(
        resolve_backend_with(Arc::clone(&active), None).await,
        Ok(Backend::Sqlite)
      ));
    }

    #[tokio::test]
    async fn explicit_checkpoint_failure_reopens_before_the_next_consumer() {
      let directory = tempfile::tempdir().unwrap();
      let (active, config) = selected_boundary(directory.path(), 7).await;
      let original = match &*active.read().await {
        Active::Native(database) => database.clone(),
        _ => unreachable!(),
      };
      insert_probe(&original, 42).await;
      original.inject_next_checkpoint_failure();

      assert!(
        original
          .checkpoint(NativeCancellation::new())
          .await
          .is_err()
      );
      assert!(original.is_invalidated());

      let Backend::Native(reopened) =
        resolve_backend_with(Arc::clone(&active), Some(Arc::clone(&config)))
          .await
          .unwrap()
      else {
        panic!("the durable selection still names the native database");
      };
      assert_eq!(read_probe(&reopened).await, 42);
      assert!(matches!(
        original
          .request_read(NativeCancellation::new(), |_| Ok(()))
          .await,
        Err(NativeDatabaseError::Invalidated)
      ));
    }

    #[tokio::test]
    async fn automatic_checkpoint_failure_is_not_retried_and_reopens_on_the_next_request()
    {
      let directory = tempfile::tempdir().unwrap();
      let (active, config) = selected_boundary(directory.path(), 7).await;
      let original = match &*active.read().await {
        Active::Native(database) => database.clone(),
        _ => unreachable!(),
      };
      insert_probe(&original, 42).await;
      let attempts = Arc::new(AtomicUsize::new(0));
      let attempt_counter = Arc::clone(&attempts);
      let failure = original
        .request_write(NativeCancellation::new(), move |_| {
          attempt_counter.fetch_add(1, Ordering::Relaxed);
          Err::<(), _>(NativeDatabaseError::duckdb(
            "commit transaction",
            duckdb::Error::DuckDBFailure(
              duckdb::ffi::Error::new(duckdb::ffi::DuckDBError),
              Some(
                "IO Error: Checkpoint failed for database. The database has been invalidated."
                  .to_owned(),
              ),
            ),
          ))
        })
        .await;

      assert!(failure.is_err());
      assert_eq!(attempts.load(Ordering::Relaxed), 1);
      assert!(original.is_invalidated());

      let Backend::Native(reopened) =
        resolve_backend_with(Arc::clone(&active), Some(Arc::clone(&config)))
          .await
          .unwrap()
      else {
        panic!("the durable selection still names the native database");
      };
      assert_eq!(attempts.load(Ordering::Relaxed), 1);
      assert_eq!(read_probe(&reopened).await, 42);
    }

    #[tokio::test]
    async fn unhealthy_lane_worker_error_reopens_before_the_next_request() {
      let directory = tempfile::tempdir().unwrap();
      let (active, config) = selected_boundary(directory.path(), 7).await;
      let original = match &*active.read().await {
        Active::Native(database) => database.clone(),
        _ => unreachable!(),
      };
      insert_probe(&original, 42).await;
      let attempts = Arc::new(AtomicUsize::new(0));
      let attempt_counter = Arc::clone(&attempts);
      original.inject_next_unhealthy_request();

      let failure = original
        .request_write(NativeCancellation::new(), move |_| {
          attempt_counter.fetch_add(1, Ordering::Relaxed);
          Err::<(), _>(NativeDatabaseError::Worker {
            message: "injected rollback failure".to_owned(),
          })
        })
        .await;

      assert!(matches!(
        failure,
        Err(NativeDatabaseError::Worker { ref message })
          if message == "injected rollback failure"
      ));
      assert_eq!(attempts.load(Ordering::Relaxed), 1);
      assert!(original.is_invalidated());

      let Backend::Native(reopened) =
        resolve_backend_with(Arc::clone(&active), Some(Arc::clone(&config)))
          .await
          .unwrap()
      else {
        panic!("the durable selection still names the native database");
      };
      assert_eq!(attempts.load(Ordering::Relaxed), 1);
      assert_eq!(read_probe(&reopened).await, 42);
    }

    #[tokio::test]
    async fn failed_reopen_refuses_the_native_request_instead_of_using_sqlite() {
      let directory = tempfile::tempdir().unwrap();
      let (active, config) = selected_boundary(directory.path(), 8).await;
      let original = match &*active.read().await {
        Active::Native(database) => database.clone(),
        _ => unreachable!(),
      };
      original.inject_next_checkpoint_failure();
      assert!(
        original
          .checkpoint(NativeCancellation::new())
          .await
          .is_err()
      );

      assert!(matches!(
        resolve_backend_with(Arc::clone(&active), Some(Arc::clone(&config))).await,
        Err(DispatchError::NativeUnavailable { .. })
      ));
      let guard = active.read().await;
      assert!(matches!(
        backend_from_active(&guard),
        Err(DispatchError::NativeUnavailable { .. })
      ));
    }

    #[tokio::test]
    async fn cancelled_consumer_does_not_cancel_native_recovery() {
      let directory = tempfile::tempdir().unwrap();
      let (active, config) = selected_boundary(directory.path(), 7).await;
      let original = match &*active.read().await {
        Active::Native(database) => database.clone(),
        _ => unreachable!(),
      };
      // Keep one lane busy so detached recovery waits in close after installing
      // the non-serving transition state.
      let (read_started_tx, read_started_rx) = tokio::sync::oneshot::channel();
      let (release_read_tx, release_read_rx) = std::sync::mpsc::channel();
      let blocked_read = {
        let database = original.clone();
        tokio::spawn(async move {
          database
            .request_read(NativeCancellation::new(), move |_| {
              let _ = read_started_tx.send(());
              release_read_rx
                .recv()
                .map_err(|error| NativeDatabaseError::Worker {
                  message: error.to_string(),
                })?;
              Ok::<(), NativeDatabaseError>(())
            })
            .await
        })
      };
      tokio::time::timeout(Duration::from_secs(3), read_started_rx)
        .await
        .expect("the blocked read should start before testing cancellation")
        .unwrap();
      original.inject_next_checkpoint_failure();
      assert!(
        original
          .checkpoint(NativeCancellation::new())
          .await
          .is_err()
      );

      let recovery = {
        let active = Arc::clone(&active);
        let config = Arc::clone(&config);
        tokio::spawn(async move { resolve_backend_with(active, Some(config)).await })
      };
      tokio::time::timeout(Duration::from_secs(3), async {
        loop {
          if active.try_write().is_err() {
            break;
          }
          tokio::task::yield_now().await;
        }
      })
      .await
      .expect("recovery should be waiting for the blocked lane to close");
      recovery.abort();
      assert!(matches!(recovery.await, Err(error) if error.is_cancelled()));

      // A later consumer waits for the detached handoff instead of being
      // refused or starting to serve SQLite while the old owner is closing.
      let mut next_consumer = {
        let active = Arc::clone(&active);
        let config = Arc::clone(&config);
        tokio::spawn(async move { resolve_backend_with(active, Some(config)).await })
      };
      assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut next_consumer)
          .await
          .is_err()
      );

      release_read_tx.send(()).unwrap();
      blocked_read.await.unwrap().unwrap();

      let reopened = tokio::time::timeout(Duration::from_secs(10), next_consumer)
        .await
        .expect("the later consumer should complete after detached recovery")
        .expect("the later consumer task should not panic")
        .unwrap_or_else(|error| {
          panic!("the selected native database should reopen: {error}")
        });
      let Backend::Native(reopened) = reopened else {
        panic!("the durable selection still names the native database");
      };
      assert_eq!(read_probe(&reopened).await, 42);
      reopened.close().await.unwrap();
      original.close().await.unwrap();
    }
  }
}

#[cfg(feature = "duckdb-archive")]
pub use boundary::{init, reobserve_authority, shutdown};

/// Checkpoint the native database if it is the currently selected backend;
/// a no-op on SQLite (there is nothing to checkpoint, and no live owner to
/// reach one through).
///
/// This is the one operation every dispatch consumer function's own
/// `resolve_backend`/`Backend::Native(database)` pattern would otherwise
/// have to repeat just to reach the same live owner
/// [`NativeDatabase::checkpoint`][cp] measures the cost of - see its own
/// documentation for the measured schedule. #2135's App lifecycle owner
/// calls this once, after a daily expiry pass
/// (`persistence::archive::cleanup_old_data`), through the boundary rather
/// than a second `NativeDatabase` instance of its own.
///
/// [cp]: super::native_database::NativeDatabase::checkpoint
#[cfg(feature = "duckdb-archive")]
pub async fn checkpoint() -> Result<(), DispatchError> {
  match boundary::resolve_backend().await? {
    boundary::Backend::Native(database) => database
      .checkpoint(super::native_database::NativeCancellation::new())
      .await
      .map_err(DispatchError::from),
    boundary::Backend::Sqlite => Ok(()),
  }
}

#[cfg(not(feature = "duckdb-archive"))]
pub async fn checkpoint() -> Result<(), DispatchError> {
  Ok(())
}

/// The Process Stats family: [`super::process_stats`] (SQLite) and
/// [`super::native_database::process_stats`] (native).
pub mod process_stats {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert(
    processes: Vec<ProcessStatData>,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    super::super::process_stats::insert(processes, timestamp)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert(
    processes: Vec<ProcessStatData>,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::process_stats::insert(
          &database,
          super::super::native_database::NativeCancellation::new(),
          processes,
          timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::process_stats::insert(processes, timestamp)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    super::super::process_stats::delete_old_data(retention_days)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::process_stats::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
        )
        .await
        .map(|_deleted| ())
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::process_stats::delete_old_data(retention_days)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_process_stats(
    start: &str,
    end: &str,
    order_by_cpu_desc: bool,
  ) -> Result<Vec<ProcessStatRecord>, DispatchError> {
    super::super::archive_queries::select_process_stats(start, end, order_by_cpu_desc)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_process_stats(
    start: &str,
    end: &str,
    order_by_cpu_desc: bool,
  ) -> Result<Vec<ProcessStatRecord>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::process_stats::select_process_stats(
          &database,
          super::super::native_database::NativeCancellation::new(),
          start.to_owned(),
          end.to_owned(),
          order_by_cpu_desc,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::archive_queries::select_process_stats(start, end, order_by_cpu_desc)
          .await
          .map_err(DispatchError::from)
      }
    }
  }
}

/// The `DATA_ARCHIVE` family: [`super::hardware_archive`] (SQLite writes),
/// [`super::archive_queries::select_data_archive_series`] (SQLite reads) and
/// [`super::native_database::data_archive`] (native).
pub mod data_archive {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert(
    row: HardwareArchiveRow,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    super::super::hardware_archive::insert(row, timestamp)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert(
    row: HardwareArchiveRow,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::data_archive::insert(
          &database,
          super::super::native_database::NativeCancellation::new(),
          row,
          timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::hardware_archive::insert(row, timestamp)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    super::super::hardware_archive::delete_old_data(retention_days)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::data_archive::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
        )
        .await
        .map(|_deleted| ())
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::hardware_archive::delete_old_data(retention_days)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_data_archive_series(
    column: DataArchiveColumn,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<Vec<ArchiveSeriesPoint>, DispatchError> {
    super::super::archive_queries::select_data_archive_series(
      column,
      start,
      end,
      bucket_width_ms,
      bucket_timestamp,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_data_archive_series(
    column: DataArchiveColumn,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<Vec<ArchiveSeriesPoint>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        let window = super::super::native_database::NativeSeriesWindow {
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        };
        super::super::native_database::data_archive::select_data_archive_series(
          &database,
          super::super::native_database::NativeCancellation::new(),
          column,
          window,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::archive_queries::select_data_archive_series(
          column,
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }
}

/// The `GPU_DATA_ARCHIVE` family: [`super::gpu_archive`] (SQLite writes),
/// the GPU reads in [`super::archive_queries`], and
/// [`super::native_database::gpu_archive`] (native).
pub mod gpu_archive {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert(
    data: GpuData,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    super::super::gpu_archive::insert(data, timestamp)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert(
    data: GpuData,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::gpu_archive::insert(
          &database,
          super::super::native_database::NativeCancellation::new(),
          data,
          timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::gpu_archive::insert(data, timestamp)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    super::super::gpu_archive::delete_old_data(retention_days)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::gpu_archive::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
        )
        .await
        .map(|_deleted| ())
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::gpu_archive::delete_old_data(retention_days)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_gpu_archive_series(
    column: GpuArchiveColumn,
    gpu_name: &str,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<Vec<ArchiveSeriesPoint>, DispatchError> {
    super::super::archive_queries::select_gpu_archive_series(
      column,
      gpu_name,
      start,
      end,
      bucket_width_ms,
      bucket_timestamp,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_gpu_archive_series(
    column: GpuArchiveColumn,
    gpu_name: &str,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<Vec<ArchiveSeriesPoint>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        let window = super::super::native_database::NativeSeriesWindow {
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        };
        super::super::native_database::gpu_archive::select_gpu_archive_series(
          &database,
          super::super::native_database::NativeCancellation::new(),
          column,
          gpu_name,
          window,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::archive_queries::select_gpu_archive_series(
          column,
          gpu_name,
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_gpu_names() -> Result<Vec<String>, DispatchError> {
    super::super::archive_queries::select_gpu_names()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_gpu_names() -> Result<Vec<String>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::gpu_archive::select_gpu_names(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::archive_queries::select_gpu_names()
          .await
          .map_err(DispatchError::from)
      }
    }
  }
}

/// The `AMBIENT_ARCHIVE` family: [`super::ambient_archive`] (SQLite writes),
/// the ambient read in [`super::archive_queries`], and
/// [`super::native_database::ambient_archive`] (native).
pub mod ambient_archive {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert(
    rows: Vec<AmbientData>,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    super::super::ambient_archive::insert(rows, timestamp)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert(
    rows: Vec<AmbientData>,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::ambient_archive::insert(
          &database,
          super::super::native_database::NativeCancellation::new(),
          rows,
          timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::ambient_archive::insert(rows, timestamp)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    super::super::ambient_archive::delete_old_data(retention_days)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::ambient_archive::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
        )
        .await
        .map(|_deleted| ())
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::ambient_archive::delete_old_data(retention_days)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_ambient_archive_series(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<AmbientArchiveSeries, DispatchError> {
    super::super::archive_queries::select_ambient_archive_series(
      start,
      end,
      bucket_width_ms,
      bucket_timestamp,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_ambient_archive_series(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<AmbientArchiveSeries, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::ambient_archive::select_ambient_archive_series(
          &database,
          super::super::native_database::NativeCancellation::new(),
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::archive_queries::select_ambient_archive_series(
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }
}

/// The `FAN_ARCHIVE` family: [`super::fan_archive`] (SQLite writes and
/// retention), the fan read in [`super::archive_queries`], and
/// [`super::native_database::fan_archive`] (native).
pub mod fan_archive {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert(
    rows: Vec<FanArchiveRow>,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    super::super::fan_archive::insert(rows, timestamp)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert(
    rows: Vec<FanArchiveRow>,
    timestamp: DateTime<Utc>,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::fan_archive::insert(
          &database,
          super::super::native_database::NativeCancellation::new(),
          rows,
          timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::fan_archive::insert(rows, timestamp)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    super::super::fan_archive::delete_old_data(retention_days)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::fan_archive::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
        )
        .await
        .map(|_deleted| ())
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::fan_archive::delete_old_data(retention_days)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_fan_archive_series(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<Vec<FanArchiveSeries>, DispatchError> {
    super::super::archive_queries::select_fan_archive_series(
      start,
      end,
      bucket_width_ms,
      bucket_timestamp,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_fan_archive_series(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    bucket_width_ms: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<Vec<FanArchiveSeries>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::fan_archive::select_fan_archive_series(
          &database,
          super::super::native_database::NativeCancellation::new(),
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::archive_queries::select_fan_archive_series(
          start,
          end,
          bucket_width_ms,
          bucket_timestamp,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }

  /// The daily rollup's own per-day range read (#2022).
  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_fan_minutes_for_range(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
  ) -> Result<Vec<FanArchiveMinuteSample>, DispatchError> {
    super::super::fan_archive::select_fan_minutes_for_range(start, end)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_fan_minutes_for_range(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
  ) -> Result<Vec<FanArchiveMinuteSample>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::fan_archive::select_fan_minutes_for_range(
          &database,
          super::super::native_database::NativeCancellation::new(),
          start,
          end,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::fan_archive::select_fan_minutes_for_range(start, end)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn has_any_reading() -> Result<bool, DispatchError> {
    super::super::fan_archive::has_any_reading()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn has_any_reading() -> Result<bool, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::fan_archive::has_any_reading(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::fan_archive::has_any_reading()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_fan_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    super::super::fan_archive::max_fan_archive_timestamp_before(before)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_fan_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::fan_archive::max_fan_archive_timestamp_before(
          &database,
          super::super::native_database::NativeCancellation::new(),
          before,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::fan_archive::max_fan_archive_timestamp_before(before)
          .await
          .map_err(DispatchError::from)
      }
    }
  }
}

/// The `cooling_daily_summary` family: [`super::cooling_daily_summary`]
/// (SQLite) and [`super::native_database::cooling_daily_summary`] (native).
pub mod cooling_daily_summary {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_archive_minutes_for_range(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
  ) -> Result<Vec<ArchiveMinuteSample>, DispatchError> {
    super::super::cooling_daily_summary::select_archive_minutes_for_range(start, end)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_archive_minutes_for_range(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
  ) -> Result<Vec<ArchiveMinuteSample>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::select_archive_minutes_for_range(
          &database,
          super::super::native_database::NativeCancellation::new(),
          start,
          end,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_daily_summary::select_archive_minutes_for_range(
        start, end,
      )
      .await
      .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    super::super::cooling_daily_summary::max_summarized_date()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::max_summarized_date(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_daily_summary::max_summarized_date()
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_pairable_summarized_date() -> Result<Option<NaiveDate>, DispatchError>
  {
    super::super::cooling_daily_summary::max_pairable_summarized_date()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_pairable_summarized_date() -> Result<Option<NaiveDate>, DispatchError>
  {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::max_pairable_summarized_date(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_daily_summary::max_pairable_summarized_date()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_powered_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    super::super::cooling_daily_summary::max_powered_summarized_date()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_powered_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::max_powered_summarized_date(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_daily_summary::max_powered_summarized_date()
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn earliest_archived_timestamp()
  -> Result<Option<DateTime<Utc>>, DispatchError> {
    super::super::cooling_daily_summary::earliest_archived_timestamp()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn earliest_archived_timestamp()
  -> Result<Option<DateTime<Utc>>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::earliest_archived_timestamp(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_daily_summary::earliest_archived_timestamp()
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_powered_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    super::super::cooling_daily_summary::max_powered_archive_timestamp_before(before)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_powered_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::max_powered_archive_timestamp_before(
          &database,
          super::super::native_database::NativeCancellation::new(),
          before,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_daily_summary::max_powered_archive_timestamp_before(
        before,
      )
      .await
      .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_daily_idle_samples() -> Result<Vec<DailyIdleSample>, DispatchError>
  {
    super::super::cooling_daily_summary::select_daily_idle_samples()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_daily_idle_samples() -> Result<Vec<DailyIdleSample>, DispatchError>
  {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::select_daily_idle_samples(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_daily_summary::select_daily_idle_samples()
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_all_daily_cooling_summaries()
  -> Result<Vec<DailyCoolingSummary>, DispatchError> {
    super::super::cooling_daily_summary::select_all_daily_cooling_summaries()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_all_daily_cooling_summaries()
  -> Result<Vec<DailyCoolingSummary>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::select_all_daily_cooling_summaries(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_daily_summary::select_all_daily_cooling_summaries()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    super::super::cooling_daily_summary::delete_old_data(
      retention_days,
      preserved_windows,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_daily_summary::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
          preserved_windows,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_daily_summary::delete_old_data(
          retention_days,
          preserved_windows,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }
}

/// The `cooling_hourly_summary` family: [`super::cooling_hourly_summary`]
/// (SQLite) and [`super::native_database::cooling_hourly_summary`] (native).
pub mod cooling_hourly_summary {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_hours_in_date_range(
    start_date: NaiveDate,
    end_date: NaiveDate,
  ) -> Result<Vec<HourlyCoolingSummary>, DispatchError> {
    super::super::cooling_hourly_summary::select_hours_in_date_range(start_date, end_date)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_hours_in_date_range(
    start_date: NaiveDate,
    end_date: NaiveDate,
  ) -> Result<Vec<HourlyCoolingSummary>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_hourly_summary::select_hours_in_date_range(
          &database,
          super::super::native_database::NativeCancellation::new(),
          start_date,
          end_date,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_hourly_summary::select_hours_in_date_range(
          start_date, end_date,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    super::super::cooling_hourly_summary::max_summarized_date()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_hourly_summary::max_summarized_date(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_hourly_summary::max_summarized_date()
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    super::super::cooling_hourly_summary::delete_old_data(
      retention_days,
      preserved_windows,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_hourly_summary::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
          preserved_windows,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_hourly_summary::delete_old_data(
          retention_days,
          preserved_windows,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }
}

/// The `cooling_fan_daily_summary` family: [`super::cooling_fan_daily_summary`]
/// (SQLite) and [`super::native_database::cooling_fan_daily_summary`] (native).
pub mod cooling_fan_daily_summary {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    super::super::cooling_fan_daily_summary::max_summarized_date()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_fan_daily_summary::max_summarized_date(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_fan_daily_summary::max_summarized_date()
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_all_fan_daily_summaries()
  -> Result<Vec<FanDailySummary>, DispatchError> {
    super::super::cooling_fan_daily_summary::select_all_fan_daily_summaries()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_all_fan_daily_summaries()
  -> Result<Vec<FanDailySummary>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_fan_daily_summary::select_all_fan_daily_summaries(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_fan_daily_summary::select_all_fan_daily_summaries()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    super::super::cooling_fan_daily_summary::delete_old_data(retention_days)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_fan_daily_summary::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_fan_daily_summary::delete_old_data(retention_days)
          .await
          .map_err(DispatchError::from)
      }
    }
  }
}

/// The `cooling_thermal_delta_daily_summary` family:
/// [`super::cooling_thermal_delta_daily_summary`] (SQLite) and
/// [`super::native_database::cooling_thermal_delta_daily_summary`] (native).
pub mod cooling_thermal_delta_daily_summary {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_thermal_delta_minutes_for_range(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
  ) -> Result<Vec<ThermalDeltaMinuteSample>, DispatchError> {
    super::super::cooling_thermal_delta_daily_summary::select_thermal_delta_minutes_for_range(
      start, end,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_thermal_delta_minutes_for_range(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
  ) -> Result<Vec<ThermalDeltaMinuteSample>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_thermal_delta_daily_summary::select_thermal_delta_minutes_for_range(
          &database,
          super::super::native_database::NativeCancellation::new(),
          start,
          end,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_thermal_delta_daily_summary::select_thermal_delta_minutes_for_range(
        start, end,
      )
      .await
      .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    super::super::cooling_thermal_delta_daily_summary::max_summarized_date()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_thermal_delta_daily_summary::max_summarized_date(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_thermal_delta_daily_summary::max_summarized_date()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_pairable_ambient_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    super::super::cooling_thermal_delta_daily_summary::max_pairable_ambient_archive_timestamp_before(
      before,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_pairable_ambient_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_thermal_delta_daily_summary::max_pairable_ambient_archive_timestamp_before(
          &database,
          super::super::native_database::NativeCancellation::new(),
          before,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_thermal_delta_daily_summary::max_pairable_ambient_archive_timestamp_before(
        before,
      )
      .await
      .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_all_thermal_delta_daily_summaries()
  -> Result<Vec<ThermalDeltaDailySummary>, DispatchError> {
    super::super::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_all_thermal_delta_daily_summaries()
  -> Result<Vec<ThermalDeltaDailySummary>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    super::super::cooling_thermal_delta_daily_summary::delete_old_data(
      retention_days,
      preserved_windows,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => super::super::native_database::cooling_thermal_delta_daily_summary::delete_old_data(
        &database,
        super::super::native_database::NativeCancellation::new(),
        retention_days,
        preserved_windows,
      )
      .await
      .map_err(DispatchError::from),
      super::boundary::Backend::Sqlite => super::super::cooling_thermal_delta_daily_summary::delete_old_data(
        retention_days,
        preserved_windows,
      )
      .await
      .map_err(DispatchError::from),
    }
  }
}

/// The `cooling_covariate_daily_summary` family:
/// [`super::cooling_covariate_daily_summary`] (SQLite) and
/// [`super::native_database::cooling_covariate_daily_summary`] (native).
pub mod cooling_covariate_daily_summary {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    super::super::cooling_covariate_daily_summary::max_summarized_date()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_summarized_date() -> Result<Option<NaiveDate>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_covariate_daily_summary::max_summarized_date(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_covariate_daily_summary::max_summarized_date()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn max_classifiable_pairable_ambient_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    super::super::cooling_covariate_daily_summary::max_classifiable_pairable_ambient_archive_timestamp_before(
      before,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn max_classifiable_pairable_ambient_archive_timestamp_before(
    before: &DateTime<Utc>,
  ) -> Result<Option<DateTime<Utc>>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_covariate_daily_summary::max_classifiable_pairable_ambient_archive_timestamp_before(
          &database,
          super::super::native_database::NativeCancellation::new(),
          before,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_covariate_daily_summary::max_classifiable_pairable_ambient_archive_timestamp_before(
        before,
      )
      .await
      .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_all_covariate_daily_summaries()
  -> Result<Vec<CovariateDailySummary>, DispatchError> {
    super::super::cooling_covariate_daily_summary::select_all_covariate_daily_summaries()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_all_covariate_daily_summaries()
  -> Result<Vec<CovariateDailySummary>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_covariate_daily_summary::select_all_covariate_daily_summaries(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_covariate_daily_summary::select_all_covariate_daily_summaries()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_all_fan_covariate_daily_summaries()
  -> Result<Vec<FanCovariateDailySummary>, DispatchError> {
    super::super::cooling_covariate_daily_summary::select_all_fan_covariate_daily_summaries()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_all_fan_covariate_daily_summaries()
  -> Result<Vec<FanCovariateDailySummary>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_covariate_daily_summary::select_all_fan_covariate_daily_summaries(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::cooling_covariate_daily_summary::select_all_fan_covariate_daily_summaries()
        .await
        .map_err(DispatchError::from),
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    super::super::cooling_covariate_daily_summary::delete_old_data(
      retention_days,
      preserved_windows,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(
    retention_days: u32,
    preserved_windows: &[(NaiveDate, NaiveDate)],
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_covariate_daily_summary::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
          preserved_windows,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_covariate_daily_summary::delete_old_data(
          retention_days,
          preserved_windows,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }
}

/// The `cooling_baseline` single pinned row: [`super::cooling_baseline`]
/// (SQLite) and [`super::native_database::cooling_baseline`] (native).
pub mod cooling_baseline {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_established_baseline()
  -> Result<Option<EstablishedBaseline>, DispatchError> {
    super::super::cooling_baseline::select_established_baseline()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_established_baseline()
  -> Result<Option<EstablishedBaseline>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_baseline::select_established_baseline(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_baseline::select_established_baseline()
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert_established_baseline(
    baseline: &EstablishedBaseline,
  ) -> Result<(), DispatchError> {
    super::super::cooling_baseline::insert_established_baseline(baseline)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert_established_baseline(
    baseline: &EstablishedBaseline,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::cooling_baseline::insert_established_baseline(
          &database,
          super::super::native_database::NativeCancellation::new(),
          baseline,
          Utc::now(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::cooling_baseline::insert_established_baseline(baseline)
          .await
          .map_err(DispatchError::from)
      }
    }
  }
}

/// The `cooling_delta_baseline` single pinned row:
/// [`super::cooling_delta_baseline`] (SQLite) and
/// [`super::native_database::cooling_delta_baseline`] (native).
pub mod cooling_delta_baseline {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn select_established_delta_baseline()
  -> Result<Option<EstablishedDeltaBaseline>, DispatchError> {
    super::super::cooling_delta_baseline::select_established_delta_baseline()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn select_established_delta_baseline()
  -> Result<Option<EstablishedDeltaBaseline>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => super::super::native_database::cooling_delta_baseline::select_established_delta_baseline(
        &database,
        super::super::native_database::NativeCancellation::new(),
      )
      .await
      .map_err(DispatchError::from),
      super::boundary::Backend::Sqlite => super::super::cooling_delta_baseline::select_established_delta_baseline()
        .await
        .map_err(DispatchError::from),
    }
  }

  /// SQLite has no top-level, pool-free `insert_established_delta_baseline`
  /// (unlike [`super::cooling_baseline::insert_established_baseline`]): every
  /// existing caller already holds a pool it threads through
  /// `insert_established_delta_baseline_from_pool`. The not-selected branch
  /// here resolves Core's process-wide pool itself instead, so this function
  /// has the same "just call it" shape as every other dispatch function.
  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert_established_delta_baseline(
    baseline: &EstablishedDeltaBaseline,
  ) -> Result<(), DispatchError> {
    let pool = super::super::db::get_pool().await?;
    super::super::cooling_delta_baseline::insert_established_delta_baseline_from_pool(
      &pool,
      baseline,
      Utc::now(),
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert_established_delta_baseline(
    baseline: &EstablishedDeltaBaseline,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => super::super::native_database::cooling_delta_baseline::insert_established_delta_baseline(
        &database,
        super::super::native_database::NativeCancellation::new(),
        baseline,
        Utc::now(),
      )
      .await
      .map_err(DispatchError::from),
      super::boundary::Backend::Sqlite => {
        let pool = super::super::db::get_pool().await?;
        super::super::cooling_delta_baseline::insert_established_delta_baseline_from_pool(
          &pool,
          baseline,
          Utc::now(),
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }
}

/// One rolled-up day's six cooling projections, in front of
/// [`crate::persistence::cooling_rollup::persist_day_rollup_from_pool`]
/// (SQLite) and [`super::native_database::cooling_rollup::persist_day_rollup`]
/// (native).
///
/// Keeps the same transaction boundary on both paths: the native branch
/// builds one [`super::native_database::DayRollup`] and writes it through
/// one native transaction, exactly as the SQLite branch writes all six
/// tables inside one `sqlx::Transaction`. Neither path is ever asked to
/// write the six projections one at a time.
pub mod cooling_rollup {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn persist_day_rollup(
    summary: Option<&DailyCoolingSummary>,
    hours: &[HourlyCoolingSummary],
    fans: &[FanDailySummary],
    thermal_deltas: &[ThermalDeltaDailySummary],
    covariates: &CovariateDaySummary,
  ) -> Result<(), DispatchError> {
    let pool = super::super::db::get_pool().await?;
    crate::persistence::cooling_rollup::persist_day_rollup_from_pool(
      &pool,
      summary,
      hours,
      fans,
      thermal_deltas,
      covariates,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn persist_day_rollup(
    summary: Option<&DailyCoolingSummary>,
    hours: &[HourlyCoolingSummary],
    fans: &[FanDailySummary],
    thermal_deltas: &[ThermalDeltaDailySummary],
    covariates: &CovariateDaySummary,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        let rollup = super::super::native_database::DayRollup {
          summary: summary.cloned(),
          hours: hours.to_vec(),
          fans: fans.to_vec(),
          thermal_deltas: thermal_deltas.to_vec(),
          covariates: covariates.clone(),
        };
        super::super::native_database::cooling_rollup::persist_day_rollup(
          &database,
          super::super::native_database::NativeCancellation::new(),
          rollup,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        let pool = super::super::db::get_pool().await?;
        crate::persistence::cooling_rollup::persist_day_rollup_from_pool(
          &pool,
          summary,
          hours,
          fans,
          thermal_deltas,
          covariates,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }
}

/// The Storage Health family: [`super::storage_health`] (SQLite) and
/// [`super::native_database::storage_health`] (native).
pub mod storage_health {
  use super::*;

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn insert_daily_records(
    devices: Vec<StorageDeviceRecord>,
    records: Vec<StorageHealthRecordDraft>,
  ) -> Result<(), DispatchError> {
    super::super::storage_health::insert_daily_records(devices, records)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn insert_daily_records(
    devices: Vec<StorageDeviceRecord>,
    records: Vec<StorageHealthRecordDraft>,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::storage_health::insert_daily_records(
          &database,
          super::super::native_database::NativeCancellation::new(),
          devices,
          records,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::storage_health::insert_daily_records(devices, records)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn refresh_daily_records(
    active_device_ids: &[String],
    devices: Vec<StorageDeviceRecord>,
    records: Vec<StorageHealthRecordDraft>,
  ) -> Result<(), DispatchError> {
    super::super::storage_health::refresh_daily_records(
      active_device_ids,
      devices,
      records,
    )
    .await
    .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn refresh_daily_records(
    active_device_ids: &[String],
    devices: Vec<StorageDeviceRecord>,
    records: Vec<StorageHealthRecordDraft>,
  ) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::storage_health::refresh_daily_records(
          &database,
          super::super::native_database::NativeCancellation::new(),
          active_device_ids.to_vec(),
          devices,
          records,
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::storage_health::refresh_daily_records(
          active_device_ids,
          devices,
          records,
        )
        .await
        .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    super::super::storage_health::delete_old_data(retention_days)
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn delete_old_data(retention_days: u32) -> Result<(), DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::storage_health::delete_old_data(
          &database,
          super::super::native_database::NativeCancellation::new(),
          retention_days,
        )
        .await
        .map(|_deleted| ())
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => {
        super::super::storage_health::delete_old_data(retention_days)
          .await
          .map_err(DispatchError::from)
      }
    }
  }

  #[cfg(not(feature = "duckdb-archive"))]
  pub async fn latest_records() -> Result<Vec<StorageHealthRecord>, DispatchError> {
    super::super::storage_health::latest_records()
      .await
      .map_err(DispatchError::from)
  }

  #[cfg(feature = "duckdb-archive")]
  pub async fn latest_records() -> Result<Vec<StorageHealthRecord>, DispatchError> {
    match super::boundary::resolve_backend().await? {
      super::boundary::Backend::Native(database) => {
        super::super::native_database::storage_health::latest_records(
          &database,
          super::super::native_database::NativeCancellation::new(),
        )
        .await
        .map_err(DispatchError::from)
      }
      super::boundary::Backend::Sqlite => super::super::storage_health::latest_records()
        .await
        .map_err(DispatchError::from),
    }
  }
}
