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
