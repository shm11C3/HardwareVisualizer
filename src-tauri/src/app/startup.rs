use crate::utils;
use hardviz_core::persistence::preflight::DbStartupError;
use std::path::Path;
use tauri_plugin_dialog::{
  DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};

#[cfg(feature = "duckdb-archive")]
use crate::app::native_lifecycle::LifecycleIssue;

const RESET_LABEL: &str = "Reset and Restart";
const CONTINUE_LABEL: &str = "Continue Anyway";
const EXIT_LABEL: &str = "Exit";
#[cfg(feature = "duckdb-archive")]
const RETRY_LABEL: &str = "Retry";
#[cfg(feature = "duckdb-archive")]
const MORE_OPTIONS_LABEL: &str = "More Options";

/// The one Reset-consequence sentence both dialogs that offer it show.
///
/// Deliberately does not say "all" archived history: with `duckdb-archive`,
/// a `.retired` pre-conversion copy (`hv-database.db.retired`) - if one
/// exists - survives Reset on purpose (see
/// `app::native_maintenance::discard_native_authority_files`'s "What is
/// deliberately not removed"), so a blanket "all history" claim here would
/// be false for exactly the profiles that have one. Does not promise a
/// Settings-based way to remove that copy either - #2195 would add one, but
/// it does not exist yet.
const RESET_HISTORY_NOTE: &str = "* Resetting deletes the hardware monitoring history in the \
   current database. A pre-conversion recovery copy (hv-database.db.retired), if one exists, \
   is kept.";

/// User's chosen action from the DB startup error dialog.
pub enum StartupErrorAction {
  /// User chose to delete DB and restart.
  ResetAndRestart,
  /// User chose to continue without the database.
  ContinueAnyway,
  /// User chose to exit.
  Exit,
}

/// Show a dialog for a DB startup error and return the user's chosen action.
pub fn prompt_startup_error(
  handle: &tauri::AppHandle,
  error: DbStartupError,
) -> StartupErrorAction {
  let message = build_message(&error);

  let result = handle
    .dialog()
    .message(message)
    .title("Data Compatibility Issue")
    .kind(MessageDialogKind::Warning)
    .buttons(MessageDialogButtons::YesNoCancelCustom(
      RESET_LABEL.into(),
      CONTINUE_LABEL.into(),
      EXIT_LABEL.into(),
    ))
    .blocking_show_with_result();

  let is_reset = result == MessageDialogResult::Yes
    || result == MessageDialogResult::Custom(RESET_LABEL.into());
  let is_continue = result == MessageDialogResult::No
    || result == MessageDialogResult::Custom(CONTINUE_LABEL.into());

  if is_reset {
    StartupErrorAction::ResetAndRestart
  } else if is_continue {
    StartupErrorAction::ContinueAnyway
  } else {
    StartupErrorAction::Exit
  }
}

/// Delete every artifact this profile's database lifecycle owns, then
/// restart the app on a clean profile.
///
/// With `duckdb-archive` disabled this is unchanged from before: the SQLite
/// file and its WAL/SHM companions.
///
/// With `duckdb-archive` enabled it additionally, and in this order:
///
/// 1. Closes the dispatch boundary's live native owner
///    ([`close_native_owner_before_reset`]) - the native database may only
///    ever be open once per process
///    ([`hardviz_core::infrastructure::database::native_database::NativeDatabaseError::AlreadyOpen`]),
///    so the file cannot be removed while dispatch still holds it.
/// 2. Removes the native database, its write-ahead log, the authority
///    marker and any interrupted conversion work directories
///    ([`crate::app::native_maintenance::discard_native_authority_files`] -
///    see its own documentation for the crash-safe order and the deliberate
///    decision to leave a `.retired` SQLite copy in place).
/// 3. Removes the SQLite source and its WAL/SHM companions, same as always.
///
/// This is the one Reset implementation both startup dialogs use
/// ([`prompt_startup_error`]'s SQLite-compatibility dialog and
/// [`prompt_native_authority_issue`]'s native-authority dialog): a
/// SQLite-authoritative profile can still carry leftover native artifacts
/// from an earlier interrupted or reverted conversion, and leaving those
/// behind is exactly the bug (#2194) this function closes - a partial reset
/// that finds "the same problem" on the next startup.
pub fn reset_database_and_restart(handle: &tauri::AppHandle) {
  #[cfg(feature = "duckdb-archive")]
  if let Err(e) = close_native_owner_before_reset() {
    show_error_dialog(
      handle,
      &format!("Failed to close the native database before reset: {e}"),
    );
    handle.exit(1);
    return;
  }
  #[cfg(feature = "duckdb-archive")]
  {
    let paths = crate::infrastructure::database::native_paths::authority_paths();
    let workspace = crate::infrastructure::database::native_paths::database_directory();
    // Stop here, before the SQLite source is touched at all, if any native
    // artifact could not be removed: continuing on to delete the SQLite
    // source would leave a state `discard_native_authority_files`'s own
    // documentation warns about - e.g. a marker removed while the native
    // file it named survives (locked, permissions), which
    // `inspect_startup_authority` treats as the one repairable gap and
    // silently rewrites on the next boot, undoing the reset the user just
    // asked for.
    if let Err(e) =
      crate::app::native_maintenance::discard_native_authority_files(&paths, &workspace)
    {
      show_error_dialog(
        handle,
        &format!("Failed to remove native database files: {e}"),
      );
      handle.exit(1);
      return;
    }
  }

  let db_path = utils::file::get_app_data_dir("hv-database.db");
  if let Err(e) = delete_database_files(&db_path) {
    show_error_dialog(handle, &format!("Failed to delete database file: {e}"));
    handle.exit(1);
    return;
  }

  restart(handle);
}

