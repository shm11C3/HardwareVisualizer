// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![macro_use]

mod commands;
mod constants;
mod enums;
mod infrastructure;
mod models;
mod platform;
mod services;
mod utils;
mod workers;

#[cfg(test)]
mod _tests;

use commands::background_image;
use commands::hardware;
use commands::settings;
use commands::system;
use commands::ui;
use tauri::Manager;
use tauri::Wry;
use tauri_plugin_autostart::MacosLauncher;
use tauri_specta::{Builder, collect_commands};

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use sysinfo::System;

#[cfg(debug_assertions)]
use specta_typescript::Typescript;

pub fn build(config: tauri::Config) -> tauri::Builder<Wry> {
  utils::tauri::init_config(config);

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

  let state = models::hardware::HardwareMonitorState {
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

  let migrations = infrastructure::database::migration::get_migrations();

  let builder = Builder::<tauri::Wry>::new().commands(collect_commands![
    hardware::get_process_list,
    hardware::get_cpu_usage,
    hardware::get_hardware_info,
    hardware::get_memory_info_detail,
    hardware::get_memory_usage,
    hardware::get_gpu_usage,
    hardware::get_processors_usage,
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
    settings::commands::set_burn_in_shift,
    settings::commands::set_burn_in_shift_mode,
    settings::commands::set_burn_in_shift_preset,
    settings::commands::set_burn_in_shift_idle_only,
    settings::commands::set_burn_in_shift_options,
    settings::commands::read_license_file,
    settings::commands::read_third_party_notices_file,
    settings::commands::open_license_file_path,
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
      Typescript::default().header("// @ts-nocheck\n"), // TODO Remove unused imports to eliminate type errors
      //.formatter(specta_typescript::formatter::biome),
      "../src/rspc/bindings.ts",
    )
    .expect("Failed to export typescript bindings");

  tauri::Builder::<Wry>::default()
    .invoke_handler(builder.invoke_handler())
    .setup(move |app| {
      let path_resolver = app.path();
      let handle = app.handle().clone();

      // Initialize logger
      utils::logger::init(path_resolver.app_log_dir().unwrap());

      // Initialize UI
      commands::ui::init(app);

      builder.mount_events(app);

      // Check updates
      tauri::async_runtime::spawn(async move {
        if let Err(e) = workers::updater::update(handle).await {
          log_error!("Update process failed", "lib.run", Some(e.to_string()));
          eprintln!("Update process failed: {e:?}");
        }
      });

      let monitor = workers::system_monitor::SystemMonitorController::setup(
        models::hardware_archive::MonitorResources {
          system: Arc::clone(&system),
          cpu_history: Arc::clone(&cpu_history),
          memory_history: Arc::clone(&memory_history),
          process_cpu_histories: Arc::clone(&process_cpu_histories),
          process_memory_histories: Arc::clone(&process_memory_histories),
          nv_gpu_usage_histories: Arc::clone(&nv_gpu_usage_histories),
          nv_gpu_temperature_histories: Arc::clone(&nv_gpu_temperature_histories),
          nv_gpu_dedicated_memory_histories: Arc::clone(
            &nv_gpu_dedicated_memory_histories,
          ),
        },
      );
      {
        let ws = app.state::<workers::WorkersState>();
        ws.monitor.lock().unwrap().replace(monitor);
      }

      // Start hardware archive service
      if settings.hardware_archive.enabled {
        let hw_archive = workers::hardware_archive::HardwareArchiveController::setup(
          models::hardware_archive::MonitorResources {
            system: Arc::clone(&system),
            cpu_history: Arc::clone(&cpu_history),
            memory_history: Arc::clone(&memory_history),
            process_cpu_histories: Arc::clone(&process_cpu_histories),
            process_memory_histories: Arc::clone(&process_memory_histories),
            nv_gpu_usage_histories: Arc::clone(&nv_gpu_usage_histories),
            nv_gpu_temperature_histories: Arc::clone(&nv_gpu_temperature_histories),
            nv_gpu_dedicated_memory_histories: Arc::clone(
              &nv_gpu_dedicated_memory_histories,
            ),
          },
        );
        {
          let ws = app.state::<workers::WorkersState>();
          ws.hw_archive.lock().unwrap().replace(hw_archive);
        }
      }

      // Start scheduled data deletion
      if settings.hardware_archive.scheduled_data_deletion {
        tauri::async_runtime::spawn(workers::hardware_archive::batch_delete_old_data(
          settings.hardware_archive.refresh_interval_days,
        ));
      }

      Ok(())
    })
    .on_window_event(|win, ev| {
      if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
        api.prevent_close();
        let app = win.app_handle().clone();

        tauri::async_runtime::spawn(async move {
          // Stop all background processing
          let ws = app.state::<workers::WorkersState>();
          ws.terminate_all().await;

          app.exit(0);
        });
      }
    })
    .plugin(tauri_plugin_updater::Builder::new().build())
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
    .plugin(tauri_plugin_clipboard_manager::init())
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_opener::init())
    .manage(state)
    .manage(app_state)
    .manage(workers::WorkersState::default())
}

#[cfg(not(target_os = "macos"))]
pub fn run(config: tauri::Config) -> tauri::Result<()> {
  build(config).run()
}
