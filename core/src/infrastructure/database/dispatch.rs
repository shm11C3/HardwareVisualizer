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
//! - [`shutdown()`](shutdown) - close the live native owner without
//!   deciding a new backend. Idempotent; call it during process shutdown,
//!   after database-backed workers have drained and before the process
//!   exits, so the two DuckDB lane threads are joined deliberately instead
//!   of torn down with the process.
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
}

#[cfg(feature = "duckdb-archive")]
mod boundary {
  use std::sync::OnceLock;

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
  /// exists only so a consumer can be refused with a typed error rather
  /// than silently reading stale SQLite rows - see the module doc's "What
  /// decides the answer" and [`DispatchError::NativeUnavailable`].
  enum Active {
    Sqlite,
    Native(NativeDatabase),
    Unavailable(String),
  }

  /// What a dispatch function actually dispatches to. Not `Unavailable`:
  /// that case is resolved into an `Err` by [`resolve_backend`] before any
  /// caller sees it.
  pub(super) enum Backend {
    Sqlite,
    Native(NativeDatabase),
  }

  static CONFIG: OnceLock<Config> = OnceLock::new();
  static ACTIVE: OnceLock<RwLock<Active>> = OnceLock::new();

  /// The ordinary state before [`reobserve_authority`] has ever run - a
  /// fresh install, or a consumer called before App startup wiring reaches
  /// it (every current test does this deliberately, to prove the
  /// not-selected path needs no configuration).
  fn active() -> &'static RwLock<Active> {
    ACTIVE.get_or_init(|| RwLock::new(Active::Sqlite))
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
  /// #2135" for when to call this.
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
  pub async fn reobserve_authority() -> Result<AuthorityState, NativeDatabaseError> {
    let mut guard = active().write().await;
    if let Active::Native(database) = std::mem::replace(&mut *guard, Active::Sqlite) {
      close_or_refuse(&mut guard, database).await?;
    }
    let config = config();
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
            return Err(error);
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

  /// Close the live native owner, if any, without deciding a new backend.
  /// Leaves the boundary on SQLite - correct at process shutdown, where
  /// nothing answers a dispatch call again before the process exits.
  pub async fn shutdown() -> Result<(), NativeDatabaseError> {
    let mut guard = active().write().await;
    if let Active::Native(database) = std::mem::replace(&mut *guard, Active::Sqlite) {
      close_or_refuse(&mut guard, database).await?;
    }
    Ok(())
  }

  /// Refuse every consumer until a later [`reobserve_authority`] decides
  /// again, closing a live native owner first.
  ///
  /// For a caller that knows more than the files can show right now: a
  /// selection may have committed even though a fresh read cannot prove it,
  /// so the refusal must not depend on what [`reobserve_authority`] would
  /// infer from the files at that moment. Refusal holds on every path,
  /// including a failed close ([`close_or_refuse`]).
  pub async fn refuse_consumers(reason: String) -> Result<(), NativeDatabaseError> {
    let mut guard = active().write().await;
    if let Active::Native(database) =
      std::mem::replace(&mut *guard, Active::Unavailable(reason))
    {
      close_or_refuse(&mut guard, database).await?;
    }
    Ok(())
  }

  /// Close a native owner that was just taken out of the boundary.
  ///
  /// The caller has already put [`Active::Sqlite`] in its place, which is only
  /// true once the owner is really gone. If the close fails, the durable state
  /// may still say native is selected, so the boundary is left
  /// [`Active::Unavailable`] rather than answering from a stale SQLite copy.
  async fn close_or_refuse(
    guard: &mut Active,
    database: NativeDatabase,
  ) -> Result<(), NativeDatabaseError> {
    if let Err(error) = database.close().await {
      *guard =
        Active::Unavailable(format!("closing the previous native owner failed: {error}"));
      return Err(error);
    }
    Ok(())
  }

  /// The backend to dispatch to, or the typed refusal if the durable state
  /// says native is selected (or ambiguous) and this boundary cannot safely
  /// serve either engine.
  pub(super) async fn resolve_backend() -> Result<Backend, DispatchError> {
    match &*active().read().await {
      Active::Sqlite => Ok(Backend::Sqlite),
      // Cloning is an `Arc` bump, not a new connection.
      Active::Native(database) => Ok(Backend::Native(database.clone())),
      Active::Unavailable(reason) => Err(DispatchError::NativeUnavailable {
        reason: reason.clone(),
      }),
    }
  }
}

#[cfg(feature = "duckdb-archive")]
pub use boundary::{init, refuse_consumers, reobserve_authority, shutdown};

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
