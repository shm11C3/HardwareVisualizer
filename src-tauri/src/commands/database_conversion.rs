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

/// After the user asks for a recovery check, validate SQLite and verify under
/// the DuckDB writer lock that the native file is finalized but unselected.
/// Move it to a retained backup and start conversion only when those checks
/// pass; otherwise leave the database files in place.
#[command]
#[specta::specta]
pub async fn rebuild_native_database_from_sqlite(
  app: tauri::AppHandle,
) -> Result<(), String> {
  imp::rebuild_native_database_from_sqlite(app).await
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
    ClaimedStart, ConversionOutcome, ConversionRuntime, ConversionTarget,
    SelectionHandoff, build_producer_resumers, run_conversion,
  };
  use crate::app::native_lifecycle::{
    ConversionProgress, DatabaseLifecycleState, LifecycleIssue, NativeLifecycleOwner,
  };
  use crate::infrastructure::database::{native_paths, native_schema};
  use crate::models::database_conversion::DatabaseConversionState;
  use crate::workers::WorkersState;
  use crate::{log_error, log_info};
  use hardviz_core::infrastructure::database::candidate_database::verify_source_schema;
  use hardviz_core::infrastructure::database::native_database::{
    NativeDatabaseError, archive_unselected_native_for_rebuild, plan_conversion_space,
  };

  struct ConversionAttemptGuard(tauri::AppHandle);

  impl Drop for ConversionAttemptGuard {
    fn drop(&mut self) {
      self.0.state::<ConversionRuntime>().end_attempt();
    }
  }

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
    // start actually begins from. The claim also returns the state it
    // replaced, which the recovery restart decision after the attempt needs.
    let Some(ClaimedStart {
      cancellation,
      bus,
      previous_state,
    }) = conversion_runtime.begin_attempt_marking_converting(&owner_for_start_state)?
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

      let restart_after_recovery = should_restart_after_native_open_failure_retry(
        &previous_state,
        result.as_ref().ok(),
        &owner.state(),
      );

      // Looked up fresh here too, not carried across the spawn boundary -
      // see the comment above this task's construction.
      app_for_task.state::<ConversionRuntime>().end_attempt();

      if restart_after_recovery {
        log_info!(
          "restarting after native database recovery so startup services can initialize",
          "commands::database_conversion::start_database_conversion",
          None::<&str>
        );
        // `terminate_all` waits for every producer and the dispatch owner to
        // close before Tauri starts the replacement process. Even if native
        // close reports an error, process exit releases any remaining handle
        // and the next startup rechecks durable authority.
        if let Err(error) = workers.terminate_all().await {
          log_error!(
            "native database shutdown reported an error before recovery restart",
            "commands::database_conversion::start_database_conversion",
            Some(error)
          );
        }
        app_for_task.restart();
      }
    });

    Ok(())
  }

  pub(super) async fn rebuild_native_database_from_sqlite(
    app: tauri::AppHandle,
  ) -> Result<(), String> {
    let runtime_handle = tauri::async_runtime::handle().inner().clone();
    let conversion_runtime = app.state::<ConversionRuntime>();
    let Some((cancellation, bus)) = conversion_runtime.begin_attempt_with_bus()? else {
      return Ok(());
    };

    let owner = app.state::<NativeLifecycleOwner>();
    let previous_state = owner.state();
    if owner.selected_database().is_some()
      || matches!(previous_state, DatabaseLifecycleState::NativeAuthoritative)
    {
      conversion_runtime.end_attempt();
      return Err(
        "the selected native database cannot be rebuilt from SQLite".to_owned(),
      );
    }
    owner.set_state(DatabaseLifecycleState::Converting(
      ConversionProgress::Preflight,
    ));

    let paths = native_paths::authority_paths();
    let workspace = native_paths::database_directory();
    let schema_version = native_schema::NATIVE_SCHEMA_VERSION;
    let app_for_task = app.clone();
    runtime_handle.clone().spawn(async move {
      // The command RPC returns immediately. Keep preflight, the lock-held
      // archive, and conversion in this owned task so dropping the invoke
      // future cannot strand a moved file before `run_conversion` starts.
      let _attempt_guard = ConversionAttemptGuard(app_for_task.clone());
      let owner = app_for_task.state::<NativeLifecycleOwner>();
      let preflight = async {
        plan_conversion_space(&paths.source_database, &workspace, None, false)
          .map_err(|error| error.to_string())?;
        verify_source_schema(
          &paths.source_database,
          crate::infrastructure::database::migration::get_migrations(),
        )
        .await
        .map_err(|error| format!("SQLite source schema preflight failed: {error}"))
      }
      .await;
      if let Err(message) = preflight {
        owner.set_state(DatabaseLifecycleState::ActionRequired(
          LifecycleIssue::NativeRecoveryRefused {
            message: message.clone(),
          },
        ));
        log_error!(
          "native rebuild recovery stopped before moving the native database",
          "commands::database_conversion::rebuild_native_database_from_sqlite",
          Some(message)
        );
        return;
      }
      if cancellation.is_cancelled() {
        let message = "native database recovery was cancelled before backup".to_owned();
        owner.set_state(DatabaseLifecycleState::ActionRequired(
          LifecycleIssue::NativeRecoveryRefused {
            message: message.clone(),
          },
        ));
        log_info!(
          "native rebuild recovery was cancelled before moving the native database",
          "commands::database_conversion::rebuild_native_database_from_sqlite",
          Some(message)
        );
        return;
      }

      let backup_path = match archive_unselected_native_for_rebuild(&paths).await {
        Ok(path) => path,
        Err(error) => {
          let message = error.to_string();
          let issue = if matches!(
            &error,
            NativeDatabaseError::RecoveryBackupVerification { .. }
          ) {
            LifecycleIssue::NativeRecoveryFailed {
              message: message.clone(),
            }
          } else {
            LifecycleIssue::NativeRecoveryRefused {
              message: message.clone(),
            }
          };
          owner.set_state(DatabaseLifecycleState::ActionRequired(issue));
          log_error!(
            "native rebuild recovery could not archive the native database",
            "commands::database_conversion::rebuild_native_database_from_sqlite",
            Some(message)
          );
          return;
        }
      };

      log_info!(
        "archived an unselected native database before explicit SQLite rebuild",
        "commands::database_conversion::rebuild_native_database_from_sqlite",
        Some(backup_path.display().to_string())
      );
      let resumers = build_producer_resumers(&app_for_task, bus, runtime_handle.clone());
      let target = ConversionTarget {
        paths,
        workspace,
        expected_schema_version: schema_version,
      };
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
          "explicit SQLite rebuild after native backup finished",
          "commands::database_conversion::rebuild_native_database_from_sqlite",
          Some(format!("{outcome:?}"))
        ),
        Err(error) => log_error!(
          "explicit SQLite rebuild after native backup failed",
          "commands::database_conversion::rebuild_native_database_from_sqlite",
          Some(error.to_string())
        ),
      }
    });

    Ok(())
  }

  pub(super) async fn cancel_database_conversion(
    app: tauri::AppHandle,
  ) -> Result<(), String> {
    app.state::<ConversionRuntime>().cancel_current();
    Ok(())
  }

  fn should_restart_after_native_open_failure_retry(
    previous_state: &DatabaseLifecycleState,
    outcome: Option<&ConversionOutcome>,
    current_state: &DatabaseLifecycleState,
  ) -> bool {
    matches!(
      previous_state,
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed { .. })
    ) && matches!(
      outcome,
      Some(ConversionOutcome::AlreadySelected | ConversionOutcome::Selected { .. })
    ) && matches!(current_state, DatabaseLifecycleState::NativeAuthoritative)
  }

  // These tests cover only the pure restart decision, never
  // `start_database_conversion` itself: it resolves its `ConversionTarget`
  // from `native_paths::authority_paths()` / `database_directory()`, which
  // read the *real* OS app-data directory with no test seam to redirect them
  // (see `native_paths.rs`). Actually invoking this command in a test would
  // spawn a task that runs the real conversion driver against whatever
  // profile happens to exist on the machine running the tests. The
  // claim-and-mark ordering this command depends on is covered instead by
  // `ConversionRuntime::begin_attempt_marking_converting`'s own tests in
  // `app::native_conversion`, which use only a `NativeLifecycleOwner` and a
  // `ConversionRuntime` - no paths, no disk access - matching how this
  // module's sibling driver tests already avoid touching real app data.
  #[cfg(test)]
  mod tests {
    use super::*;

    #[test]
    fn restart_after_retry_only_when_native_open_failure_recovers() {
      let native_open_failed =
        DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed {
          message: "native database could not be opened".to_owned(),
        });
      let native_authoritative = DatabaseLifecycleState::NativeAuthoritative;
      let recovered = ConversionOutcome::AlreadySelected;

      assert!(should_restart_after_native_open_failure_retry(
        &native_open_failed,
        Some(&recovered),
        &native_authoritative,
      ));
      assert!(!should_restart_after_native_open_failure_retry(
        &native_open_failed,
        None,
        &native_open_failed,
      ));
      assert!(!should_restart_after_native_open_failure_retry(
        &DatabaseLifecycleState::SqliteAuthoritative,
        Some(&ConversionOutcome::Selected { total_rows: 1 }),
        &native_authoritative,
      ));
    }
  }
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

  pub(super) async fn rebuild_native_database_from_sqlite(
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
