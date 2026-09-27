//! App-side orchestration of External Component Setup (ADR 0024).
//!
//! The service never installs anything in the app process. It reads the
//! component state through Core, and for a setup run it launches the current
//! executable elevated in its setup command-line mode, waits for it, and
//! derives the outcome from the exit code of the process handle it owns. No
//! result file or pipe exists for a same-user process to redirect or forge.

use std::sync::Mutex;

use hardviz_core::external_component_setup::{
  self as core_setup, ExternalComponentSetupPlan, setup_plan,
};
use hardviz_core::models::ExternalComponent;
use hardviz_core::platform::factory::PlatformFactory;
use hardviz_core::platform::traits::{
  ElevatedProcessRun, ExternalComponentSetupPlatform,
};

use crate::cli::{EXTERNAL_COMPONENT_SETUP_FLAG, component_cli_id};
use crate::log_warn;
use crate::models::external_component_setup::{
  ExternalComponentSetupOutcome, ExternalComponentSetupResult,
};
use crate::services::external_component_guidance_service::ExternalComponentGuidanceState;

/// Components with a setup run in flight. One elevated setup per component at
/// a time; a second request while one runs is rejected instead of starting a
/// duplicate installer.
static IN_FLIGHT: Mutex<Vec<ExternalComponent>> = Mutex::new(Vec::new());

/// Marks a component as in flight until dropped, or forever when the run's
/// elevated child could not be confirmed dead (see [`Self::keep_held`]).
struct InFlightGuard(ExternalComponent);

impl InFlightGuard {
  fn acquire(component: ExternalComponent) -> Result<Self, String> {
    let mut in_flight = IN_FLIGHT
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    if in_flight.contains(&component) {
      return Err(format!(
        "External Component Setup for {} is already running",
        component_cli_id(component)
      ));
    }
    in_flight.push(component);
    Ok(Self(component))
  }

  /// Leave the component marked in flight for the rest of the app's life.
  /// Used when the elevated child may still be running: the app holds no
  /// handle that can end it, so a retry could start a second installer next
  /// to it. The mark clears with the app restart the user is told to do.
  fn keep_held(self) {
    // Forgetting skips `Drop`, which is the only place the mark is cleared.
    std::mem::forget(self);
  }
}

impl Drop for InFlightGuard {
  fn drop(&mut self) {
    let mut in_flight = IN_FLIGHT
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    in_flight.retain(|component| *component != self.0);
  }
}

fn plan_for(
  component: ExternalComponent,
) -> Result<&'static ExternalComponentSetupPlan, String> {
  setup_plan(component).ok_or_else(|| {
    format!(
      "{} has no External Component Setup plan",
      component_cli_id(component)
    )
  })
}

pub fn status(
  component: ExternalComponent,
) -> Result<core_setup::ExternalComponentSetupStatus, String> {
  let plan = plan_for(component)?;
  let platform = PlatformFactory::shared().map_err(|e| e.to_string())?;
  Ok(platform.external_component_setup_status(plan))
}

/// Run setup for `component` in an elevated child process and report the
/// outcome together with the refreshed state. Blocks until the child exits;
/// call it from a blocking task.
///
/// A run that installed the component also defers its External Component
/// Guidance for the session through `guidance`. The sensor providers keep the
/// failed probe cached until the app restarts, so the snapshot would otherwise
/// keep offering "install, then restart" for a component the user just
/// installed (#2249). The deferral lives here rather than in the command so
/// that the outcome-to-guidance rule is tested with the run itself; both the
/// guidance state and this service are App-owned, so no ownership boundary is
/// crossed.
pub fn run(
  component: ExternalComponent,
  guidance: &ExternalComponentGuidanceState,
) -> Result<ExternalComponentSetupResult, String> {
  let platform = PlatformFactory::shared().map_err(|e| e.to_string())?;
  run_with(platform.as_ref(), component, guidance)
}

fn run_with(
  platform: &dyn ExternalComponentSetupPlatform,
  component: ExternalComponent,
  guidance: &ExternalComponentGuidanceState,
) -> Result<ExternalComponentSetupResult, String> {
  let result = run_elevated(platform, component)?;
  // `AlreadyInstalled` is excluded: nothing changed, so a guidance the
  // providers still report (for example a permission problem) stays valid.
  if matches!(
    result.outcome,
    ExternalComponentSetupOutcome::Installed
      | ExternalComponentSetupOutcome::RebootRequired
  ) {
    guidance.defer_component_for_session(component);
  }
  Ok(result)
}

