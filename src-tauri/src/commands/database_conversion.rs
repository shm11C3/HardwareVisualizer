//! #2136: the explicit-user-intent commands that surface the native
//! database conversion lifecycle to the frontend and start/cancel it.
//!
//! Conversion never starts on its own - `lib::run` never calls
//! [`crate::app::native_conversion::run_conversion`], only these commands
//! do, and only in response to a frontend action. Progress and failures
//! read from [`crate::app::native_lifecycle::NativeLifecycleOwner`], the
//! App's single lifecycle state owner, rather than a second source of
//! truth.
//!
//! Registered unconditionally (both feature states compile and are part
//! of the stable command surface) so the frontend never has to special-case
//! a missing command; [`get_database_conversion_state`] reports
//! `NotSupported` when the `duckdb-archive` feature is off, and the
//! frontend hides the entry point on that value.

use tauri::command;

use crate::models::database_conversion::DatabaseConversionState;

#[command]
#[specta::specta]
pub async fn get_database_conversion_state(
  app: tauri::AppHandle,
) -> DatabaseConversionState {
  imp::get_database_conversion_state(app).await
}

/// Start a conversion attempt if one is not already running. Returns
/// immediately; the frontend polls [`get_database_conversion_state`] for
/// progress, the same state vocabulary a startup recovery reads.
#[command]
#[specta::specta]
pub async fn start_database_conversion(app: tauri::AppHandle) -> Result<(), String> {
  imp::start_database_conversion(app).await
}

/// Ask the currently running conversion to stop at the next step boundary.
/// A no-op (not an error) if nothing is running: cancellation racing
/// completion is expected, not exceptional.
#[command]
#[specta::specta]
pub async fn cancel_database_conversion(app: tauri::AppHandle) -> Result<(), String> {
  imp::cancel_database_conversion(app).await
}

#[cfg(feature = "duckdb-archive")]
mod imp {
  use tauri::Manager;

  use crate::app::native_conversion::{
    ConversionRuntime, ConversionTarget, SelectionHandoff, build_producer_resumers,
    run_conversion,
  };
  use crate::app::native_lifecycle::NativeLifecycleOwner;
  use crate::infrastructure::database::{native_paths, native_schema};
  use crate::models::database_conversion::DatabaseConversionState;
  use crate::workers::WorkersState;
  use crate::{log_error, log_info};

  pub(super) async fn get_database_conversion_state(
    app: tauri::AppHandle,
  ) -> DatabaseConversionState {
    app.state::<NativeLifecycleOwner>().state().into()
  }

  pub(super) async fn start_database_conversion(
    app: tauri::AppHandle,
  ) -> Result<(), String> {
    let runtime_handle = tauri::async_runtime::handle().inner().clone();
    let conversion_runtime = app.state::<ConversionRuntime>();
    let owner_for_start_state = app.state::<NativeLifecycleOwner>();

    // Claims the in-progress flag, resolves the bus, and marks `owner` as
    // `Converting` - all atomically with the claim - so a caller that polls
    // `get_database_conversion` right after this command resolves never
    // reads a pre-start state; see
    // `ConversionRuntime::begin_attempt_marking_converting`'s own
    // documentation for the race this closes, why it is safe to mark
    // `owner` on a successful claim, and why a claim is refused (and
    // `owner` left untouched) whenever `owner`'s current state is not one a
    // start actually begins from.
    let Some((cancellation, bus)) =
      conversion_runtime.begin_attempt_marking_converting(&owner_for_start_state)?
    else {
      // Nothing to do: either another attempt already claimed it (the
      // frontend already shows that attempt's progress), or `owner` was
      // already past the point a start begins from (e.g.
      // `NativeAuthoritative`, `ActionRequired`) - neither is an error.
      return Ok(());
    };

    let resumers = build_producer_resumers(&app, bus, runtime_handle.clone());
    let target = ConversionTarget {
      paths: native_paths::authority_paths(),
      workspace: native_paths::database_directory(),
      expected_schema_version: native_schema::NATIVE_SCHEMA_VERSION,
    };

    // Run on the shared Tokio runtime rather than awaiting inline: the
    // command must return promptly so the frontend can start polling
    // progress, and `run_conversion` legitimately runs for as long as the
    // preflight/candidate/finalize/reconcile steps take. `app` (an
    // `AppHandle`, cheap to clone and `'static`) moves into the task so
    // `NativeLifecycleOwner`/`WorkersState`/`ConversionRuntime` are looked
    // up fresh here rather than trying to smuggle a borrowed `State` across
    // the spawn boundary.
    let app_for_task = app.clone();
    runtime_handle.spawn(async move {
      let owner = app_for_task.state::<NativeLifecycleOwner>();
      let workers = app_for_task.state::<WorkersState>();

      let result = run_conversion(
        target,
        &owner,
        &workers,
        resumers,
        &cancellation,
        SelectionHandoff::ThroughDispatch,
      )
      .await;

      match &result {
        Ok(outcome) => log_info!(
          "explicit database conversion attempt finished",
          "commands::database_conversion::start_database_conversion",
          Some(format!("{outcome:?}"))
        ),
        Err(error) => log_error!(
          "explicit database conversion attempt failed",
          "commands::database_conversion::start_database_conversion",
          Some(error.to_string())
        ),
      }

      // Looked up fresh here too, not carried across the spawn boundary -
      // see the comment above this task's construction.
      app_for_task.state::<ConversionRuntime>().end_attempt();
    });

    Ok(())
  }

  pub(super) async fn cancel_database_conversion(
    app: tauri::AppHandle,
  ) -> Result<(), String> {
    app.state::<ConversionRuntime>().cancel_current();
    Ok(())
  }

  // No `#[cfg(test)] mod tests` here: `start_database_conversion` resolves
  // its `ConversionTarget` from `native_paths::authority_paths()` /
  // `database_directory()`, which read the *real* OS app-data directory
  // with no test seam to redirect them (see `native_paths.rs`). Actually
  // invoking this command in a test would spawn a task that runs the real
  // conversion driver against whatever profile happens to exist on the
  // machine running the tests. The claim-and-mark ordering this command
  // depends on is covered instead by
  // `ConversionRuntime::begin_attempt_marking_converting`'s own tests in
  // `app::native_conversion`, which use only a `NativeLifecycleOwner` and a
  // `ConversionRuntime` - no paths, no disk access - matching how this
  // module's sibling driver tests already avoid touching real app data.
}

#[cfg(not(feature = "duckdb-archive"))]
mod imp {
  use crate::models::database_conversion::DatabaseConversionState;

  pub(super) async fn get_database_conversion_state(
    _app: tauri::AppHandle,
  ) -> DatabaseConversionState {
    DatabaseConversionState::NotSupported
  }

  pub(super) async fn start_database_conversion(
    _app: tauri::AppHandle,
  ) -> Result<(), String> {
    Err("this build does not include the native database conversion feature".to_string())
  }

  pub(super) async fn cancel_database_conversion(
    _app: tauri::AppHandle,
  ) -> Result<(), String> {
    Err("this build does not include the native database conversion feature".to_string())
  }
}
