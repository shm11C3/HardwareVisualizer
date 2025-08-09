use crate::enums;
use crate::structs;
use crate::utils;
use crate::{log_error, log_info, log_internal};
use std::io::Write;

pub const SETTINGS_FILENAME: &str = "settings.json";

pub trait SettingActions {
  fn write_file(&self) -> Result<(), String>;
  fn read_file(&mut self) -> Result<(), String>;
}

impl SettingActions for structs::settings::Settings {
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
      log_info!(
        "Creating configuration directory",
        "write_file",
        None::<&str>
      );

      std::fs::create_dir_all(config_dir).map_err(|e| {
        log_error!(
          "Failed to create configuration directory",
          "write_file",
          Some(e.to_string())
        );
        format!("Failed to create configuration directory: {e}")
      })?;
    }

    let serialized = match serde_json::to_string(self) {
      Ok(s) => s,
      Err(e) => {
        log_error!(
          "Failed to serialize settings",
          "write_file",
          Some(e.to_string())
        );
        return Err(format!("Failed to serialize settings: {e}"));
      }
    };

    // 一時ファイルに書き込む
    let mut temp_file = match tempfile::NamedTempFile::new_in(config_dir) {
      Ok(file) => file,
      Err(e) => {
        log_error!(
          "Failed to create temporary file for settings",
          "write_file",
          Some(e.to_string())
        );
        return Err(format!("Failed to create temporary file for settings: {e}"));
      }
    };

    if let Err(e) = temp_file.write_all(serialized.as_bytes()) {
      log_error!(
        "Failed to write to temporary settings file",
        "write_file",
        Some(e.to_string())
      );
      return Err(format!("Failed to write to temporary settings file: {e}"));
    }

    // 一時ファイルを本来の設定ファイルに置き換える
    if let Err(e) = temp_file.persist(&config_file) {
      log_error!(
        "Failed to persist temporary settings file",
        "write_file",
        Some(e.to_string())
      );
      return Err(format!("Failed to persist temporary settings file: {e}"));
    }

    Ok(())
  }

  fn read_file(&mut self) -> Result<(), String> {
    let config_file = utils::file::get_app_data_dir(SETTINGS_FILENAME);

    match std::fs::read_to_string(config_file) {
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
          Err(format!("Failed to deserialize settings: {e}"))
        }
      },
      Err(e) => {
        log_error!(
          "Failed to deserialize settings",
          "read_file",
          Some(e.to_string())
        );
        Err(format!("Failed to read settings file: {e}"))
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

  pub fn set_theme(&mut self, new_theme: enums::settings::Theme) -> Result<(), String> {
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
    new_size: enums::settings::GraphSize,
  ) -> Result<(), String> {
    self.graph_size = new_size;
    self.write_file()
  }

  pub fn set_line_graph_type(
    &mut self,
    new_type: enums::settings::LineGraphType,
  ) -> Result<(), String> {
    self.line_graph_type = new_type;
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
      enums::hardware::HardwareType::Cpu => {
        self.line_graph_color.cpu = new_color;
      }
      enums::hardware::HardwareType::Memory => {
        self.line_graph_color.memory = new_color;
      }
      enums::hardware::HardwareType::Gpu => {
        self.line_graph_color.gpu = new_color;
      }
    }

    let _ = self.write_file();

    match key {
      enums::hardware::HardwareType::Cpu => Ok(
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
      enums::hardware::HardwareType::Gpu => Ok(
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

  pub fn set_line_graph_show_tooltip(&mut self, new_value: bool) -> Result<(), String> {
    self.line_graph_show_tooltip = new_value;
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
    new_unit: enums::settings::TemperatureUnit,
  ) -> Result<(), String> {
    self.temperature_unit = new_unit;
    self.write_file()
  }

  pub fn set_hardware_archive_enabled(&mut self, new_value: bool) -> Result<(), String> {
    self.hardware_archive.enabled = new_value;
    self.write_file()
  }

  pub fn set_hardware_archive_interval(
    &mut self,
    new_interval: u32,
  ) -> Result<(), String> {
    self.hardware_archive.refresh_interval_days = new_interval;
    self.write_file()
  }

  pub fn set_hardware_archive_scheduled_data_deletion(
    &mut self,
    new_value: bool,
  ) -> Result<(), String> {
    self.hardware_archive.scheduled_data_deletion = new_value;
    self.write_file()
  }
}
