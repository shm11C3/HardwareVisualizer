#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  let context = tauri::generate_context!();

  #[cfg(target_os = "macos")]
  let app = hardware_monitor_lib::build(context.config().clone());

  #[cfg(not(target_os = "macos"))]
  let app = hardware_monitor_lib::build();

  app
    .run(context)
    .expect("error while running tauri application");
}
