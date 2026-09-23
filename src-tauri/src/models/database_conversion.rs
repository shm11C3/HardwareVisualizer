//! Wire-format mirror of [`crate::app::native_lifecycle::DatabaseLifecycleState`]
//! for the #2136 explicit conversion flow.
//!
//! The frontend renders exactly this small, enumerable vocabulary rather
//! than a second interpretation of the lifecycle owner's state - see
//! `app::native_lifecycle`'s own module documentation for why that
//! vocabulary stays small on purpose. `NotSupported` is the one variant
//! with no App-side counterpart: it is what a build without the
//! `duckdb-archive` feature reports, so the frontend can hide the
//! conversion entry point instead of guessing from a command error.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Wire mirror of `app::native_lifecycle::ConversionProgress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ConversionStep {
  Preflight,
  BuildingCandidate,
  Finalizing,
  PausingProducers,
  Reconciling,
  Selecting,
  ResumingProducers,
}

#[cfg(feature = "duckdb-archive")]
impl From<crate::app::native_lifecycle::ConversionProgress> for ConversionStep {
  fn from(value: crate::app::native_lifecycle::ConversionProgress) -> Self {
    use crate::app::native_lifecycle::ConversionProgress as P;
    match value {
      P::Preflight => Self::Preflight,
      P::BuildingCandidate => Self::BuildingCandidate,
      P::Finalizing => Self::Finalizing,
      P::PausingProducers => Self::PausingProducers,
      P::Reconciling => Self::Reconciling,
      P::Selecting => Self::Selecting,
      P::ResumingProducers => Self::ResumingProducers,
    }
  }
}

/// Wire mirror of `app::native_lifecycle::DatabaseLifecycleState`, widened
/// with [`Self::NotSupported`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DatabaseConversionState {
  /// This build does not include the native database conversion feature
  /// (`duckdb-archive` is off). The frontend hides the conversion entry
  /// point rather than showing a control nothing behind it can act on.
  NotSupported,
  /// SQLite is authoritative and no conversion has produced anything yet.
  SqliteAuthoritative,
  /// A previous conversion left recoverable state; SQLite is still
  /// authoritative. `resumable` says whether a complete finalized file
  /// exists to resume from reconciliation, or whether the copy restarts.
  ConversionRecoverable { resumable: bool },
  /// The conversion is running.
  Converting { step: ConversionStep },
  /// The native database is authoritative and open.
  NativeAuthoritative,
  /// The lifecycle owner stopped rather than guess. `reason` is a stable
  /// key the frontend maps through i18n into a user-actionable message;
  /// `diagnostic` is the underlying `Debug` detail, meant for a
  /// collapsible "technical details" section rather than the primary
  /// message (#2136: actionable without leaking implementation detail in
  /// normal flows, diagnostics stay reachable for the cases that need
  /// them).
  ActionRequired { reason: String, diagnostic: String },
}

#[cfg(feature = "duckdb-archive")]
impl From<crate::app::native_lifecycle::DatabaseLifecycleState>
  for DatabaseConversionState
{
  fn from(value: crate::app::native_lifecycle::DatabaseLifecycleState) -> Self {
    use crate::app::native_lifecycle::DatabaseLifecycleState as S;
    match value {
      S::SqliteAuthoritative => Self::SqliteAuthoritative,
      S::ConversionRecoverable { resumable } => Self::ConversionRecoverable { resumable },
      S::Converting(step) => Self::Converting { step: step.into() },
      S::NativeAuthoritative => Self::NativeAuthoritative,
      S::ActionRequired(issue) => {
        let (reason, diagnostic) = describe_issue(&issue);
        Self::ActionRequired { reason, diagnostic }
      }
    }
  }
}

/// A stable i18n key per [`crate::app::native_lifecycle::LifecycleIssue`]
/// case, paired with the `Debug` detail for diagnostics. None of these
/// leak a file path or an internal type into the normal-flow message; the
/// frontend looks the key up in translation, it never displays `reason`
/// directly.
#[cfg(feature = "duckdb-archive")]
fn describe_issue(
  issue: &crate::app::native_lifecycle::LifecycleIssue,
) -> (String, String) {
  use crate::app::native_lifecycle::LifecycleIssue as I;
  use hardviz_core::infrastructure::database::native_database::AuthorityInconsistency;
  let reason = match issue {
    I::Authority(AuthorityInconsistency::NativeMetadataUnreadable) => {
      "nativeMetadataUnreadable"
    }
    I::Authority(_) => "authorityDisagreement",
    I::NativeOpenFailed { .. } => "nativeOpenFailed",
    I::FreshCreationFailed { .. } => "freshCreationFailed",
    I::ConversionFailed { .. } => "conversionFailed",
    I::ConversionCancelled { .. } => "conversionCancelled",
  };
  (reason.to_string(), format!("{issue:?}"))
}

#[cfg(all(test, feature = "duckdb-archive"))]
mod tests {
  use super::*;
  use crate::app::native_lifecycle::{
    ConversionProgress, DatabaseLifecycleState, LifecycleIssue,
  };
  use hardviz_core::infrastructure::database::native_database::AuthorityInconsistency;

  #[test]
  fn sqlite_authoritative_maps_directly() {
    assert_eq!(
      DatabaseConversionState::from(DatabaseLifecycleState::SqliteAuthoritative),
      DatabaseConversionState::SqliteAuthoritative
    );
  }

  #[test]
  fn converting_carries_the_step() {
    assert_eq!(
      DatabaseConversionState::from(DatabaseLifecycleState::Converting(
        ConversionProgress::Reconciling
      )),
      DatabaseConversionState::Converting {
        step: ConversionStep::Reconciling
      }
    );
  }

  #[test]
  fn action_required_never_embeds_the_debug_detail_in_the_reason_key() {
    let state = DatabaseConversionState::from(DatabaseLifecycleState::ActionRequired(
      LifecycleIssue::Authority(AuthorityInconsistency::SourceDatabaseMissing),
    ));

    let DatabaseConversionState::ActionRequired { reason, diagnostic } = state else {
      panic!("expected ActionRequired");
    };
    assert_eq!(reason, "authorityDisagreement");
    assert!(diagnostic.contains("SourceDatabaseMissing"));
  }

  #[test]
  fn unreadable_native_metadata_has_a_retryable_reason_key() {
    let state = DatabaseConversionState::from(DatabaseLifecycleState::ActionRequired(
      LifecycleIssue::Authority(AuthorityInconsistency::NativeMetadataUnreadable),
    ));

    let DatabaseConversionState::ActionRequired { reason, diagnostic } = state else {
      panic!("expected ActionRequired");
    };
    assert_eq!(reason, "nativeMetadataUnreadable");
    assert!(diagnostic.contains("NativeMetadataUnreadable"));
  }
}
