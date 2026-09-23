use crate::cli;
use crate::workers::WorkersState;
use hardviz_core::enums::error::PlatformError;
use hardviz_core::platform::factory::PlatformFactory;
use hardviz_core::platform::traits::{ElevationAvailability, ProcessElevationPlatform};
use std::sync::Arc;
use tauri::Manager;

pub async fn restart_app(app_handle: &tauri::AppHandle) {
  // Get current executable file path
  let exe_path = std::env::current_exe().expect("Failed to obtain executable file path");

  // Spawn new process. It waits for this one to exit (`cli::restart_args`)
  // so it neither opens the database beside this process's live owner nor
  // exits as a second instance while this process is still draining.
  #[allow(clippy::zombie_processes)]
  std::process::Command::new(exe_path)
    .args(cli::restart_args())
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
  hand_off_to_elevated_process(platform, &workers).await?;
  app_handle.exit(0);
  Ok(())
}

/// Launch the elevated child before anything stops, so a declined UAC prompt
/// or a failed launch returns the error with every worker still running
/// (#2216 follow-up). The launch can go first only because the child waits
/// for this process to exit (`cli::WAIT_FOR_PARENT_FLAG`) before it takes the
/// single-instance lock or opens the database; the elevated launch itself
/// refuses an executable outside Program Files before it prompts. The launch
/// blocks until the prompt is answered, and the workers are still producing
/// updates meanwhile, so it runs on the blocking pool rather than on a
/// runtime worker thread.
async fn hand_off_to_elevated_process<P>(
  platform: Arc<P>,
  workers: &WorkersState,
) -> Result<(), PlatformError>
where
  P: ProcessElevationPlatform + ?Sized + 'static,
{
  let this_process = platform.current_process_identity()?;
  let args = cli::elevated_handoff_args(&this_process);
  tauri::async_runtime::spawn_blocking(move || {
    platform.relaunch_current_process_elevated(&args)
  })
  .await
  .map_err(|e| {
    PlatformError::fault(format!("The elevated launch did not complete: {e}"))
  })??;
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
  use std::sync::Mutex;
  use std::sync::atomic::Ordering;

  const CREATION_TIME: u64 = 133_800_000_000_000_000;

  /// A platform whose elevated launch succeeds or fails as configured and
  /// records the arguments it was asked to launch.
  struct FakeElevation {
    launch: Result<(), PlatformError>,
    launched_with: Mutex<Option<Vec<String>>>,
  }

  impl FakeElevation {
    fn new(launch: Result<(), PlatformError>) -> Arc<Self> {
      Arc::new(Self {
        launch,
        launched_with: Mutex::new(None),
      })
    }
  }

  impl ProcessElevationPlatform for FakeElevation {
    fn is_process_elevated(&self) -> Result<bool, PlatformError> {
      Ok(false)
    }

    fn relaunch_current_process_elevated(
      &self,
      args: &[String],
    ) -> Result<(), PlatformError> {
      *self.launched_with.lock().unwrap() = Some(args.to_vec());
      self.launch.clone()
    }

    fn elevation_availability(&self) -> ElevationAvailability {
      ElevationAvailability::Available
    }

    fn current_process_identity(&self) -> Result<ProcessIdentity, PlatformError> {
      Ok(ProcessIdentity {
        pid: std::process::id(),
        creation_time: CREATION_TIME,
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
    let platform = FakeElevation::new(Err(PlatformError::fault(
      "the elevation prompt was declined",
    )));
    let workers = WorkersState::default();

    let result = hand_off_to_elevated_process(Arc::clone(&platform), &workers).await;

    assert!(result.is_err());
    assert!(!workers.shutting_down.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn a_successful_launch_stops_the_workers_and_names_this_process() {
    let platform = FakeElevation::new(Ok(()));
    let workers = WorkersState::default();

    let result = hand_off_to_elevated_process(Arc::clone(&platform), &workers).await;

    assert_eq!(result, Ok(()));
    assert!(workers.shutting_down.load(Ordering::SeqCst));
    let launched_with = platform.launched_with.lock().unwrap().clone();
    let launched_with = launched_with.expect("the launch received arguments");
    assert_eq!(
      &launched_with[launched_with.len() - 2..],
      [
        cli::WAIT_FOR_PARENT_FLAG.to_string(),
        format!("{}:{CREATION_TIME}", std::process::id()),
      ]
    );
  }
}
