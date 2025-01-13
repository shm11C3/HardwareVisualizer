use crate::enums;
use crate::structs;
use crate::utils;
use crate::{log_error, log_internal};
use std::fs;
use std::io::Write;
use std::sync::Mutex;
use tempfile::NamedTempFile;

const SETTINGS_FILENAME: &str = "settings.json";

pub trait Config {
  fn write_file(&self) -> Result<(), String>;
  fn read_file(&mut self) -> Result<(), String>;
}

impl Config for structs::settings::Settings {
  fn write_file(&self) -> Result<(), String> {
    let config_file = utils::file::get_app_data_dir(SETTINGS_FILENAME);
    let config_dir = match config_file.parent() {
      Some(dir) => dir,
      None => {
        log_error!(
          "Failed to get parent directory for settings file",
          "write_file",
          None::<&str>
        );
        return Err("Failed to get parent directory for settings file".to_string());
      }
    };

    if !config_dir.exists() {
      if let Err(e) = fs::create_dir_all(config_dir) {
        log_error!(
          "Failed to create configuration directory",
          "write_file",
          Some(e.to_string())
        );
        return Err(format!("Failed to create configuration directory: {}", e));
      }
    }

    let serialized = match serde_json::to_string(self) {
      Ok(s) => s,
      Err(e) => {
        log_error!(
          "Failed to serialize settings",
          "write_file",
          Some(e.to_string())
        );
        return Err(format!("Failed to serialize settings: {}", e));
      }
    };

    // 一時ファイルに書き込む
    let mut temp_file = match NamedTempFile::new_in(config_dir) {
      Ok(file) => file,
      Err(e) => {
        log_error!(
          "Failed to create temporary file for settings",
          "write_file",
          Some(e.to_string())
        );
        return Err(format!(
          "Failed to create temporary file for settings: {}",
          e
        ));
      }
    };

    if let Err(e) = temp_file.write_all(serialized.as_bytes()) {
      log_error!(
        "Failed to write to temporary settings file",
        "write_file",
        Some(e.to_string())
      );
      return Err(format!("Failed to write to temporary settings file: {}", e));
    }

    // 一時ファイルを本来の設定ファイルに置き換える
    if let Err(e) = temp_file.persist(&config_file) {
      log_error!(
        "Failed to persist temporary settings file",
        "write_file",
        Some(e.to_string())
      );
      return Err(format!("Failed to persist temporary settings file: {}", e));
    }

    Ok(())
  }

  fn read_file(&mut self) -> Result<(), String> {
    let config_file = utils::file::get_app_data_dir(SETTINGS_FILENAME);

    match fs::read_to_string(config_file) {
      Ok(input) => match serde_json::from_str::<Self>(&input) {
        Ok(deserialized) => {
          *self = deserialized;
          Ok(())
        }
        Err(e) => {
          log_error!(
            "Failed to deserialize settings",
            "read_file",
            Some(e.to_string())
          );
          Err(format!("Failed to deserialize settings: {}", e))
        }
      },
      Err(e) => {
        log_error!(
          "Failed to deserialize settings",
          "read_file",
          Some(e.to_string())
        );
        Err(format!("Failed to read settings file: {}", e))
      }
    }
  }
}

impl structs::settings::Settings {
  pub fn new() -> Self {
    let config_file = utils::file::get_app_data_dir(SETTINGS_FILENAME);

    let mut settings = Self::default();

    if !config_file.exists() {
      return settings;
    }

    if let Err(e) = settings.read_file() {
      log_error!("read_config_failed", "read_file", Some(e.to_string()));
    }

    settings
  }

  pub fn set_language(&mut self, new_lang: String) -> Result<(), String> {
    self.language = new_lang;
    self.write_file()
  }

  pub fn set_theme(&mut self, new_theme: enums::config::Theme) -> Result<(), String> {
    self.theme = new_theme;
    self.write_file()
  }

  pub fn set_display_targets(
    &mut self,
    new_targets: Vec<enums::hardware::HardwareType>,
  ) -> Result<(), String> {
    self.display_targets = new_targets;
    self.write_file()
  }

  pub fn set_graph_size(
    &mut self,
    new_size: enums::config::GraphSize,
  ) -> Result<(), String> {
    self.graph_size = new_size;
    self.write_file()
  }

  pub fn set_line_graph_border(&mut self, new_value: bool) -> Result<(), String> {
    self.line_graph_border = new_value;
    self.write_file()
  }

  pub fn set_line_graph_fill(&mut self, new_value: bool) -> Result<(), String> {
    self.line_graph_fill = new_value;
    self.write_file()
  }