/// Start a fresh copy of this process and exit this one. Used from a startup
/// dialog, before any database-backed worker has started.
pub fn restart(handle: &tauri::AppHandle) {
  let exe_path = match std::env::current_exe() {
    Ok(path) => path,
    Err(e) => {
      show_error_dialog(handle, &format!("Failed to obtain executable path: {e}"));
      handle.exit(1);
      return;
    }
  };
  // The restarted process waits for this one to exit (`cli::restart_args`):
  // this process still holds the single-instance lock, so a child that ran
  // ahead would exit as a second instance and leave no app.
  #[allow(clippy::zombie_processes)]
  if let Err(e) = std::process::Command::new(exe_path)
    .args(crate::cli::restart_args())
    .spawn()
  {
    show_error_dialog(handle, &format!("Failed to restart process: {e}"));
    handle.exit(1);
    return;
  }
  handle.exit(0);
}

/// Close the dispatch boundary's live native owner (if any) so its file can
/// be removed. Idempotent and safe to call from a state where dispatch never
/// opened a native database at all -
/// [`hardviz_core::infrastructure::database::dispatch::shutdown`] is itself
/// a no-op in that case. Runs on a short-lived current-thread runtime, the
/// same pattern [`crate::resolve_native_authority`] uses to call into
/// dispatch outside of Tauri's own async runtime.
#[cfg(feature = "duckdb-archive")]
fn close_native_owner_before_reset() -> Result<
  (),
  hardviz_core::infrastructure::database::native_database::NativeDatabaseError,
> {
  use hardviz_core::infrastructure::database::dispatch;
  use hardviz_core::infrastructure::database::native_database::NativeDatabaseError;

  let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .map_err(|error| NativeDatabaseError::Worker {
      message: error.to_string(),
    })?;
  runtime.block_on(dispatch::shutdown())
}

/// Delete the database file and its WAL/SHM companions.
///
/// Returns `Ok(())` if the main DB file was successfully deleted (or didn't exist).
/// WAL/SHM deletion errors are silently ignored since they are optional files.
pub(crate) fn delete_database_files(db_path: &Path) -> std::io::Result<()> {
  if db_path.exists() {
    std::fs::remove_file(db_path)?;
  }
  let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
  let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
  Ok(())
}

/// User's chosen action from the native authority issue dialog.
#[cfg(feature = "duckdb-archive")]
#[derive(Debug, PartialEq, Eq)]
pub enum NativeAuthorityAction {
  /// Restart and inspect the same files again, changing nothing. Offered
  /// first when the issue may clear on its own ([`offers_retry_first`]).
  Retry,
  /// Delete every database artifact (SQLite and native) and restart on a
  /// clean profile. The only way forward when continuing means running with
  /// database-backed features disabled indefinitely and the user wants a
  /// working history-tracking install instead.
  ResetAndRestart,
  /// Continue with SQLite; DB-dependent features behave as they did before
  /// any conversion was attempted.
  ContinueAnyway,
  Exit,
}

