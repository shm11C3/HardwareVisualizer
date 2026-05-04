// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![macro_use]

// Re-export the logging macros from `hardviz_core` so existing
// `use crate::{log_internal, log_warn};` sites keep compiling. The macros
// themselves expand to `tracing::*` calls and live in `hardviz_core::utils::logger`.
pub use hardviz_core::{log_debug, log_error, log_info, log_internal, log_warn};

mod adapters;
mod app;
mod commands;
mod enums;
mod infrastructure;
mod lifecycle;
mod models;
mod services;
mod tray;
mod utils;
mod workers;

#[cfg(test)]
mod _tests;

use commands::background_image;
use commands::hardware;
use commands::settings;
use commands::system;
use commands::ui;
use commands::updater::app_updates;
use hardviz_core::collector::HistoryStore;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri::Wry;
use tauri_plugin_autostart::MacosLauncher;
use tauri_specta::{Builder, collect_commands, collect_events};

#[cfg(debug_assertions)]
use specta_typescript::Typescript;

pub fn run() {
  let app_state = settings::AppState::new();

  // Core-owned shared sensor history. App-side commands and the collector
  // loop read/write through this store. Persistence no longer shares it —
  // the archive worker subscribes to the EventBus instead (#1407).
  let history_store = Arc::new(HistoryStore::new());

  let core_settings = app_state.core_settings.lock().unwrap().clone();

  let db_path = utils::file::get_app_data_dir("hv-database.db");
  // Initialize Core's DB pool location once at process start. Core
  // can't resolve the bundle identifier on its own, so App owns path
  // resolution and hands the file path to
  // `hardviz_core::infrastructure::database::db`. We don't care about
  // the return value here: this is the first and only caller during
  // App startup.
  let _ = hardviz_core::infrastructure::database::db::init(db_path.clone());

  let app_max_version = infrastructure::database::migration::get_max_migration_version();
  let db_error = hardviz_core::persistence::preflight::check_db_compatibility(
    &db_path,
    app_max_version,
  );
  let is_db_ok = db_error.is_none();

  let migrations = infrastructure::database::migration::get_migrations();

  let builder = Builder::<tauri::Wry>::new()
    .events(collect_events![models::hardware::HardwareMonitorUpdate,])
    .commands(collect_commands![
      app_updates::fetch_update,
      app_updates::install_update,
      hardware::get_process_list,
      hardware::get_cpu_usage,
      hardware::get_hardware_info,
      hardware::get_memory_info_detail,
      hardware::get_memory_usage,
      hardware::get_gpu_usage,
      hardware::get_processors_usage,
      hardware::get_gpu_temperature,
      hardware::get_cpu_usage_history,
      hardware::get_memory_usage_history,
      hardware::get_gpu_usage_history,
      hardware::get_network_info,
      hardware::get_gpu_memory_usage,
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
      settings::commands::set_text_selectable,
      settings::commands::read_license_file,
      settings::commands::read_third_party_notices_file,
      settings::commands::open_license_file_path,
      background_image::get_background_image,
      background_image::get_background_images,
      background_image::save_background_image,
      background_image::delete_background_image,
      ui::set_decoration,
      system::restart_app,
      system::quit_app,
      system::is_close_to_tray_available,
      system::mark_close_to_tray_listener_ready,
    ]);

  // TS bindings
  #[cfg(debug_assertions)]
  builder
    .export(Typescript::default(), "../src/rspc/bindings.ts")
    .expect("Failed to export typescript bindings");

  let store_for_setup = Arc::clone(&history_store);

  let mut tauri_builder = tauri::Builder::<Wry>::default()
    .invoke_handler(builder.invoke_handler())
    .setup(move |app| {
      let path_resolver = app.path();

      // Initialize logger
      utils::logger::init(path_resolver.app_log_dir().unwrap());

      // Initialize UI and real-time monitoring (independent of DB)
      commands::ui::init(app);
      builder.mount_events(app);

      // Real-time pipeline: collector publishes MetricsSnapshot to the
      // EventBus, WindowAdapter subscribes and emits HardwareMonitorUpdate.
      let bus = hardviz_core::event_bus::EventBus::new();
      let window_adapter =
        adapters::window::WindowAdapter::setup(app.handle().clone(), bus.subscribe());

      // Run the Core collector on Tauri's tokio runtime. Core has no
      // `tauri` dep, so it can't reach Tauri's static runtime directly —
      // we hand it the inner `tokio::runtime::Handle`.
      let runtime_handle = tauri::async_runtime::handle().inner().clone();
      let monitor = hardviz_core::collector::SystemMonitorController::setup(
        Arc::clone(&store_for_setup),
        bus.clone(),
        runtime_handle.clone(),
      );
      {
        let ws = app.state::<workers::WorkersState>();
        ws.monitor.lock().unwrap().replace(monitor);
        ws.window_adapter.lock().unwrap().replace(window_adapter);
      }

      // #1422: register the tray unconditionally. The visibility
      // policy (always-on vs persisted user setting) is a UX call owned
      // by #1423 and lives outside the adapter.
      match adapters::tray::TrayAdapter::setup(app, bus.subscribe()) {
        Ok(tray) => {
          let ws = app.state::<workers::WorkersState>();
          ws.tray.lock().unwrap().replace(tray);
        }
        Err(e) => {
          app
            .state::<lifecycle::CloseToTrayRuntimeState>()
            .disable_for_session();
          // Linux desktops without an indicator implementation, or any
          // other tray failure: log and continue. Close-to-tray is
          // disabled for this session so the close button still quits.
          log_warn!(
            &format!("tray icon setup failed; continuing without tray: {e}"),
            "lib::setup",
            None::<&str>
          );
        }
      }

      if is_db_ok {
        // Start DB-dependent archive services. Persistence subscribes to
        // the EventBus so a slow DB write can't back-pressure the
        // collector cadence (#1407).
        if core_settings.hardware_archive.enabled {
          let hw_archive = hardviz_core::persistence::ArchiveController::setup(
            &bus,
            runtime_handle.clone(),
          );
          {
            let ws = app.state::<workers::WorkersState>();
            ws.hw_archive.lock().unwrap().replace(hw_archive);
          }
        }

        // Retention cleanup runs once per process boot — the pre-Phase-4
        // `batch_delete_old_data` wrapper had the same one-shot semantics.
        // The setting name `scheduled_data_deletion` is historical; the
        // refresh trigger is "next app launch", not a recurring schedule.
        // See `hardviz_core::persistence::cleanup_old_data` doc comment.
        if core_settings.hardware_archive.scheduled_data_deletion {
          tauri::async_runtime::spawn(hardviz_core::persistence::cleanup_old_data(
            core_settings.hardware_archive.refresh_interval_days,
          ));
        }
      } else {
        // Database schema is incompatible — show error dialog
        // Hide window while dialog is shown, then restore based on user choice
        if let Some(window) = app.get_webview_window("main") {
          let _ = window.hide();
        }

        let handle = app.handle().clone();
        let db_err = db_error.expect("db_error must be Some when is_db_ok is false");
        std::thread::spawn(move || {
          use app::startup::{self, StartupErrorAction};
          match startup::prompt_startup_error(&handle, db_err) {
            StartupErrorAction::ResetAndRestart => {
              startup::reset_database_and_restart(&handle);
            }
            StartupErrorAction::ContinueAnyway => {
              // Show the main window — app runs without DB-backed features
              if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
              }
            }
            StartupErrorAction::Exit => handle.exit(1),
          }
        });
      }

      Ok(())
    })
    .on_window_event(|win, ev| {
      if win.label() == tray::TRAY_WIDGET_FLYOUT_LABEL {
        match ev {
          tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = win.hide();
          }
          tauri::WindowEvent::Focused(false) => {
            let _ = win.hide();
          }
          _ => {}
        }
        return;
      }

      if win.label() == "main"
        && let tauri::WindowEvent::CloseRequested { api, .. } = ev
      {
        // Always prevent the default close so lifecycle picks the
        // outcome (hide vs. quit) deterministically — see
        // `lifecycle::handle_close_request` for the policy.
        api.prevent_close();
        lifecycle::on_close_requested(win);
      }
    })
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_autostart::init(
      MacosLauncher::LaunchAgent,
      Some(vec![]),
    ))
    .plugin(tauri_plugin_clipboard_manager::init())
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_opener::init())
    .manage(history_store)
    .manage(app_state)
    .manage(lifecycle::CloseToTrayRuntimeState::default())
    .manage(workers::WorkersState::default())
    .manage(app_updates::PendingUpdate(Mutex::new(None)));

  if is_db_ok {
    tauri_builder = tauri_builder.plugin(
      tauri_plugin_sql::Builder::new()
        .add_migrations("sqlite:hv-database.db", migrations)
        .build(),
    );
  }

  tauri_builder
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