  ///
  /// ## グラフの色を設定する
  ///
  /// - グラフの色は #ffffff 形式の文字列で入力される
  /// - グラフの色は RGB 形式の値に変換して保存する
  ///
  pub fn set_line_graph_color(
    &mut self,
    key: enums::hardware::HardwareType,
    new_color: String,
  ) -> Result<String, String> {
    let new_color = match utils::color::hex_to_rgb(&new_color) {
      Ok(rgb) => rgb,
      Err(e) => {
        log_error!("Invalid color format", "set_line_graph_color", Some(e));
        return Err("Invalid color format".to_string());
      }
    };

    match key {
      enums::hardware::HardwareType::CPU => {
        self.line_graph_color.cpu = new_color;
      }
      enums::hardware::HardwareType::Memory => {
        self.line_graph_color.memory = new_color;
      }
      enums::hardware::HardwareType::GPU => {
        self.line_graph_color.gpu = new_color;
      }
    }

    let _ = self.write_file();

    match key {
      enums::hardware::HardwareType::CPU => Ok(
        self
          .line_graph_color
          .cpu
          .iter()
          .map(|&c| c.to_string())
          .collect::<Vec<String>>()
          .join(","),
      ),
      enums::hardware::HardwareType::Memory => Ok(
        self
          .line_graph_color
          .memory
          .iter()
          .map(|&c| c.to_string())
          .collect::<Vec<String>>()
          .join(","),
      ),
      enums::hardware::HardwareType::GPU => Ok(
        self
          .line_graph_color
          .gpu
          .iter()
          .map(|&c| c.to_string())
          .collect::<Vec<String>>()
          .join(","),
      ),
    }
  }

  pub fn set_line_graph_mix(&mut self, new_value: bool) -> Result<(), String> {
    self.line_graph_mix = new_value;
    self.write_file()
  }

  pub fn set_line_graph_show_legend(&mut self, new_value: bool) -> Result<(), String> {
    self.line_graph_show_legend = new_value;
    self.write_file()
  }

  pub fn set_line_graph_show_scale(&mut self, new_value: bool) -> Result<(), String> {
    self.line_graph_show_scale = new_value;
    self.write_file()
  }

  pub fn set_background_img_opacity(&mut self, new_value: u8) -> Result<(), String> {
    self.background_img_opacity = new_value;
    self.write_file()
  }

  pub fn set_selected_background_img(
    &mut self,
    new_value: Option<String>,
  ) -> Result<(), String> {
    self.selected_background_img = new_value;
    self.write_file()
  }

  pub fn set_temperature_unit(
    &mut self,
    new_unit: enums::config::TemperatureUnit,
  ) -> Result<(), String> {
    self.temperature_unit = new_unit;
    self.write_file()
  }

  pub async fn get_temperature_unit(&self) -> &enums::config::TemperatureUnit {
    &self.temperature_unit
  }
}

#[derive(Debug)]
pub struct AppState {
  pub settings: Mutex<structs::settings::Settings>,
}

impl AppState {
  pub fn new() -> Self {
    Self {
      settings: Mutex::from(structs::settings::Settings::new()),
    }
  }
}

pub mod commands {
  use super::*;
  use serde_json::json;
  use tauri::{Emitter, EventTarget, Window};

  const ERROR_TITLE: &str = "Failed to update settings file";

  ///
  /// ## エラーイベントを発生させフロントエンドに通知する
  ///
  /// [TODO] dialog を使ってエラーメッセージを表示する
  ///
  fn emit_error(window: &Window) -> Result<(), String> {
    let settings_json_path = utils::file::get_app_data_dir(SETTINGS_FILENAME);

    window
      .emit_to(
        EventTarget::window(window.label().to_string()),
        "error_event",
        json!({
            "title": ERROR_TITLE,
            "message": format!("If this happens repeatedly, delete {} and restart the app.", settings_json_path.display())
        }),
      )
      .map_err(|e| format!("Failed to emit event: {}", e))?;

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn get_settings(
    state: tauri::State<'_, AppState>,
  ) -> Result<structs::settings::ClientSettings, String> {
    let settings = state.settings.lock().unwrap().clone();

    // フロントで扱いやすいようにカンマ区切りの文字列に変換する
    let color_strings = structs::settings::LineGraphColorStringSettings {
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

    let client_settings = structs::settings::ClientSettings {
      language: settings.language,
      theme: settings.theme,
      display_targets: settings.display_targets,
      graph_size: settings.graph_size,
      line_graph_border: settings.line_graph_border,
      line_graph_fill: settings.line_graph_fill,
      line_graph_color: color_strings,
      line_graph_mix: settings.line_graph_mix,
      line_graph_show_legend: settings.line_graph_show_legend,
      line_graph_show_scale: settings.line_graph_show_scale,
      background_img_opacity: settings.background_img_opacity,
      selected_background_img: settings.selected_background_img,
      temperature_unit: settings.temperature_unit,
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
    new_theme: enums::config::Theme,
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
    new_size: enums::config::GraphSize,
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
    new_unit: enums::config::TemperatureUnit,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_temperature_unit(new_unit) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }
}
