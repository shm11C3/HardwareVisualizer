use thiserror::Error;

/// Error type for the Core platform layer (`crate::platform`).
///
/// Variants preserve the DP-02 distinction between data availability and
/// faults, mirroring [`crate::models::SensorAvailability`]:
///
/// - `Unsupported`: the platform or hardware path does not provide the
///   value by design (build stub, OS-exclusive feature).
/// - `Unavailable`: the path exists, but the current attempt could not
///   produce a user-visible value (warm-up, every provider fallback
///   exhausted, nothing detected).
/// - `Fault`: a supported operation failed unexpectedly (OS API error,
///   provider failure, task join error).
/// - `InitializationFailed`: the OS platform implementation could not be
///   constructed.
///
/// The App-side `commands::*` translate these into wire errors before
/// returning them to the frontend; Core never depends on the wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlatformError {
  #[error("{reason}")]
  Unsupported { reason: String },
  #[error("{reason}")]
  Unavailable { reason: String },
  #[error("{reason}")]
  Fault { reason: String },
  #[error("Platform initialization failed: {reason}")]
  InitializationFailed { reason: String },
}

impl PlatformError {
  pub fn unsupported(reason: impl Into<String>) -> Self {
    Self::Unsupported {
      reason: reason.into(),
    }
  }

  pub fn unavailable(reason: impl Into<String>) -> Self {
    Self::Unavailable {
      reason: reason.into(),
    }
  }

  pub fn fault(reason: impl Into<String>) -> Self {
    Self::Fault {
      reason: reason.into(),
    }
  }

  pub fn initialization_failed(reason: impl Into<String>) -> Self {
    Self::InitializationFailed {
      reason: reason.into(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn display_forwards_the_reason_unchanged_for_data_errors() {
    assert_eq!(
      PlatformError::unsupported("not implemented for X").to_string(),
      "not implemented for X"
    );
    assert_eq!(
      PlatformError::unavailable("no sensor found").to_string(),
      "no sensor found"
    );
    assert_eq!(
      PlatformError::fault("WMI query failed").to_string(),
      "WMI query failed"
    );
  }

  #[test]
  fn display_keeps_the_established_initialization_prefix() {
    // App-side call sites format this error into user-visible strings;
    // the prefix matches the pre-typed factory Display output.
    assert_eq!(
      PlatformError::initialization_failed("boom").to_string(),
      "Platform initialization failed: boom"
    );
  }

  #[test]
  fn constructors_classify_into_their_variant() {
    assert!(matches!(
      PlatformError::unsupported("r"),
      PlatformError::Unsupported { .. }
    ));
    assert!(matches!(
      PlatformError::unavailable("r"),
      PlatformError::Unavailable { .. }
    ));
    assert!(matches!(
      PlatformError::fault("r"),
      PlatformError::Fault { .. }
    ));
    assert!(matches!(
      PlatformError::initialization_failed("r"),
      PlatformError::InitializationFailed { .. }
    ));
  }
}
