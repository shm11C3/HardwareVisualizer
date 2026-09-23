use hardviz_core::enums::error::PlatformError;
use hardviz_core::platform::factory::PlatformFactory;
use hardviz_core::platform::traits::ElevationAvailability;
use tauri::Manager;

pub async fn restart_app(app_handle: &tauri::AppHandle) {
  // Get current executable file path
  let exe_path = std::env::current_exe().expect("Failed to obtain executable file path");
  let args: Vec<String> = std::env::args().collect();

  // Spawn new process
  #[allow(clippy::zombie_processes)]
  std::process::Command::new(exe_path)
    .args(args)
    .spawn()
    .expect("Failed to restart process");

  let state = app_handle.state::<crate::workers::WorkersState>();
  state.terminate_all().await;

  app_handle.exit(0);
}

pub async fn restart_app_elevated(
  app_handle: &tauri::AppHandle,
) -> Result<(), PlatformError> {
  // Refuse before shutting anything down: a refused relaunch must leave the
  // running app fully working (#2216).
  ensure_elevation_available()?;

  let state = app_handle.state::<crate::workers::WorkersState>();
  state.terminate_all().await;

  relaunch_current_process_elevated()?;
  app_handle.exit(0);
  Ok(())
}

pub fn relaunch_for_elevated_startup_if_needed(
  app_handle: &tauri::AppHandle,
) -> Result<bool, PlatformError> {
  if is_process_elevated()? {
    return Ok(false);
  }

  relaunch_current_process_elevated()?;
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

fn ensure_elevation_available() -> Result<(), PlatformError> {
  elevation_unavailable_error(elevation_availability()).map_or(Ok(()), Err)
}

fn elevation_unavailable_error(
  availability: ElevationAvailability,
) -> Option<PlatformError> {
  match availability {
    ElevationAvailability::Available => None,
    ElevationAvailability::UnprotectedLocation => Some(PlatformError::unavailable(
      "Cannot restart as administrator: HardwareVisualizer is not installed under \
       Program Files, so its executable could have been replaced.",
    )),
    ElevationAvailability::Unsupported => Some(PlatformError::unsupported(
      "Elevated Startup Mode is only supported on Windows.",
    )),
  }
}

pub fn relaunch_current_process_elevated() -> Result<(), PlatformError> {
  let platform = PlatformFactory::shared()?;
  let args = std::env::args().skip(1).collect::<Vec<_>>();
  platform.relaunch_current_process_elevated(&args)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_available_elevation_passes_the_precheck() {
    assert!(elevation_unavailable_error(ElevationAvailability::Available).is_none());
    assert!(
      elevation_unavailable_error(ElevationAvailability::UnprotectedLocation).is_some()
    );
    assert!(elevation_unavailable_error(ElevationAvailability::Unsupported).is_some());
  }
}
