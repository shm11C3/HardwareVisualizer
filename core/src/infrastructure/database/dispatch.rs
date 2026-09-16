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
//! native metadata opens a second DuckDB instance, and DuckDB refuses a file
//! this boundary's own live owner already holds. Instead the boundary's answer
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
//! Only the Process Stats family is routed as of #2134's first change. The
//! raw archive families (`DATA_ARCHIVE`, `GPU_DATA_ARCHIVE`, Ambient, Fan)
//! follow in a stacked change that reuses this same boundary and its
//! `native_handle`/`DispatchError` plumbing; Cooling, Storage Health and the
//! cooling rollup persistence are tracked separately (see the PR description).
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
//!   [`NativeDatabase`] and adopts it. Any other state leaves the boundary on
//!   SQLite.
//!
//! `reobserve_authority` was chosen over a "hand me an already-open database"
//! entry point because it is the one that cannot race a second instance
//! opening the file: it always closes its own current owner (if any) before it
//! looks at disk again, and it is the only thing that ever opens a
//! [`NativeDatabase`] on this boundary's behalf. A lifecycle owner that has
//! just called `select_native_database` (which opens and closes its own
//! scoped connection to record the selection, and is not this boundary's
//! owner) simply calls `reobserve_authority()` next; nothing else is needed.

use super::archive_queries::{ArchiveSeriesError, ProcessStatRecord};
use crate::persistence::archive_data::ProcessStatData;
use chrono::{DateTime, Utc};

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
}

#[cfg(feature = "duckdb-archive")]
mod boundary {
  use std::sync::OnceLock;

  use tokio::sync::RwLock;

  use super::super::native_database::{
    AuthorityPaths, AuthorityState, NativeDatabase, NativeDatabaseError,
    NativeDatabaseOptions, inspect_authority, observe_authority,
  };

  struct Config {
    paths: AuthorityPaths,
    expected_schema_version: u32,
  }

  static CONFIG: OnceLock<Config> = OnceLock::new();
  static ACTIVE: OnceLock<RwLock<Option<NativeDatabase>>> = OnceLock::new();

  fn active() -> &'static RwLock<Option<NativeDatabase>> {
    ACTIVE.get_or_init(|| RwLock::new(None))
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
  /// disk again and adopt a fresh one if - and only if - the durable state is
  /// [`AuthorityState::NativeSelected`]. See the module doc's "Seam for
  /// #2135" for when to call this.
  pub async fn reobserve_authority() -> Result<AuthorityState, NativeDatabaseError> {
    let mut guard = active().write().await;
    if let Some(database) = guard.take() {
      database.close().await?;
    }
    let config = config();
    let facts = observe_authority(&config.paths, config.expected_schema_version);
    let state = inspect_authority(&facts);
    if matches!(state, AuthorityState::NativeSelected) {
      let database = NativeDatabase::open(
        &config.paths.native_database,
        NativeDatabaseOptions::new(config.expected_schema_version),
      )
      .await?;
      *guard = Some(database);
    }
    Ok(state)
  }

  /// The live owner, if the durable state is currently selected. Cloning is
  /// an `Arc` bump, not a new connection.
  pub(super) async fn native_handle() -> Option<NativeDatabase> {
    active().read().await.clone()
  }
}

#[cfg(feature = "duckdb-archive")]
pub use boundary::{init, reobserve_authority};

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
    match super::boundary::native_handle().await {
      Some(database) => super::super::native_database::process_stats::insert(
        &database,
        super::super::native_database::NativeCancellation::new(),
        processes,
        timestamp,
      )
      .await
      .map_err(DispatchError::from),
      None => super::super::process_stats::insert(processes, timestamp)
        .await
        .map_err(DispatchError::from),
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
    match super::boundary::native_handle().await {
      Some(database) => super::super::native_database::process_stats::delete_old_data(
        &database,
        super::super::native_database::NativeCancellation::new(),
        retention_days,
      )
      .await
      .map(|_deleted| ())
      .map_err(DispatchError::from),
      None => super::super::process_stats::delete_old_data(retention_days)
        .await
        .map_err(DispatchError::from),
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
    match super::boundary::native_handle().await {
      Some(database) => {
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
      None => {
        super::super::archive_queries::select_process_stats(start, end, order_by_cpu_desc)
          .await
          .map_err(DispatchError::from)
      }
    }
  }
}
