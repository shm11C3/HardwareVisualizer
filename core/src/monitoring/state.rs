use std::fmt;

/// Lifecycle state of sensor collection.
///
/// `Running` is the default. `Paused` halts new sample emission while
/// keeping the collector resident (Phase 5 doesn't wire pause to the
/// runtime yet — that's #1275). `Stopped` is terminal: the worker tasks
/// are draining and the process is on its way to `exit(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonitoringState {
  Running,
  Paused,
  Stopped,
}

/// Returned when a caller asks for a transition the state machine
/// rejects: any transition that doesn't change state, or any transition
/// out of [`MonitoringState::Stopped`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTransition {
  pub from: MonitoringState,
  pub to: MonitoringState,
}

impl fmt::Display for InvalidTransition {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "invalid monitoring transition: {:?} -> {:?}",
      self.from, self.to
    )
  }
}

impl std::error::Error for InvalidTransition {}

impl MonitoringState {
  /// Initial state for a freshly-started monitor.
  pub fn new() -> Self {
    Self::Running
  }

  pub fn is_running(&self) -> bool {
    matches!(self, Self::Running)
  }

  pub fn is_paused(&self) -> bool {
    matches!(self, Self::Paused)
  }

  pub fn is_stopped(&self) -> bool {
    matches!(self, Self::Stopped)
  }

  /// Pause collection. Valid only from [`MonitoringState::Running`].
  pub fn pause(&mut self) -> Result<(), InvalidTransition> {
    self.try_transition(Self::Paused)
  }

  /// Resume collection. Valid only from [`MonitoringState::Paused`].
  pub fn resume(&mut self) -> Result<(), InvalidTransition> {
    self.try_transition(Self::Running)
  }

  /// Move to [`MonitoringState::Stopped`]. Valid from `Running` or
  /// `Paused`. Terminal — no transitions out of `Stopped` are accepted.
  pub fn stop(&mut self) -> Result<(), InvalidTransition> {
    self.try_transition(Self::Stopped)
  }

  /// Generic transition entry point. Prefer [`Self::pause`],
  /// [`Self::resume`], [`Self::stop`] at call sites; this is exposed for
  /// callers that hold a target state value (e.g. test parameterization).
  pub fn try_transition(
    &mut self,
    target: MonitoringState,
  ) -> Result<(), InvalidTransition> {
    if Self::is_valid_transition(*self, target) {
      *self = target;
      Ok(())
    } else {
      Err(InvalidTransition {
        from: *self,
        to: target,
      })
    }
  }

  fn is_valid_transition(from: MonitoringState, to: MonitoringState) -> bool {
    use MonitoringState::*;
    matches!(
      (from, to),
      (Running, Paused) | (Paused, Running) | (Running, Stopped) | (Paused, Stopped)
    )
  }
}

impl Default for MonitoringState {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_state_is_running() {
    assert_eq!(MonitoringState::new(), MonitoringState::Running);
    assert_eq!(MonitoringState::default(), MonitoringState::Running);
  }

  #[test]
  fn predicates_match_state() {
    assert!(MonitoringState::Running.is_running());
    assert!(!MonitoringState::Running.is_paused());
    assert!(!MonitoringState::Running.is_stopped());

    assert!(MonitoringState::Paused.is_paused());
    assert!(!MonitoringState::Paused.is_running());
    assert!(!MonitoringState::Paused.is_stopped());

    assert!(MonitoringState::Stopped.is_stopped());
    assert!(!MonitoringState::Stopped.is_running());
    assert!(!MonitoringState::Stopped.is_paused());
  }

  // ── Valid transitions ──

  #[test]
  fn running_to_paused_is_valid() {
    let mut s = MonitoringState::Running;
    s.pause().unwrap();
    assert_eq!(s, MonitoringState::Paused);
  }

  #[test]
  fn paused_to_running_is_valid() {
    let mut s = MonitoringState::Paused;
    s.resume().unwrap();
    assert_eq!(s, MonitoringState::Running);
  }

