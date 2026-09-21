// CI validation touch for PR #2199 (test-tauri-duckdb-archive job); revert
// before merge, this file has no functional change.
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
/// in that state (see `app::native_lifecycle`), and dispatch (#2134) itself
/// answers nothing while its own boundary is `Active::Unavailable` in that
/// same state (`DispatchError::NativeUnavailable`). This is a second,
/// App-level refusal in front of that one - a clearer error before the
/// service layer is even reached, not a substitute for it - so a command
/// that reads Hardware Archive, Cooling or Storage Health history never
/// answers from whatever a stale SQLite pool happens to hold while the
/// authority decision has refused to choose a backend.
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

/// Refuse an on-demand *write* command
/// (`commands::hardware::refresh_storage_devices`, the only one outside the
/// producers `native_conversion::pause_and_drain_producers` already stops)
/// unless a write issued through dispatch is currently safe
/// ([`crate::app::native_lifecycle::database_writable`]).
///
/// Stricter than [`ensure_database_available`] in one respect and looser in
/// another: a read is only wrong once startup gave up (`ActionRequired`),
/// but a write is *also* wrong for the whole window a conversion is
/// quiescing producers to capture a consistent snapshot (`Converting`) -
/// this command is not one of the producers that window pauses, so a write
/// that lands during it could be silently absent from what reconciliation
/// selects. Once dispatch has actually adopted a selection
/// (`NativeAuthoritative`), though, a write is fine: dispatch answers it
/// from the native database, not a possibly-retired SQLite file.
#[cfg(feature = "duckdb-archive")]
pub fn ensure_database_writable(app: &tauri::AppHandle) -> Result<(), String> {
  use tauri::Manager;

  use crate::app::native_lifecycle::{NativeLifecycleOwner, database_writable};

  match app.try_state::<NativeLifecycleOwner>() {
    None => Ok(()),
    Some(owner) => {
      let state = owner.state();
      if database_writable(&state) {
        Ok(())
      } else {
        Err(format!(
          "the database is unavailable for writing right now ({state:?})"
        ))
      }
    }
  }
}

#[cfg(not(feature = "duckdb-archive"))]
pub fn ensure_database_writable(_app: &tauri::AppHandle) -> Result<(), String> {
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

  // `ensure_database_writable` is a thin `AppHandle` -> owner-state wrapper
  // around `native_lifecycle::database_writable`, the same shape as
  // `ensure_database_available` around `database_available` above - see
  // that module's own tests for the decision table this delegates to.
}
