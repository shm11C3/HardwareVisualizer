use hardviz_core::platform::factory::PlatformFactory;
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

pub async fn restart_app_elevated(app_handle: &tauri::AppHandle) -> Result<(), String> {
  let state = app_handle.state::<crate::workers::WorkersState>();
  state.terminate_all().await;

  relaunch_current_process_elevated()?;
  app_handle.exit(0);
  Ok(())
}

pub fn relaunch_for_elevated_startup_if_needed(
  app_handle: &tauri::AppHandle,
) -> Result<bool, String> {
  if is_process_elevated()? {
    return Ok(false);
  }

  relaunch_current_process_elevated()?;
  app_handle.exit(0);
  Ok(true)
}

pub fn is_process_elevated() -> Result<bool, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  platform.is_process_elevated()
}

pub fn relaunch_current_process_elevated() -> Result<(), String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  platform.relaunch_current_process_elevated()
}
