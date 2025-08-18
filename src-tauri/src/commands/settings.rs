use crate::enums;
use crate::models;
use crate::services;
use crate::utils;
use crate::{log_error, log_internal};
use tauri_plugin_opener::OpenerExt;

#[derive(Debug)]
pub struct AppState {
  pub settings: std::sync::Mutex<models::settings::Settings>,
}

impl AppState {
  pub fn new() -> Self {
    Self {
      settings: std::sync::Mutex::from(models::settings::Settings::new()),
    }
  }
}

pub mod commands {

  use super::*;
  use serde_json::json;
  use tauri::{Emitter, EventTarget, Manager, Window};

  const ERROR_TITLE: &str = "Failed to update settings file";

  ///
  /// ## エラーイベントを発生させフロントエンドに通知する
  ///
  /// [TODO] dialog を使ってエラーメッセージを表示する
  ///
  fn emit_error(window: &Window) -> Result<(), String> {
    let settings_json_path =
      utils::file::get_app_data_dir(services::settings_service::SETTINGS_FILENAME);

    log_error!(
      "Failed to update settings file",
      "settings.rs",
      Some(settings_json_path.display().to_string())
    );

    window
      .emit_to(
        EventTarget::window(window.label().to_string()),
        "error_event",
        json!({
            "title": ERROR_TITLE,
            "message": format!("If this happens repeatedly, delete {settings_json_path} and restart the app.", settings_json_path = settings_json_path.display())
        }),
      )
      .map_err(|e| format!("Failed to emit event: {e}"))?;

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn get_settings(
    state: tauri::State<'_, AppState>,
  ) -> Result<models::settings::ClientSettings, String> {
    let settings = state.settings.lock().unwrap().clone();

    // フロントで扱いやすいようにカンマ区切りの文字列に変換する
    let color_strings = models::settings::LineGraphColorStringSettings {
      cpu: settings
        .line_graph_color
        .cpu
        .iter()
        .map(|&c| c.to_string())
        .collect::<Vec<String>>()
        .join(","),
      memory: settings
        .line_graph_color
        .memory
        .iter()
        .map(|&c| c.to_string())
        .collect::<Vec<String>>()
        .join(","),
      gpu: settings
        .line_graph_color
        .gpu
        .iter()
        .map(|&c| c.to_string())
        .collect::<Vec<String>>()
        .join(","),
    };

    let client_settings = models::settings::ClientSettings {
      version: settings.version,
      language: settings.language,
      theme: settings.theme,
      display_targets: settings.display_targets,
      graph_size: settings.graph_size,
      line_graph_type: settings.line_graph_type,
      line_graph_border: settings.line_graph_border,
      line_graph_fill: settings.line_graph_fill,
      line_graph_color: color_strings,
      line_graph_mix: settings.line_graph_mix,
      line_graph_show_legend: settings.line_graph_show_legend,
      line_graph_show_scale: settings.line_graph_show_scale,
      line_graph_show_tooltip: settings.line_graph_show_tooltip,
      background_img_opacity: settings.background_img_opacity,
      selected_background_img: settings.selected_background_img,
      temperature_unit: settings.temperature_unit,
      hardware_archive: settings.hardware_archive,
    };

    Ok(client_settings)
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_language(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_language: String,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_language(new_language) {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_theme(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_theme: enums::settings::Theme,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_theme(new_theme) {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_display_targets(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_targets: Vec<enums::hardware::HardwareType>,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_display_targets(new_targets) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_graph_size(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_size: enums::settings::GraphSize,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_graph_size(new_size) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_type(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_type: enums::settings::LineGraphType,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_type(new_type) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_border(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_border(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_fill(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_fill(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_color(
    window: Window,
    state: tauri::State<'_, AppState>,
    target: enums::hardware::HardwareType,
    new_color: String,
  ) -> Result<String, String> {
    let mut settings = state.settings.lock().unwrap();

    match settings.set_line_graph_color(target, new_color) {
      Ok(result) => Ok(result),
      Err(e) => {
        emit_error(&window)?;
        Err(e)
      }
    }
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_mix(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_mix(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_show_legend(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_show_legend(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_show_scale(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_show_scale(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_show_tooltip(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_show_tooltip(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_background_img_opacity(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: u8,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_background_img_opacity(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_selected_background_img(
    window: Window,
    state: tauri::State<'_, AppState>,
    file_id: Option<String>,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_selected_background_img(file_id) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_temperature_unit(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_unit: enums::settings::TemperatureUnit,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_temperature_unit(new_unit) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_hardware_archive_enabled(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_hardware_archive_enabled(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_hardware_archive_interval(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_interval: u32,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_hardware_archive_interval(new_interval) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_hardware_archive_scheduled_data_deletion(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_hardware_archive_scheduled_data_deletion(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn read_license_file(app: tauri::AppHandle) -> Result<String, String> {
    let resource_path = app
      .path()
      .resource_dir()
      .map_err(|e| format!("Failed to get resource directory: {}", e))?
      .join("LICENSE");

    std::fs::read_to_string(&resource_path)
      .map_err(|e| format!("Failed to read LICENSE file: {}", e))
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn read_third_party_notices_file(
    app: tauri::AppHandle,
  ) -> Result<String, String> {
    let resource_path = app
      .path()
      .resource_dir()
      .map_err(|e| format!("Failed to get resource directory: {}", e))?
      .join("THIRD_PARTY_NOTICES.md");

    std::fs::read_to_string(&resource_path)
      .map_err(|e| format!("Failed to read THIRD_PARTY_NOTICES.md file: {}", e))
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn open_license_file_path(app: tauri::AppHandle) -> Result<(), String> {
    let resource_path = app
      .path()
      .resource_dir()
      .map_err(|e| format!("Failed to get resource directory: {}", e));

    let path_str = resource_path?
      .to_str()
      .ok_or_else(|| "Failed to convert path to string".to_string())?
      .to_string();

    app
      .opener()
      .open_path(path_str, None::<&str>)
      .map_err(|e| format!("Failed to open LICENSE file path: {}", e))
  }
}