/// Show a dialog for a startup authority inconsistency
/// ([`crate::app::native_lifecycle::DatabaseLifecycleState::ActionRequired`])
/// and return the user's chosen action.
///
/// Reset is offered here for the same reason it is offered from
/// [`prompt_startup_error`]: [`reset_database_and_restart`] performs one
/// full, state-independent discard of every database artifact (SQLite and
/// native - see its own documentation), so it never has to guess which of
/// two disagreeing files is correct the way a narrower "keep one, discard
/// the other" repair would. The only files `inspect_authority` still refuses
/// to guess about are the ones Reset removes entirely.
///
/// When the files could not be read rather than shown to disagree
/// ([`offers_retry_first`]), a first dialog offers only Retry, More Options
/// and Exit. Reset is reached only through More Options, so the destructive
/// path is never the first or default choice for a lock or a missing
/// temporary directory that clears on its own (#2268).
#[cfg(feature = "duckdb-archive")]
pub fn prompt_native_authority_issue(
  handle: &tauri::AppHandle,
  issue: &LifecycleIssue,
) -> NativeAuthorityAction {
  if offers_retry_first(issue) {
    let result = handle
      .dialog()
      .message(build_native_retry_message(issue))
      .title("Data Temporarily Unavailable")
      .kind(MessageDialogKind::Warning)
      .buttons(MessageDialogButtons::YesNoCancelCustom(
        RETRY_LABEL.into(),
        MORE_OPTIONS_LABEL.into(),
        EXIT_LABEL.into(),
      ))
      .blocking_show_with_result();
    if let Some(action) = retry_first_action(&result) {
      return action;
    }
  }
  let result = handle
    .dialog()
    .message(build_native_authority_message(issue))
    .title("Data Compatibility Issue")
    .kind(MessageDialogKind::Warning)
    .buttons(MessageDialogButtons::YesNoCancelCustom(
      RESET_LABEL.into(),
      CONTINUE_LABEL.into(),
      EXIT_LABEL.into(),
    ))
    .blocking_show_with_result();

  if result == MessageDialogResult::Yes
    || result == MessageDialogResult::Custom(RESET_LABEL.into())
  {
    NativeAuthorityAction::ResetAndRestart
  } else if result == MessageDialogResult::No
    || result == MessageDialogResult::Custom(CONTINUE_LABEL.into())
  {
    NativeAuthorityAction::ContinueAnyway
  } else {
    NativeAuthorityAction::Exit
  }
}

/// Whether startup offers Retry before Reset for `issue`: the native database
/// could not be opened this time, which a lock or a missing temporary
/// directory explains, rather than the files showing a disagreement. Core owns
/// which authority reasons may clear
/// ([`AuthorityInconsistency::may_clear_on_retry`]); `NativeOpenFailed` is the
/// same failure to open a database already selected, which Reset would delete.
///
/// [`AuthorityInconsistency::may_clear_on_retry`]: hardviz_core::infrastructure::database::native_database::AuthorityInconsistency::may_clear_on_retry
#[cfg(feature = "duckdb-archive")]
fn offers_retry_first(issue: &LifecycleIssue) -> bool {
  match issue {
    LifecycleIssue::Authority(reason)
    | LifecycleIssue::NativeRebuildInspectionRequired { reason } => {
      reason.may_clear_on_retry()
    }
    LifecycleIssue::NativeOpenFailed { .. } => true,
    _ => false,
  }
}

/// The retry-first dialog's result: an action, or `None` for More Options,
/// which shows the full dialog. Closing the dialog exits; no result resets.
#[cfg(feature = "duckdb-archive")]
fn retry_first_action(result: &MessageDialogResult) -> Option<NativeAuthorityAction> {
  if *result == MessageDialogResult::Yes
    || *result == MessageDialogResult::Custom(RETRY_LABEL.into())
  {
    Some(NativeAuthorityAction::Retry)
  } else if *result == MessageDialogResult::No
    || *result == MessageDialogResult::Custom(MORE_OPTIONS_LABEL.into())
  {
    None
  } else {
    Some(NativeAuthorityAction::Exit)
  }
}

#[cfg(feature = "duckdb-archive")]
fn build_native_retry_message(issue: &LifecycleIssue) -> String {
  format!(
    "HardwareVisualizer could not open the native database to check its state. This is \
     often temporary: another app may be using the file, or temporary storage may be \
     unavailable.\n\n\
     Close any app that may be using the database and choose Retry. Nothing has been \
     deleted.\n\n\
     More Options lets you continue with real-time monitoring only, or reset the data if \
     retrying does not help.\n\n\
     [Details: {issue:?}]"
  )
}

