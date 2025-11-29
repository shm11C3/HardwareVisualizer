#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  let context = tauri::generate_context!();
  let config = context.config().clone();

  hardware_monitor_lib::build(config)
    .run(context)
    .expect("error while running tauri application");
}
