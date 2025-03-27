// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![macro_use]

mod backgrounds;
mod commands;
mod database;
mod enums;
mod services;
mod structs;
mod utils;

#[cfg(test)]
mod _tests;

use backgrounds::system_monitor::MonitorResources;
use commands::background_image;
use commands::hardware;
use commands::settings;
use commands::system;
use commands::ui;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri::Wry;
use tauri_plugin_autostart::MacosLauncher;
use tauri_specta::{Builder, collect_commands};

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use sysinfo::System;

pub fn run() {
  let app_state = settings::AppState::new();

  let system = Arc::new(Mutex::new(System::new_all()));
  let cpu_history = Arc::new(Mutex::new(VecDeque::with_capacity(60)));
  let memory_history = Arc::new(Mutex::new(VecDeque::with_capacity(60)));
  let gpu_history = Arc::new(Mutex::new(VecDeque::with_capacity(60)));
  let process_cpu_histories = Arc::new(Mutex::new(HashMap::new()));
  let process_memory_histories = Arc::new(Mutex::new(HashMap::new()));
  let nv_gpu_usage_histories = Arc::new(Mutex::new(HashMap::new()));
  let nv_gpu_temperature_histories = Arc::new(Mutex::new(HashMap::new()));
  let nv_gpu_dedicated_memory_histories = Arc::new(Mutex::new(HashMap::new()));

  let state = structs::hardware::HardwareMonitorState {
    system: Arc::clone(&system),
    cpu_history: Arc::clone(&cpu_history),
    memory_history: Arc::clone(&memory_history),
    gpu_history: Arc::clone(&gpu_history),
    process_cpu_histories: Arc::clone(&process_cpu_histories),
    process_memory_histories: Arc::clone(&process_memory_histories),
    nv_gpu_usage_histories: Arc::clone(&nv_gpu_usage_histories),
    nv_gpu_temperature_histories: Arc::clone(&nv_gpu_temperature_histories),
  };

  let settings = app_state.settings.lock().unwrap().clone();

  let migrations = database::migration::get_migrations();

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
    hardware::get_network_info,
    settings::commands::get_settings,
    settings::commands::set_language,
    settings::commands::set_theme,
    settings::commands::set_display_targets,
    settings::commands::set_graph_size,
    settings::commands::set_line_graph_type,
    settings::commands::set_line_graph_border,
    settings::commands::set_line_graph_fill,
    settings::commands::set_line_graph_color,
    settings::commands::set_line_graph_mix,
    settings::commands::set_line_graph_show_legend,
    settings::commands::set_line_graph_show_scale,
    settings::commands::set_line_graph_show_tooltip,
    settings::commands::set_background_img_opacity,
    settings::commands::set_selected_background_img,
    settings::commands::set_temperature_unit,
    settings::commands::set_hardware_archive_enabled,
    settings::commands::set_hardware_archive_interval,
    settings::commands::set_hardware_archive_scheduled_data_deletion,
    background_image::get_background_image,
    background_image::get_background_images,
    background_image::save_background_image,
    background_image::delete_background_image,
    ui::set_decoration,
    system::restart_app,
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

      tauri::async_runtime::spawn(backgrounds::system_monitor::setup(MonitorResources {
        system: Arc::clone(&system),
        cpu_history: Arc::clone(&cpu_history),
        memory_history: Arc::clone(&memory_history),
        process_cpu_histories: Arc::clone(&process_cpu_histories),
        process_memory_histories: Arc::clone(&process_memory_histories),
        nv_gpu_usage_histories: Arc::clone(&nv_gpu_usage_histories),
        nv_gpu_temperature_histories: Arc::clone(&nv_gpu_temperature_histories),
        nv_gpu_dedicated_memory_histories: Arc::clone(&nv_gpu_dedicated_memory_histories),
      }));

      // ハードウェアアーカイブサービスの開始
      if settings.hardware_archive.enabled {
        tauri::async_runtime::spawn(backgrounds::hardware_archive::setup(
          Arc::clone(&cpu_history),
          Arc::clone(&memory_history),
          Arc::clone(&nv_gpu_usage_histories),
          Arc::clone(&nv_gpu_temperature_histories),
          Arc::clone(&nv_gpu_dedicated_memory_histories),
        ));
      }

      // スケジュールされたデータ削除の開始
      if settings.hardware_archive.scheduled_data_deletion {
        tauri::async_runtime::spawn(
          backgrounds::hardware_archive::batch_delete_old_data(
            settings.hardware_archive.refresh_interval_days,
          ),
        );
      }

      Ok(())
    })
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .plugin(tauri_plugin_shell::init())
    .plugin(
      tauri_plugin_sql::Builder::new()
        .add_migrations("sqlite:hv-database.db", migrations)
        .build(),
    )
    .plugin(tauri_plugin_autostart::init(
      MacosLauncher::LaunchAgent,
      Some(vec![]),
    ))
    .manage(state)
    .manage(app_state)
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