fn run_elevated(
  platform: &dyn ExternalComponentSetupPlatform,
  component: ExternalComponent,
) -> Result<ExternalComponentSetupResult, String> {
  let plan = plan_for(component)?;
  let guard = InFlightGuard::acquire(component)?;

  let before = platform.external_component_setup_status(plan);
  if let Some(blocker) = before.setup_blocker() {
    let stage = if before.support == core_setup::ExternalComponentSetupSupport::Supported
    {
      core_setup::SetupFailureStage::StateUnknown
    } else {
      core_setup::SetupFailureStage::UnsupportedPlatform
    };
    return Ok(ExternalComponentSetupResult::from_outcome(
      core_setup::ExternalComponentSetupOutcome::failed(stage, blocker),
      before,
    ));
  }
  if before.is_complete() {
    return Ok(ExternalComponentSetupResult::from_outcome(
      core_setup::ExternalComponentSetupOutcome::AlreadyInstalled,
      before,
    ));
  }

  let args = setup_args(component);
  let run = platform
    .run_current_executable_elevated(&args)
    .map_err(|e| e.to_string())?;
  let after = platform.external_component_setup_status(plan);

  // `guard` drops with every return below unless something may still be
  // running: the elevated child itself (`StillRunning`), or the installer it
  // could not confirm stopped (`InstallerStillRunning`). A child that is
  // confirmed gone frees the component for a retry without an app restart.
  Ok(match run {
    ElevatedProcessRun::Declined => ExternalComponentSetupResult::cancelled(after),
    ElevatedProcessRun::StillRunning => {
      log_warn!(
        format!(
          "external component setup for {} did not finish in time and could not be \
           confirmed stopped; refusing retries until the app restarts",
          component_cli_id(component)
        ),
        "external_component_setup_service::run",
        None::<&str>
      );
      guard.keep_held();
      ExternalComponentSetupResult::still_running(
        after,
        "the setup process did not finish in time and may still be running; restart \
         the app before trying again",
      )
    }
    ElevatedProcessRun::TimedOut => {
      log_warn!(
        format!(
          "external component setup for {} did not finish in time and was stopped",
          component_cli_id(component)
        ),
        "external_component_setup_service::run",
        None::<&str>
      );
      ExternalComponentSetupResult::timed_out(
        after,
        "the setup process did not finish in time and was stopped",
      )
    }
    ElevatedProcessRun::Exited { exit_code } => {
      let outcome = core_setup::ExternalComponentSetupOutcome::from_exit_code(exit_code);
      if let core_setup::ExternalComponentSetupOutcome::Failed { stage, detail } =
        &outcome
      {
        log_warn!(
          format!(
            "external component setup for {} failed at {stage:?}: {detail}",
            component_cli_id(component)
          ),
          "external_component_setup_service::run",
          None::<&str>
        );
        if *stage == core_setup::SetupFailureStage::InstallerStillRunning {
          // The child exited, but the installer it started may still be
          // running from the staging directory it left behind; a retry would
          // start a second installer beside it.
          guard.keep_held();
        }
      }
      ExternalComponentSetupResult::from_outcome(outcome, after)
    }
  })
}

