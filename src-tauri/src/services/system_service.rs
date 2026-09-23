use crate::cli;
use crate::workers::WorkersState;
use hardviz_core::enums::error::PlatformError;
use hardviz_core::platform::factory::PlatformFactory;
use hardviz_core::platform::traits::{ElevationAvailability, ProcessElevationPlatform};
use tauri::Manager;

pub async fn restart_app(app_handle: &tauri::AppHandle) {
  // Get current executable file path
  let exe_path = std::env::current_exe().expect("Failed to obtain executable file path");

  // Spawn new process
  #[allow(clippy::zombie_processes)]
  std::process::Command::new(exe_path)
    .args(cli::relaunch_args())
    .spawn()
    .expect("Failed to restart process");

  let state = app_handle.state::<WorkersState>();
  state.terminate_all().await;

  app_handle.exit(0);
}

pub async fn restart_app_elevated(
  app_handle: &tauri::AppHandle,
) -> Result<(), PlatformError> {
  let platform = PlatformFactory::shared()?;
  let workers = app_handle.state::<WorkersState>();
  hand_off_to_elevated_process(platform.as_ref(), &workers).await?;
  app_handle.exit(0);
  Ok(())
}

/// Launch the elevated child before anything stops, so a declined UAC prompt
/// or a failed launch returns the error with every worker still running
/// (#2216 follow-up). The launch can go first only because the child waits
/// for this process to exit (`cli::WAIT_FOR_PARENT_FLAG`) before it takes the
/// single-instance lock or opens the database; the elevated launch itself
/// refuses an executable outside Program Files before it prompts.
async fn hand_off_to_elevated_process(
  platform: &dyn ProcessElevationPlatform,
  workers: &WorkersState,
) -> Result<(), PlatformError> {
  let this_process = platform.current_process_identity()?;
  platform
    .relaunch_current_process_elevated(&cli::elevated_handoff_args(&this_process))?;
  workers.terminate_all().await;
  Ok(())
}

/// The same handoff at startup: no worker is running yet inside `setup`, but
/// this process already holds the single-instance lock and, once the native
/// database is selected, its live owner, so the child must still wait for
/// the exit below before it starts.
pub fn relaunch_for_elevated_startup_if_needed(
  app_handle: &tauri::AppHandle,
) -> Result<bool, PlatformError> {
  if is_process_elevated()? {
    return Ok(false);
  }

  let platform = PlatformFactory::shared()?;
  let this_process = platform.current_process_identity()?;
  platform
    .relaunch_current_process_elevated(&cli::elevated_handoff_args(&this_process))?;
  app_handle.exit(0);
  Ok(true)
}

pub fn is_process_elevated() -> Result<bool, PlatformError> {
  let platform = PlatformFactory::shared()?;
  platform.is_process_elevated()
}

/// Whether this installation can be launched elevated. A platform that
/// cannot be resolved cannot elevate either.
pub fn elevation_availability() -> ElevationAvailability {
  PlatformFactory::shared()
    .map(|platform| platform.elevation_availability())
    .unwrap_or(ElevationAvailability::Unsupported)
}

#[cfg(test)]
mod tests {
  use super::*;
  use hardviz_core::platform::traits::{ProcessExitWait, ProcessIdentity};
  use std::sync::atomic::Ordering;

  /// A platform whose elevated launch succeeds or fails as configured.
  struct FakeElevation {
    launch: Result<(), PlatformError>,
  }

  impl ProcessElevationPlatform for FakeElevation {
    fn is_process_elevated(&self) -> Result<bool, PlatformError> {
      Ok(false)
    }

    fn relaunch_current_process_elevated(
      &self,
      _args: &[String],
    ) -> Result<(), PlatformError> {
      self.launch.clone()
    }

    fn elevation_availability(&self) -> ElevationAvailability {
      ElevationAvailability::Available
    }

    fn current_process_identity(&self) -> Result<ProcessIdentity, PlatformError> {
      Ok(ProcessIdentity {
        pid: 1,
        creation_time: 1,
      })
    }

    fn wait_for_process_exit(
      &self,
      _identity: &ProcessIdentity,
    ) -> Result<ProcessExitWait, PlatformError> {
      Ok(ProcessExitWait::Exited)
    }
  }

  #[tokio::test]
  async fn a_declined_or_failed_launch_leaves_the_workers_running() {
    let platform = FakeElevation {
      launch: Err(PlatformError::fault("the elevation prompt was declined")),
    };
    let workers = WorkersState::default();

    let result = hand_off_to_elevated_process(&platform, &workers).await;

    assert!(result.is_err());
    assert!(!workers.shutting_down.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn a_successful_launch_stops_the_workers() {
    let platform = FakeElevation { launch: Ok(()) };
    let workers = WorkersState::default();

    let result = hand_off_to_elevated_process(&platform, &workers).await;

    assert_eq!(result, Ok(()));
    assert!(workers.shutting_down.load(Ordering::SeqCst));
  }
}
