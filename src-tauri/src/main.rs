#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  #[cfg(not(target_os = "macos"))]
  {
    hardware_monitor_lib::build()
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
  }

  {
    let context = tauri::generate_context!();
    let config = context.config().clone();

    hardware_monitor_lib::build(config)
      .run(context)
      .expect("error while running tauri application");
  }
}
