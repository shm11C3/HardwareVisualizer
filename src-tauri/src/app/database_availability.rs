//! Whether a DB-dependent read command may answer, given the current
//! native database lifecycle state.
//!
//! Unconditional module (unlike `native_lifecycle`) so command files do not
//! need to `#[cfg]` every call site: without the `duckdb-archive` feature
//! there is no lifecycle owner and nothing to guard against, so the check is
//! simply `Ok(())`.

/// The decision, given the current lifecycle state: refuse only when no
/// backend is safely answering.
///
/// `ActionRequired` is not a single case for this purpose. `run_conversion`
/// (`app::native_conversion`) resumes the paused producers, and SQLite stays
/// authoritative, for exactly two of its issues: `ConversionFailed` (the
/// step failed before a selection was made durable) and `ConversionCancelled`
/// (the operator cancelled before that point). A read answered from SQLite
/// in either case is answered from the same backend that was authoritative,
/// open and being written to before the conversion attempt - there is
/// nothing tentative about it, so refusing it would only strand every
/// history read for the rest of the session over an attempt that already
/// resolved.
///
/// Every other `ActionRequired` issue refuses, because no backend is safely
/// answering: `Authority(..)` is a marker/native disagreement
/// `inspect_authority` refused to guess at (see `app::native_lifecycle`);
/// `NativeOpenFailed` means a durable selection exists but the native
/// database could not be opened, so SQLite is a stale recovery copy and the
/// producers stay paused; `FreshCreationFailed` means even a fresh profile
/// has no working database at all. Dispatch (#2134) itself answers nothing
/// while its own boundary is `Active::Unavailable` in the same underlying
/// state (`DispatchError::NativeUnavailable`); this is a second, App-level
/// refusal in front of that one - a clearer error before the service layer
/// is even reached, not a substitute for it.
///
/// Kept separate from [`ensure_database_available`] so the decision itself
/// is testable without constructing a Tauri app: this module's tests build
/// no `AppHandle` at all.
///
/// `pub(crate)` (rather than private) only so `native_conversion`'s own
/// tests can assert against it directly after a cancelled/failed
/// conversion; not part of the App's external command surface.
#[cfg(feature = "duckdb-archive")]
pub(crate) fn database_available(
  state: &crate::app::native_lifecycle::DatabaseLifecycleState,
) -> Result<(), String> {
  use crate::app::native_lifecycle::{DatabaseLifecycleState, LifecycleIssue};

  match state {
    DatabaseLifecycleState::ActionRequired(
      LifecycleIssue::ConversionFailed { .. }
      | LifecycleIssue::ConversionCancelled { .. },
    ) => Ok(()),
    DatabaseLifecycleState::ActionRequired(issue) => Err(format!(
      "the database is unavailable: startup found the native database files in an \
       unexpected state and stopped rather than guess which one is correct \
       ({issue:?})"
    )),
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
  use crate::app::native_lifecycle::{
    ConversionProgress, DatabaseLifecycleState, LifecycleIssue,
  };

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

  /// Regression for the bug found by the 2026-09-24 release audit: SQLite
  /// stays authoritative and its producers are resumed after a failed or
  /// cancelled conversion (see `native_conversion::run_conversion`'s
  /// resume/stay-paused branch), so reads must not be refused for the rest
  /// of the session over an attempt that already resolved.
  #[test]
  fn answers_ok_after_a_failed_or_cancelled_conversion_because_sqlite_stays_authoritative()
   {
    for state in [
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::ConversionFailed {
        step: ConversionProgress::BuildingCandidate,
        message: "disk full".to_string(),
      }),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::ConversionCancelled {
        step: ConversionProgress::BuildingCandidate,
      }),
    ] {
      assert!(database_available(&state).is_ok(), "{state:?}");
    }
  }

  #[test]
  fn refuses_while_action_required_and_no_backend_is_safely_answering() {
    for state in [
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::Authority(
        AuthorityInconsistency::SourceDatabaseMissing,
      )),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed {
        message: "could not open handle".to_string(),
      }),
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::FreshCreationFailed {
        message: "could not create database".to_string(),
      }),
    ] {
      let error = database_available(&state).unwrap_err();
      assert!(error.contains("unavailable"), "{error}");
    }
  }

  // `ensure_database_writable` is a thin `AppHandle` -> owner-state wrapper
  // around `native_lifecycle::database_writable`, the same shape as
  // `ensure_database_available` around `database_available` above - see
  // that module's own tests for the decision table this delegates to.
}