fn setup_args(component: ExternalComponent) -> Vec<String> {
  vec![
    EXTERNAL_COMPONENT_SETUP_FLAG.to_string(),
    component_cli_id(component).to_string(),
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn setup_args_name_the_component_only() {
    assert_eq!(
      setup_args(ExternalComponent::Pawnio),
      vec!["--external-component-setup", "pawnio"]
    );
  }

  #[test]
  fn in_flight_guard_rejects_a_duplicate_and_releases_on_drop() {
    let _serialized = serialized();
    let first = InFlightGuard::acquire(ExternalComponent::Smartctl).unwrap();
    assert!(InFlightGuard::acquire(ExternalComponent::Smartctl).is_err());
    let other = InFlightGuard::acquire(ExternalComponent::Pawnio).unwrap();
    drop(first);
    assert!(InFlightGuard::acquire(ExternalComponent::Smartctl).is_ok());
    drop(other);
  }

  use crate::models::external_component_setup::ExternalComponentSetupFailureStage;
  use crate::services::external_component_guidance_service::ExternalComponentGuidanceView;
  use hardviz_core::enums::error::PlatformError;
  use hardviz_core::external_component_setup::{
    ExternalComponentSetupPlan, ExternalComponentSetupStatus,
    ExternalComponentSetupSupport,
  };

  /// A supported platform whose component needs setup and whose elevated run
  /// always ends the same way.
  struct FakePlatform(ElevatedProcessRun);

  impl ExternalComponentSetupPlatform for FakePlatform {
    fn external_component_setup_status(
      &self,
      plan: &ExternalComponentSetupPlan,
    ) -> ExternalComponentSetupStatus {
      let mut status = ExternalComponentSetupStatus::unsupported_platform(plan);
      status.support = ExternalComponentSetupSupport::Supported;
      status
    }

    fn run_external_component_setup(
      &self,
      _plan: &ExternalComponentSetupPlan,
    ) -> core_setup::ExternalComponentSetupResult {
      unreachable!("the service never runs setup in process")
    }

    fn refresh_external_component_files(
      &self,
      _plan: &ExternalComponentSetupPlan,
    ) -> core_setup::ExternalComponentSetupResult {
      unreachable!("the service never refreshes in process")
    }

    fn run_current_executable_elevated(
      &self,
      _args: &[String],
    ) -> Result<ElevatedProcessRun, PlatformError> {
      Ok(self.0)
    }
  }

  /// The in-flight mark is process-global, so the tests that touch it run
  /// one at a time regardless of the test harness's thread count.
  static IN_FLIGHT_TESTS: Mutex<()> = Mutex::new(());

  fn serialized() -> std::sync::MutexGuard<'static, ()> {
    IN_FLIGHT_TESTS
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  fn guidance() -> ExternalComponentGuidanceState {
    ExternalComponentGuidanceState::default()
  }

  /// The snapshot candidates the providers keep reporting until the app
  /// restarts, as the guidance command re-records them on every poll.
  fn record_cached_pawnio_candidates(guidance: &ExternalComponentGuidanceState) {
    use hardviz_core::models::ExternalComponentGuidanceCandidate;
    guidance.record_candidates(vec![
      ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
        "PawnIOLib.dll not found".to_string(),
      ),
      ExternalComponentGuidanceCandidate::pawnio_motherboard_sensors(
        "PawnIOLib.dll not found".to_string(),
      ),
    ]);
  }

  fn pending_dashboard_keys(guidance: &ExternalComponentGuidanceState) -> Vec<String> {
    guidance
      .pending_candidates(ExternalComponentGuidanceView::Dashboard, &[])
      .into_iter()
      .map(|candidate| candidate.key)
      .collect()
  }

  #[test]
  fn a_successful_install_defers_the_component_guidance_for_the_session() {
    let _serialized = serialized();
    let platform = FakePlatform(ElevatedProcessRun::Exited { exit_code: Some(0) });
    let guidance = guidance();
    record_cached_pawnio_candidates(&guidance);

    let result = run_with(&platform, ExternalComponent::Pawnio, &guidance).unwrap();

    assert_eq!(result.outcome, ExternalComponentSetupOutcome::Installed);
    // The next poll re-records the still-cached candidates; they stay hidden.
    record_cached_pawnio_candidates(&guidance);
    assert!(pending_dashboard_keys(&guidance).is_empty());
  }

  #[test]
  fn an_install_that_needs_a_reboot_defers_the_component_guidance_for_the_session() {
    let _serialized = serialized();
    let exit_code = core_setup::ExternalComponentSetupOutcome::RebootRequired.exit_code();
    let platform = FakePlatform(ElevatedProcessRun::Exited {
      exit_code: Some(exit_code),
    });
    let guidance = guidance();
    record_cached_pawnio_candidates(&guidance);

    let result = run_with(&platform, ExternalComponent::Pawnio, &guidance).unwrap();

    assert_eq!(
      result.outcome,
      ExternalComponentSetupOutcome::RebootRequired
    );
    assert!(pending_dashboard_keys(&guidance).is_empty());
  }

  #[test]
  fn a_failed_install_leaves_the_component_guidance_pending() {
    let _serialized = serialized();
    let exit_code = core_setup::SetupFailureStage::InstallerExit.exit_code();
    let platform = FakePlatform(ElevatedProcessRun::Exited {
      exit_code: Some(exit_code),
    });
    let guidance = guidance();
    record_cached_pawnio_candidates(&guidance);

    let result = run_with(&platform, ExternalComponent::Pawnio, &guidance).unwrap();

    assert_eq!(result.outcome, ExternalComponentSetupOutcome::Failed);
    assert_eq!(
      pending_dashboard_keys(&guidance),
      vec![
        "pawnio:cpu-package-temperature:v1".to_string(),
        "pawnio:motherboard-sensors:v1".to_string(),
      ]
    );
  }

  #[test]
  fn a_confirmed_timeout_frees_the_component_for_a_retry() {
    let _serialized = serialized();
    let platform = FakePlatform(ElevatedProcessRun::TimedOut);

    let result = run_with(&platform, ExternalComponent::Pawnio, &guidance()).unwrap();

    assert_eq!(result.outcome, ExternalComponentSetupOutcome::Failed);
    assert_eq!(
      result.failure_stage,
      Some(ExternalComponentSetupFailureStage::SetupTimedOut)
    );
    let retry = run_with(&platform, ExternalComponent::Pawnio, &guidance());
    assert!(retry.is_ok(), "the guard was released: {retry:?}");
  }

  #[test]
  fn an_unconfirmed_stop_keeps_the_component_in_flight_and_refuses_a_retry() {
    let _serialized = serialized();
    let platform = FakePlatform(ElevatedProcessRun::StillRunning);

    let result = run_with(&platform, ExternalComponent::Pawnio, &guidance()).unwrap();

    assert_eq!(result.outcome, ExternalComponentSetupOutcome::Failed);
    assert_eq!(
      result.failure_stage,
      Some(ExternalComponentSetupFailureStage::SetupStillRunning)
    );
    assert!(
      IN_FLIGHT
        .lock()
        .unwrap()
        .contains(&ExternalComponent::Pawnio)
    );
    let retry = run_with(
      &FakePlatform(ElevatedProcessRun::Exited { exit_code: Some(0) }),
      ExternalComponent::Pawnio,
      &guidance(),
    );
    assert_eq!(
      retry.unwrap_err(),
      "External Component Setup for pawnio is already running"
    );
    // Release the process-global mark so the other tests in this binary are
    // not affected by the order they run in.
    IN_FLIGHT
      .lock()
      .unwrap()
      .retain(|component| *component != ExternalComponent::Pawnio);
  }

  #[test]
  fn an_unconfirmed_installer_stop_keeps_the_component_in_flight_and_refuses_a_retry() {
    let _serialized = serialized();
    let exit_code = core_setup::SetupFailureStage::InstallerStillRunning.exit_code();
    let platform = FakePlatform(ElevatedProcessRun::Exited {
      exit_code: Some(exit_code),
    });

    let result = run_with(&platform, ExternalComponent::Pawnio, &guidance()).unwrap();

    assert_eq!(result.outcome, ExternalComponentSetupOutcome::Failed);
    assert_eq!(
      result.failure_stage,
      Some(ExternalComponentSetupFailureStage::InstallerStillRunning)
    );
    assert!(
      IN_FLIGHT
        .lock()
        .unwrap()
        .contains(&ExternalComponent::Pawnio)
    );
    let retry = run_with(
      &FakePlatform(ElevatedProcessRun::Exited { exit_code: Some(0) }),
      ExternalComponent::Pawnio,
      &guidance(),
    );
    assert_eq!(
      retry.unwrap_err(),
      "External Component Setup for pawnio is already running"
    );
    IN_FLIGHT
      .lock()
      .unwrap()
      .retain(|component| *component != ExternalComponent::Pawnio);
  }

  #[test]
  fn an_ordinary_installer_failure_frees_the_component_for_a_retry() {
    let _serialized = serialized();
    let exit_code = core_setup::SetupFailureStage::InstallerExit.exit_code();
    let platform = FakePlatform(ElevatedProcessRun::Exited {
      exit_code: Some(exit_code),
    });

    let result = run_with(&platform, ExternalComponent::Pawnio, &guidance()).unwrap();

    assert_eq!(
      result.failure_stage,
      Some(ExternalComponentSetupFailureStage::InstallerExit)
    );
    assert!(run_with(&platform, ExternalComponent::Pawnio, &guidance()).is_ok());
  }

  #[cfg(not(target_os = "windows"))]
  #[test]
  fn run_reports_unsupported_platform_without_launching_anything() {
    use crate::models::external_component_setup::{
      ExternalComponentSetupFailureStage, ExternalComponentSetupOutcome,
      ExternalComponentSetupSupport,
    };

    let result = run(ExternalComponent::Pawnio, &guidance()).unwrap();

    assert_eq!(result.outcome, ExternalComponentSetupOutcome::Failed);
    assert_eq!(
      result.failure_stage,
      Some(ExternalComponentSetupFailureStage::UnsupportedPlatform)
    );
    assert_eq!(
      result.status.support,
      ExternalComponentSetupSupport::UnsupportedPlatform
    );
  }

  #[test]
  fn smartctl_has_no_plan() {
    assert!(status(ExternalComponent::Smartctl).is_err());
  }
}
