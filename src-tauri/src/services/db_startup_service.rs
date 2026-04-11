use crate::infrastructure::database::preflight::DbStartupError;
use crate::utils;
use std::path::Path;
use tauri_plugin_dialog::{
  DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};

const RESET_LABEL: &str = "Reset and Restart";
const CONTINUE_LABEL: &str = "Continue Anyway";
const EXIT_LABEL: &str = "Exit";

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

/// Delete the database file (and WAL/SHM companions), then restart the app.
pub fn reset_database_and_restart(handle: &tauri::AppHandle) {
  let db_path = utils::file::get_app_data_dir("hv-database.db");
  if let Err(e) = delete_database_files(&db_path) {
    show_error_dialog(handle, &format!("Failed to delete database file: {e}"));
    handle.exit(1);
    return;
  }

  let exe_path = match std::env::current_exe() {
    Ok(path) => path,
    Err(e) => {
      show_error_dialog(handle, &format!("Failed to obtain executable path: {e}"));
      handle.exit(1);
      return;
    }
  };
  let args: Vec<String> = std::env::args().collect();
  #[allow(clippy::zombie_processes)]
  if let Err(e) = std::process::Command::new(exe_path)
    .args(&args[1..])
    .spawn()
  {
    show_error_dialog(handle, &format!("Failed to restart process: {e}"));
    handle.exit(1);
    return;
  }
  handle.exit(0);
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
         * Resetting will delete all archived hardware monitoring history.\n\n\
         [Details: data schema v{db_max_version}, app supports up to v{app_max_version}]"
      )
    }
    DbStartupError::Other(msg) => {
      format!(
        "HardwareVisualizer could not read the existing data.\n\n\
         You can reset the data to continue using the app.\n\n\
         * Resetting will delete all archived hardware monitoring history.\n\n\
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