  #[test]
  fn running_to_stopped_is_valid() {
    let mut s = MonitoringState::Running;
    s.stop().unwrap();
    assert_eq!(s, MonitoringState::Stopped);
  }

  #[test]
  fn paused_to_stopped_is_valid() {
    let mut s = MonitoringState::Paused;
    s.stop().unwrap();
    assert_eq!(s, MonitoringState::Stopped);
  }

  // ── Invalid: no-op same-state transitions ──

  #[test]
  fn running_to_running_is_rejected() {
    let mut s = MonitoringState::Running;
    let err = s.try_transition(MonitoringState::Running).unwrap_err();
    assert_eq!(err.from, MonitoringState::Running);
    assert_eq!(err.to, MonitoringState::Running);
    // State unchanged on failure.
    assert_eq!(s, MonitoringState::Running);
  }

  #[test]
  fn paused_to_paused_is_rejected() {
    let mut s = MonitoringState::Paused;
    assert!(s.pause().is_err());
    assert_eq!(s, MonitoringState::Paused);
  }

  #[test]
  fn resume_from_running_is_rejected() {
    let mut s = MonitoringState::Running;
    assert!(s.resume().is_err());
    assert_eq!(s, MonitoringState::Running);
  }

  // ── Invalid: Stopped is terminal ──

  #[test]
  fn stopped_to_running_is_rejected() {
    let mut s = MonitoringState::Stopped;
    let err = s.try_transition(MonitoringState::Running).unwrap_err();
    assert_eq!(err.from, MonitoringState::Stopped);
    assert_eq!(err.to, MonitoringState::Running);
    assert_eq!(s, MonitoringState::Stopped);
  }

  #[test]
  fn stopped_to_paused_is_rejected() {
    let mut s = MonitoringState::Stopped;
    assert!(s.try_transition(MonitoringState::Paused).is_err());
    assert_eq!(s, MonitoringState::Stopped);
  }

  #[test]
  fn stopped_to_stopped_is_rejected() {
    let mut s = MonitoringState::Stopped;
    assert!(s.stop().is_err());
    assert_eq!(s, MonitoringState::Stopped);
  }

  // ── Invalid: cannot pause from Stopped ──

  #[test]
  fn pause_from_paused_is_rejected() {
    let mut s = MonitoringState::Paused;
    assert!(s.pause().is_err());
  }

  #[test]
  fn pause_from_stopped_is_rejected() {
    let mut s = MonitoringState::Stopped;
    assert!(s.pause().is_err());
  }

  #[test]
  fn resume_from_stopped_is_rejected() {
    let mut s = MonitoringState::Stopped;
    assert!(s.resume().is_err());
  }

  // ── Exhaustive coverage of the (from, to) grid ──

  #[test]
  fn transition_grid_matches_specification() {
    use MonitoringState::*;
    let cases = [
      // (from, to, expected_valid)
      (Running, Running, false),
      (Running, Paused, true),
      (Running, Stopped, true),
      (Paused, Running, true),
      (Paused, Paused, false),
      (Paused, Stopped, true),
      (Stopped, Running, false),
      (Stopped, Paused, false),
      (Stopped, Stopped, false),
    ];

    for (from, to, expected_valid) in cases {
      let mut s = from;
      let result = s.try_transition(to);
      assert_eq!(
        result.is_ok(),
        expected_valid,
        "transition {:?} -> {:?} expected valid={}, got result={:?}",
        from,
        to,
        expected_valid,
        result
      );
      // On success, state advances; on failure, state is unchanged.
      let expected_after = if expected_valid { to } else { from };
      assert_eq!(s, expected_after);
    }
  }

  #[test]
  fn invalid_transition_display_includes_states() {
    let err = InvalidTransition {
      from: MonitoringState::Stopped,
      to: MonitoringState::Running,
    };
    let msg = err.to_string();
    assert!(msg.contains("Stopped"));
    assert!(msg.contains("Running"));
  }
}