#[cfg(feature = "duckdb-archive")]
fn build_native_authority_message(issue: &LifecycleIssue) -> String {
  let recovery_hint = if matches!(
    issue,
    LifecycleIssue::NativeRebuildInspectionRequired { .. }
  ) {
    "After continuing, open Settings > Insights to check whether a backup-and-rebuild from SQLite is safe. The check refuses to move files unless it proves under the native writer lock that the native database is unselected.\n\n"
  } else {
    ""
  };
  format!(
    "HardwareVisualizer found the native database files in an unexpected state and \
     stopped rather than guess which one is correct.\n\n\
     You can continue with real-time monitoring only - archived history and other \
     database-backed features stay disabled for this session - reset the data to \
     start fresh, or exit and inspect the app data directory.\n\n\
     {recovery_hint}\
     {RESET_HISTORY_NOTE}\n\n\
     [Details: {issue:?}]"
  )
}

fn show_error_dialog(handle: &tauri::AppHandle, message: &str) {
  handle
    .dialog()
    .message(message.to_string())
    .title("Error")
    .kind(MessageDialogKind::Error)
    .buttons(MessageDialogButtons::Ok)
    .blocking_show();
}

fn build_message(error: &DbStartupError) -> String {
  match error {
    DbStartupError::IncompatibleVersion {
      db_max_version,
      app_max_version,
    } => {
      format!(
        "This version of HardwareVisualizer is not compatible with the existing data.\n\
         This usually happens when reverting to an older version of the app.\n\n\
         You can reset the data to continue using this version, \
         or update to the latest version to keep your data.\n\n\
         {RESET_HISTORY_NOTE}\n\n\
         [Details: data schema v{db_max_version}, app supports up to v{app_max_version}]"
      )
    }
    DbStartupError::Other(msg) => {
      format!(
        "HardwareVisualizer could not read the existing data.\n\n\
         You can reset the data to continue using the app.\n\n\
         {RESET_HISTORY_NOTE}\n\n\
         [Details: {msg}]"
      )
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  #[test]
  fn build_message_incompatible_version_contains_versions() {
    let msg = build_message(&DbStartupError::IncompatibleVersion {
      db_max_version: 6,
      app_max_version: 5,
    });
    assert!(msg.contains("v6"));
    assert!(msg.contains("v5"));
    assert!(msg.contains("not compatible"));
    assert!(msg.contains("older version"));
  }

  #[test]
  fn build_message_other_contains_detail() {
    let msg = build_message(&DbStartupError::Other("disk I/O error".into()));
    assert!(msg.contains("disk I/O error"));
    assert!(msg.contains("could not read"));
  }

  /// The Reset consequence wording must stay honest about the one thing
  /// Reset deliberately keeps: a `.retired` pre-conversion copy - see
  /// `app::native_maintenance::discard_native_authority_files`'s "What is
  /// deliberately not removed". A blanket "all history" claim would be false
  /// for exactly the profiles that have one.
  #[test]
  fn reset_history_note_does_not_overclaim_and_names_the_retired_copy() {
    assert!(!RESET_HISTORY_NOTE.to_lowercase().contains("all"));
    assert!(RESET_HISTORY_NOTE.contains("hv-database.db.retired"));
    assert!(RESET_HISTORY_NOTE.contains("kept"));
    // #2195 (a Settings-based way to remove the retired copy) does not
    // exist yet - the dialog must not promise it.
    assert!(!RESET_HISTORY_NOTE.to_lowercase().contains("settings"));
  }

  #[test]
  fn build_message_variants_include_the_reset_history_note() {
    let incompatible = build_message(&DbStartupError::IncompatibleVersion {
      db_max_version: 6,
      app_max_version: 5,
    });
    let other = build_message(&DbStartupError::Other("disk I/O error".into()));
    assert!(incompatible.contains(RESET_HISTORY_NOTE));
    assert!(other.contains(RESET_HISTORY_NOTE));
  }

  #[cfg(feature = "duckdb-archive")]
  #[test]
  fn native_authority_message_includes_the_reset_history_note_and_the_issue() {
    let msg = build_native_authority_message(&LifecycleIssue::NativeOpenFailed {
      message: "could not open the spill directory".to_owned(),
    });
    assert!(msg.contains(RESET_HISTORY_NOTE));
    assert!(msg.contains("could not open the spill directory"));
    assert!(!msg.contains("\n\n\n"), "{msg:?}");
  }

  /// #2268: a native database that could not be opened offers Retry first;
  /// a disagreement the files positively show keeps the full dialog.
  #[cfg(feature = "duckdb-archive")]
  #[test]
  fn only_an_issue_that_may_clear_offers_retry_first() {
    use hardviz_core::infrastructure::database::native_database::AuthorityInconsistency as A;

    for issue in [
      LifecycleIssue::Authority(A::NativeMetadataUnreadable),
      LifecycleIssue::NativeRebuildInspectionRequired {
        reason: A::NativeMetadataUnreadable,
      },
      LifecycleIssue::NativeOpenFailed {
        message: "could not open the spill directory".to_owned(),
      },
    ] {
      assert!(offers_retry_first(&issue), "{issue:?}");
    }
    for issue in [
      LifecycleIssue::Authority(A::NativeMetadataInvalid),
      LifecycleIssue::Authority(A::MarkerDisagreesWithNativeDatabase),
      LifecycleIssue::NativeRebuildInspectionRequired {
        reason: A::NativeMetadataInvalid,
      },
      LifecycleIssue::FreshCreationFailed {
        message: "disk full".to_owned(),
      },
    ] {
      assert!(!offers_retry_first(&issue), "{issue:?}");
    }
  }

  /// #2268: the retry-first dialog never resets. Its first button retries,
  /// its second opens the full dialog, and closing it exits.
  #[cfg(feature = "duckdb-archive")]
  #[test]
  fn the_retry_first_dialog_never_maps_to_reset() {
    assert_eq!(
      retry_first_action(&MessageDialogResult::Yes),
      Some(NativeAuthorityAction::Retry)
    );
    assert_eq!(
      retry_first_action(&MessageDialogResult::Custom(RETRY_LABEL.into())),
      Some(NativeAuthorityAction::Retry)
    );
    assert_eq!(retry_first_action(&MessageDialogResult::No), None);
    assert_eq!(
      retry_first_action(&MessageDialogResult::Custom(MORE_OPTIONS_LABEL.into())),
      None
    );
    for result in [
      MessageDialogResult::Cancel,
      MessageDialogResult::Ok,
      MessageDialogResult::Custom(EXIT_LABEL.into()),
      MessageDialogResult::Custom(RESET_LABEL.into()),
    ] {
      assert_eq!(
        retry_first_action(&result),
        Some(NativeAuthorityAction::Exit),
        "{result:?}"
      );
    }
  }

  #[cfg(feature = "duckdb-archive")]
  #[test]
  fn native_retry_message_names_retry_and_keeps_reset_secondary() {
    let msg = build_native_retry_message(&LifecycleIssue::Authority(
      hardviz_core::infrastructure::database::native_database::AuthorityInconsistency::NativeMetadataUnreadable,
    ));
    assert!(msg.contains("Retry"));
    assert!(msg.contains("More Options"));
    assert!(msg.contains("Nothing has been deleted"));
    assert!(msg.contains("NativeMetadataUnreadable"));
    assert!(!msg.contains(RESET_HISTORY_NOTE));
    assert!(!msg.contains("\n\n\n"), "{msg:?}");
  }

  #[cfg(feature = "duckdb-archive")]
  #[test]
  fn native_rebuild_message_describes_conditional_inspection() {
    let msg = build_native_authority_message(
      &LifecycleIssue::NativeRebuildInspectionRequired {
        reason: hardviz_core::infrastructure::database::native_database::AuthorityInconsistency::NativeMetadataUnreadable,
      },
    );
    assert!(msg.contains("Settings > Insights"));
    assert!(msg.contains("refuses to move files"));
    assert!(msg.contains("unselected"));
    assert!(!msg.contains("\n\n\n"), "{msg:?}");
  }

  #[test]
  fn delete_database_files_removes_all_files() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let wal_path = dir.path().join("test.db-wal");
    let shm_path = dir.path().join("test.db-shm");

    fs::write(&db_path, b"db").unwrap();
    fs::write(&wal_path, b"wal").unwrap();
    fs::write(&shm_path, b"shm").unwrap();

    delete_database_files(&db_path).unwrap();

    assert!(!db_path.exists());
    assert!(!wal_path.exists());
    assert!(!shm_path.exists());
  }

  #[test]
  fn delete_database_files_ok_when_no_file() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("nonexistent.db");

    assert!(delete_database_files(&db_path).is_ok());
  }

  #[test]
  fn delete_database_files_ok_without_wal_shm() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    fs::write(&db_path, b"db").unwrap();

    delete_database_files(&db_path).unwrap();

    assert!(!db_path.exists());
  }
}
