//! Whether a DB-dependent read command may answer, given the current
//! native database lifecycle state.
//!
//! Unconditional module (unlike `native_lifecycle`) so command files do not
//! need to `#[cfg]` every call site: without the `duckdb-archive` feature
//! there is no lifecycle owner and nothing to guard against, so the check is
//! simply `Ok(())`.

/// The decision, given the current lifecycle state: refuse only while it is
/// `ActionRequired`.
///
/// `inspect_authority` already refused to guess which database is correct
/// in that state (see `app::native_lifecycle`), and the App's DB-dependent
/// workers are already not running (`lib.rs`'s `start_sqlite_backed_producers`).
/// Answering a read from Core's SQLite pool regardless would present
/// whatever that pool currently holds - possibly the stale recovery copy of
/// an already-selected native database - as current data, despite the
/// authority decision having refused to choose it.
///
/// Kept separate from [`ensure_database_available`] so the decision itself
/// is testable without constructing a Tauri app: this module's tests build
/// no `AppHandle` at all.
#[cfg(feature = "duckdb-archive")]
fn database_available(
  state: &crate::app::native_lifecycle::DatabaseLifecycleState,
) -> Result<(), String> {
  match state {
    crate::app::native_lifecycle::DatabaseLifecycleState::ActionRequired(issue) => {
      Err(format!(
        "the database is unavailable: startup found the native database files in an \
       unexpected state and stopped rather than guess which one is correct \
       ({issue:?})"
      ))
    }
    _ => Ok(()),
  }
}

/// Refuse a DB-dependent read while startup left the native lifecycle in
/// `ActionRequired`. Called at the top of every command that reads Hardware
/// Archive, Cooling or Storage Health history.
#[cfg(feature = "duckdb-archive")]
pub fn ensure_database_available(app: &tauri::AppHandle) -> Result<(), String> {
  use tauri::Manager;

  use crate::app::native_lifecycle::NativeLifecycleOwner;

  match app.try_state::<NativeLifecycleOwner>() {
    // Absent only in a context (e.g. a unit test) that never managed the
    // owner; nothing to refuse.
    None => Ok(()),
    Some(owner) => database_available(&owner.state()),
  }
}

#[cfg(not(feature = "duckdb-archive"))]
pub fn ensure_database_available(_app: &tauri::AppHandle) -> Result<(), String> {
  Ok(())
}

#[cfg(all(test, feature = "duckdb-archive"))]
mod tests {
  use hardviz_core::infrastructure::database::native_database::AuthorityInconsistency;

  use super::*;
  use crate::app::native_lifecycle::{DatabaseLifecycleState, LifecycleIssue};

  #[test]
  fn answers_ok_for_every_state_but_action_required() {
    for state in [
      DatabaseLifecycleState::SqliteAuthoritative,
      DatabaseLifecycleState::ConversionRecoverable { resumable: true },
      DatabaseLifecycleState::ConversionRecoverable { resumable: false },
      DatabaseLifecycleState::NativeAuthoritative,
    ] {
      assert!(database_available(&state).is_ok(), "{state:?}");
    }
  }

  #[test]
  fn refuses_while_action_required() {
    let state = DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(
      AuthorityInconsistency::SourceDatabaseMissing,
    ));
    let error = database_available(&state).unwrap_err();
    assert!(error.contains("unavailable"), "{error}");
  }
}
