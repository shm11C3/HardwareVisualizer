// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[macro_use]

mod commands;
mod enums;
mod services;
mod structs;
mod utils;

#[cfg(test)]
mod tests;

use commands::background_image;
use commands::config;
use commands::hardware;
use commands::ui;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri::Wry;
use tauri_specta::{collect_commands, Builder};

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use sysinfo::System;

pub fn run() {
  let app_state = config::AppState::new();

  let system = Arc::new(Mutex::new(System::new_all()));
  let cpu_history = Arc::new(Mutex::new(VecDeque::with_capacity(60)));
  let memory_history = Arc::new(Mutex::new(VecDeque::with_capacity(60)));
  let gpu_usage = Arc::new(Mutex::new(0.0));
  let gpu_history = Arc::new(Mutex::new(VecDeque::with_capacity(60)));
  let process_cpu_histories = Arc::new(Mutex::new(HashMap::new()));
  let process_memory_histories = Arc::new(Mutex::new(HashMap::new()));

  let state = hardware::AppState {
    system: Arc::clone(&system),
    cpu_history: Arc::clone(&cpu_history),
    memory_history: Arc::clone(&memory_history),
    gpu_usage: Arc::clone(&gpu_usage),
    gpu_history: Arc::clone(&gpu_history),
    process_cpu_histories: Arc::clone(&process_cpu_histories),
    process_memory_histories: Arc::clone(&process_memory_histories),
  };

  hardware::initialize_system(
    system,
    cpu_history,
    memory_history,
    gpu_usage,
    gpu_history,
    process_cpu_histories,
    process_memory_histories,
  );

  let builder = Builder::<tauri::Wry>::new().commands(collect_commands![
    hardware::get_process_list,
    hardware::get_cpu_usage,
    hardware::get_hardware_info,
    hardware::get_memory_usage,
    hardware::get_gpu_usage,
    hardware::get_gpu_temperature,
    hardware::get_nvidia_gpu_cooler,
    hardware::get_cpu_usage_history,
    hardware::get_memory_usage_history,
    hardware::get_gpu_usage_history,
    config::commands::get_settings,
    config::commands::set_language,
    config::commands::set_theme,
    config::commands::set_display_targets,
    config::commands::set_graph_size,
    config::commands::set_line_graph_border,
    config::commands::set_line_graph_fill,
    config::commands::set_line_graph_color,
    config::commands::set_line_graph_mix,
    config::commands::set_line_graph_show_legend,
    config::commands::set_line_graph_show_scale,
    config::commands::set_background_img_opacity,
    config::commands::set_selected_background_img,
    background_image::get_background_image,
    background_image::get_background_images,
    background_image::save_background_image,
    background_image::delete_background_image,
    ui::set_decoration,
  ]);

  // TS bindings
  #[cfg(debug_assertions)]
  builder
    .export(
      Typescript::default().header("// @ts-nocheck\n"), // TODO 未使用なimportを削除して型エラーをなくす
      //.formatter(specta_typescript::formatter::biome),
      "../src/rspc/bindings.ts",
    )
    .expect("Failed to export typescript bindings");

  tauri::Builder::<Wry>::default()
    .invoke_handler(builder.invoke_handler())
    .setup(move |app| {
      let path_resolver = app.path();

      // ロガーの初期化
      utils::logger::init(path_resolver.app_log_dir().unwrap());

      // UIの初期化
      commands::ui::init(app);

      builder.mount_events(app);

      Ok(())
    })
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .manage(state)
    .manage(app_state)
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
