use crate::enums;
use crate::models;
use crate::services::external_component_guidance_service::normalize_external_component_guidance_key;
use crate::utils;
use crate::{log_error, log_info};
use std::io::Write;

pub const SETTINGS_FILENAME: &str = "settings.json";
pub const GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION: u32 = 1;
const MIN_WINDOW_OPACITY: u8 = 20;
const MAX_WINDOW_OPACITY: u8 = 100;
const MIN_GLASS_BLUR: u8 = 0;
const MAX_GLASS_BLUR: u8 = 30;
const MAX_GRAPH_MARGIN_PX: u32 = 200;

pub trait SettingActions {
  fn write_file(&self) -> Result<(), String>;
  fn read_file(&mut self) -> Result<(), String>;
}

impl SettingActions for models::settings::Settings {
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

    let serialized = match serialize_preserving_unknown_settings(self, &config_file) {
      Ok(s) => s,
      Err(e) => {
        log_error!(
          "Failed to serialize settings",
          "write_file",
          Some(e.clone())
        );
        return Err(e);
      }
    };

    // Write to temporary file
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

    // Replace temporary file with actual settings file
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

    let input = std::fs::read_to_string(config_file).map_err(|e| {
      log_error!(
        "Failed to read settings file",
        "read_file",
        Some(e.to_string())
      );
      format!("Failed to read settings file: {e}")
    })?;

    read_settings_from_str(self, &input)
  }
}

fn read_settings_from_str(
  settings: &mut models::settings::Settings,
  input: &str,
) -> Result<(), String> {
  match serde_json::from_str::<models::settings::Settings>(input) {
    Ok(deserialized) => {
      *settings = deserialized;
      clamp_loaded_settings(settings);
      Ok(())
    }
    Err(e) => {
      log_error!(
        "Failed to deserialize settings, attempting field-level recovery",
        "read_file",
        Some(e.to_string())
      );
      // Fall back to field-by-field recovery
      settings.merge_from_json_str(input)
    }
  }
}

fn serialize_preserving_unknown_settings(
  settings: &models::settings::Settings,
  config_file: &std::path::Path,
) -> Result<String, String> {
  let mut document = match std::fs::read_to_string(config_file) {
    Ok(input) => match serde_json::from_str::<serde_json::Value>(&input) {
      Ok(serde_json::Value::Object(map)) => map,
      Ok(_) => return Err("Existing settings file must be a JSON object".to_string()),
      Err(e) => return Err(format!("Failed to parse existing settings file: {e}")),
    },
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
    Err(e) => return Err(format!("Failed to read existing settings file: {e}")),
  };

  let app_value = serde_json::to_value(settings)
    .map_err(|e| format!("Failed to serialize settings: {e}"))?;
  let serde_json::Value::Object(app_map) = app_value else {
    return Err("Serialized settings must be a JSON object".to_string());
  };

  for (key, value) in app_map {
    document.insert(key, value);
  }

  serde_json::to_string(&serde_json::Value::Object(document))
    .map_err(|e| format!("Failed to serialize settings: {e}"))
}

impl models::settings::Settings {
  /// Parses JSON field-by-field, applying only the fields that
  /// deserialize successfully and leaving the rest at their defaults.
  pub(crate) fn merge_from_json_str(&mut self, input: &str) -> Result<(), String> {
    let value = serde_json::from_str::<serde_json::Value>(input).map_err(|e| {
      log_error!(
        "Settings file is not valid JSON",
        "merge_from_json_str",
        Some(e.to_string())
      );
      format!("Settings file is not valid JSON: {e}")
    })?;

    let map = match value {
      serde_json::Value::Object(map) => map,
      _ => {
        let msg = "Settings file must be a JSON object".to_string();
        log_error!(
          "Settings file must be a JSON object",
          "merge_from_json_str",
          None::<&str>
        );
        return Err(msg);
      }
    };

    macro_rules! try_field {
      ($field:ident, $key:expr) => {
        if let Some(v) = map.get($key) {
          match serde_json::from_value(v.clone()) {
            Ok(val) => self.$field = val,
            Err(e) => {
              log_error!(
                concat!("Failed to deserialize field: ", $key),
                "merge_from_json_str",
                Some(e.to_string())
              );
            }
          }
        }
      };
    }

    try_field!(version, "version");
    try_field!(language, "language");
    try_field!(theme, "theme");
    try_field!(navigation_layout, "navigationLayout");
    try_field!(ui_announcement_version, "uiAnnouncementVersion");
    try_field!(display_targets, "displayTargets");
    try_field!(graph_size, "graphSize");
    try_field!(graph_fit_to_window, "graphFitToWindow");
    try_field!(graph_margin_px, "graphMarginPx");
    try_field!(line_graph_type, "lineGraphType");
    try_field!(line_graph_border, "lineGraphBorder");
    try_field!(line_graph_fill, "lineGraphFill");
    try_field!(line_graph_color, "lineGraphColor");
    try_field!(line_graph_mix, "lineGraphMix");
    try_field!(line_graph_show_legend, "lineGraphShowLegend");
    try_field!(line_graph_show_scale, "lineGraphShowScale");
    try_field!(line_graph_show_tooltip, "lineGraphShowTooltip");
    try_field!(background_img_opacity, "backgroundImgOpacity");
    try_field!(selected_background_img, "selectedBackgroundImg");
    try_field!(transparent_ui, "transparentUi");
    try_field!(window_opacity, "windowOpacity");
    try_field!(glass_blur, "glassBlur");
    clamp_loaded_settings(self);
    try_field!(temperature_unit, "temperatureUnit");
    try_field!(burn_in_shift, "burnInShift");
    try_field!(burn_in_shift_mode, "burnInShiftMode");
    try_field!(burn_in_shift_preset, "burnInShiftPreset");
    try_field!(burn_in_shift_idle_only, "burnInShiftIdleOnly");
    try_field!(burn_in_shift_options, "burnInShiftOptions");
    try_field!(text_selectable, "textSelectable");
    try_field!(close_to_tray, "closeToTray");
    try_field!(close_to_tray_choice_made, "closeToTrayChoiceMade");
    try_field!(external_component_guidance, "externalComponentGuidance");
    try_field!(elevated_startup_mode, "elevatedStartupMode");
    try_field!(tray_widget, "trayWidget");

    Ok(())
  }

  pub fn new() -> Self {
    let config_file = utils::file::get_app_data_dir(SETTINGS_FILENAME);

    let mut settings = Self::default();

    if !config_file.exists() {
      return settings;
    }

    if let Err(e) = settings.read_file() {
      log_error!("read_config_failed", "read_file", Some(e.to_string()));
      return settings;
    }

    // Update version and persist new fields added by app updates
    let current_version = utils::tauri::get_app_version();
    if settings.version != current_version {
      settings.version = current_version;
      if let Err(e) = settings.write_file() {
        log_error!(
          "Failed to update settings file after version change",
          "new",
          Some(e.to_string())
        );
      }
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

  pub fn set_navigation_layout(
    &mut self,
    new_layout: enums::settings::NavigationLayout,
  ) -> Result<(), String> {
    self.set_navigation_layout_with_writer(new_layout, |settings| settings.write_file())
  }

  fn set_navigation_layout_with_writer<F>(
    &mut self,
    new_layout: enums::settings::NavigationLayout,
    write_file: F,
  ) -> Result<(), String>
  where
    F: FnOnce(&Self) -> Result<(), String>,
  {
    let previous_layout = self.navigation_layout;
    let previous_announcement_version = self.ui_announcement_version;
    self.navigation_layout = new_layout;
    if new_layout == enums::settings::NavigationLayout::Classic {
      self.ui_announcement_version = self
        .ui_announcement_version
        .max(GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION);
    }

    if let Err(error) = write_file(self) {
      self.navigation_layout = previous_layout;
      self.ui_announcement_version = previous_announcement_version;
      return Err(error);
    }

    Ok(())
  }

  pub fn acknowledge_navigation_restructure_announcement(
    &mut self,
  ) -> Result<(), String> {
    self.acknowledge_navigation_restructure_announcement_with_writer(|settings| {
      settings.write_file()
    })
  }

  fn acknowledge_navigation_restructure_announcement_with_writer<F>(
    &mut self,
    write_file: F,
  ) -> Result<(), String>
  where
    F: FnOnce(&Self) -> Result<(), String>,
  {
    let previous_value = self.ui_announcement_version;
    self.ui_announcement_version = self
      .ui_announcement_version
      .max(GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION);

    if let Err(error) = write_file(self) {
      self.ui_announcement_version = previous_value;
      return Err(error);
    }

    Ok(())
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

  pub fn set_graph_fit_to_window(&mut self, new_value: bool) -> Result<(), String> {
    self.set_graph_fit_to_window_with_writer(new_value, |settings| settings.write_file())
  }

  fn set_graph_fit_to_window_with_writer<F>(
    &mut self,
    new_value: bool,
    write_file: F,
  ) -> Result<(), String>
  where
    F: FnOnce(&Self) -> Result<(), String>,
  {
    let previous_value = self.graph_fit_to_window;
    self.graph_fit_to_window = new_value;

    if let Err(error) = write_file(self) {
      self.graph_fit_to_window = previous_value;
      return Err(error);
    }

    Ok(())
  }

  pub fn set_graph_margin_px(&mut self, new_value: u32) -> Result<(), String> {
    self.set_graph_margin_px_with_writer(new_value, |settings| settings.write_file())
  }

  fn set_graph_margin_px_with_writer<F>(
    &mut self,
    new_value: u32,
    write_file: F,
  ) -> Result<(), String>
  where
    F: FnOnce(&Self) -> Result<(), String>,
  {
    let previous_value = self.graph_margin_px;
    self.graph_margin_px = new_value.min(MAX_GRAPH_MARGIN_PX);

    if let Err(error) = write_file(self) {
      self.graph_margin_px = previous_value;
      return Err(error);
    }

    Ok(())
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
  /// ## Set graph color
  ///
  /// - Graph color is input as a #ffffff format string
  /// - Graph color is converted to RGB format values and saved
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

  pub fn set_transparent_ui(&mut self, new_value: bool) -> Result<(), String> {
    self.transparent_ui = new_value;
    self.write_file()
  }

  pub fn set_window_opacity(&mut self, new_value: u8) -> Result<(), String> {
    self.window_opacity = new_value.clamp(MIN_WINDOW_OPACITY, MAX_WINDOW_OPACITY);
    self.write_file()
  }

  pub fn set_glass_blur(&mut self, new_value: u8) -> Result<(), String> {
    self.glass_blur = new_value.clamp(MIN_GLASS_BLUR, MAX_GLASS_BLUR);
    self.write_file()
  }

  pub fn set_temperature_unit(
    &mut self,
    new_unit: enums::settings::TemperatureUnit,
  ) -> Result<(), String> {
    self.temperature_unit = new_unit;
    self.write_file()
  }

  pub fn set_burn_in_shift(&mut self, new_value: bool) -> Result<(), String> {
    self.burn_in_shift = new_value;
    self.write_file()
  }

  pub fn set_burn_in_shift_mode(
    &mut self,
    new_value: enums::settings::BurnInShiftMode,
  ) -> Result<(), String> {
    self.burn_in_shift_mode = new_value;
    self.write_file()
  }

  pub fn set_burn_in_shift_preset(
    &mut self,
    new_value: enums::settings::BurnInShiftPreset,
  ) -> Result<(), String> {
    self.burn_in_shift_preset = new_value;
    self.write_file()
  }

  pub fn set_burn_in_shift_idle_only(&mut self, new_value: bool) -> Result<(), String> {
    self.burn_in_shift_idle_only = new_value;
    self.write_file()
  }

  pub fn set_burn_in_shift_options(
    &mut self,
    new_value: Option<models::settings::BurnInShiftOptions>,
  ) -> Result<(), String> {
    self.burn_in_shift_options = new_value;
    self.write_file()
  }

  pub fn set_text_selectable(&mut self, new_value: bool) -> Result<(), String> {
    self.text_selectable = new_value;
    self.write_file()
  }

  pub fn set_tray_widget_settings(
    &mut self,
    new_value: crate::tray::widget::TrayWidgetSettings,
  ) -> Result<(), String> {
    self.tray_widget = new_value.normalized();
    self.write_file()
  }

  pub fn set_close_to_tray_preference(&mut self, new_value: bool) -> Result<(), String> {
    self.close_to_tray = new_value;
    self.close_to_tray_choice_made = true;
    self.write_file()
  }

  pub fn acknowledge_external_component_guidance_key(
    &mut self,
    key: String,
  ) -> Result<(), String> {
    let key = normalize_external_component_guidance_key(&key)?;
    if self
      .external_component_guidance
      .acknowledged_keys
      .contains(&key)
    {
      return self.write_file();
    }

    let mut next = self.clone();
    next.external_component_guidance.acknowledged_keys.push(key);
    next.write_file()?;
    *self = next;

    Ok(())
  }

  pub fn set_elevated_startup_mode(&mut self, new_value: bool) -> Result<(), String> {
    self
      .set_elevated_startup_mode_with_writer(new_value, |settings| settings.write_file())
  }

  fn set_elevated_startup_mode_with_writer<F>(
    &mut self,
    new_value: bool,
    write_file: F,
  ) -> Result<(), String>
  where
    F: FnOnce(&Self) -> Result<(), String>,
  {
    let previous_value = self.elevated_startup_mode;
    self.elevated_startup_mode = new_value;

    if let Err(e) = write_file(self) {
      self.elevated_startup_mode = previous_value;
      return Err(e);
    }

    Ok(())
  }
}

fn clamp_loaded_settings(settings: &mut models::settings::Settings) {
  settings.window_opacity = settings
    .window_opacity
    .clamp(MIN_WINDOW_OPACITY, MAX_WINDOW_OPACITY);
  settings.glass_blur = settings.glass_blur.clamp(MIN_GLASS_BLUR, MAX_GLASS_BLUR);
  settings.graph_margin_px = settings.graph_margin_px.min(MAX_GRAPH_MARGIN_PX);
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[test]
  fn read_settings_from_str_clamps_full_deserialization_values() {
    let mut settings = models::settings::Settings::default();

    read_settings_from_str(
      &mut settings,
      r#"{"windowOpacity":5,"glassBlur":200,"graphMarginPx":1000}"#,
    )
    .unwrap();

    assert_eq!(settings.window_opacity, MIN_WINDOW_OPACITY);
    assert_eq!(settings.glass_blur, MAX_GLASS_BLUR);
    assert_eq!(settings.graph_margin_px, MAX_GRAPH_MARGIN_PX);
  }

  #[test]
  fn merge_from_json_str_recovers_graph_fit_fields() {
    let mut settings = models::settings::Settings::default();

    settings
      .merge_from_json_str(r#"{"graphFitToWindow":true,"graphMarginPx":64}"#)
      .unwrap();

    assert!(settings.graph_fit_to_window);
    assert_eq!(settings.graph_margin_px, 64);
  }

  #[test]
  fn read_settings_recovers_unknown_navigation_layout_without_losing_valid_fields() {
    let mut settings = models::settings::Settings::default();

    read_settings_from_str(
      &mut settings,
      r#"{"language":"ja","navigationLayout":"future"}"#,
    )
    .unwrap();

    assert_eq!(settings.language, "ja");
    assert_eq!(
      settings.navigation_layout,
      enums::settings::NavigationLayout::Grouped
    );
  }

  #[test]
  fn set_navigation_layout_rolls_back_when_write_fails() {
    let mut settings = models::settings::Settings::default();

    let result = settings.set_navigation_layout_with_writer(
      enums::settings::NavigationLayout::Classic,
      |_| Err("write failed".to_string()),
    );

    assert_eq!(result, Err("write failed".to_string()));
    assert_eq!(
      settings.navigation_layout,
      enums::settings::NavigationLayout::Grouped
    );
    assert_eq!(settings.ui_announcement_version, 0);
  }

  #[test]
  fn opting_out_to_classic_dismisses_navigation_notice() {
    let mut settings = models::settings::Settings::default();

    settings
      .set_navigation_layout_with_writer(
        enums::settings::NavigationLayout::Classic,
        |_| Ok(()),
      )
      .unwrap();

    assert_eq!(
      settings.ui_announcement_version,
      GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION
    );
  }

  #[test]
  fn opting_out_does_not_regress_a_newer_announcement_version() {
    let mut settings = models::settings::Settings {
      ui_announcement_version: GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION + 1,
      ..models::settings::Settings::default()
    };

    settings
      .set_navigation_layout_with_writer(
        enums::settings::NavigationLayout::Classic,
        |_| Ok(()),
      )
      .unwrap();

    assert_eq!(
      settings.ui_announcement_version,
      GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION + 1
    );
  }

  #[test]
  fn acknowledge_navigation_announcement_rolls_back_when_write_fails() {
    let mut settings = models::settings::Settings::default();

    let result =
      settings.acknowledge_navigation_restructure_announcement_with_writer(|_| {
        Err("write failed".to_string())
      });

    assert_eq!(result, Err("write failed".to_string()));
    assert_eq!(settings.ui_announcement_version, 0);
  }

  #[test]
  fn merge_from_json_str_clamps_graph_margin_px() {
    let mut settings = models::settings::Settings::default();

    settings
      .merge_from_json_str(r#"{"graphMarginPx":1000}"#)
      .unwrap();

    assert_eq!(settings.graph_margin_px, MAX_GRAPH_MARGIN_PX);
  }

  #[test]
  fn set_graph_fit_to_window_restores_value_when_writer_fails() {
    let mut settings = models::settings::Settings::default();

    let error = settings
      .set_graph_fit_to_window_with_writer(true, |_| Err("write failed".to_string()))
      .unwrap_err();

    assert_eq!(error, "write failed");
    assert!(!settings.graph_fit_to_window);
  }

  #[test]
  fn set_graph_margin_px_restores_value_when_writer_fails() {
    let mut settings = models::settings::Settings::default();
    let previous_value = settings.graph_margin_px;

    let error = settings
      .set_graph_margin_px_with_writer(1000, |next_settings| {
        assert_eq!(next_settings.graph_margin_px, MAX_GRAPH_MARGIN_PX);
        Err("write failed".to_string())
      })
      .unwrap_err();

    assert_eq!(error, "write failed");
    assert_eq!(settings.graph_margin_px, previous_value);
  }

  #[test]
  fn merge_from_json_str_recovers_elevated_startup_mode() {
    let mut settings = models::settings::Settings::default();

    settings
      .merge_from_json_str(r#"{"elevatedStartupMode":true}"#)
      .unwrap();

    assert!(settings.elevated_startup_mode);
  }

  #[test]
  fn set_elevated_startup_mode_persists_when_writer_succeeds() {
    let mut settings = models::settings::Settings::default();
    let mut persisted_value = false;

    settings
      .set_elevated_startup_mode_with_writer(true, |next_settings| {
        persisted_value = next_settings.elevated_startup_mode;
        Ok(())
      })
      .unwrap();

    assert!(settings.elevated_startup_mode);
    assert!(persisted_value);
  }

  #[test]
  fn set_elevated_startup_mode_restores_value_when_writer_fails() {
    let mut settings = models::settings::Settings::default();

    let err = settings
      .set_elevated_startup_mode_with_writer(true, |_| Err("write failed".to_string()))
      .unwrap_err();

    assert_eq!(err, "write failed");
    assert!(!settings.elevated_startup_mode);
  }

  #[test]
  fn serialize_preserving_unknown_settings_keeps_core_owned_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
      &path,
      r#"{
        "hardwareArchive": {"enabled": false, "retentionDays": 90, "scheduledDataDeletion": true},
        "storageHealth": {"enabled": true, "retentionDays": 3650},
        "storageHealthIdentity": {"hashKey": "v1:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"}
      }"#,
    )
    .unwrap();

    let settings = models::settings::Settings::default();
    let serialized = serialize_preserving_unknown_settings(&settings, &path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(
      value
        .pointer("/hardwareArchive/retentionDays")
        .and_then(|v| v.as_u64()),
      Some(90)
    );
    assert_eq!(
      value
        .pointer("/storageHealth/retentionDays")
        .and_then(|v| v.as_u64()),
      Some(3650)
    );
    assert_eq!(
      value
        .pointer("/storageHealthIdentity/hashKey")
        .and_then(|v| v.as_str()),
      Some("v1:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
    );
    assert!(value.get("language").is_some());
  }

  #[test]
  fn serialize_preserving_unknown_settings_rejects_invalid_existing_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, "{invalid").unwrap();

    let settings = models::settings::Settings::default();
    let err = serialize_preserving_unknown_settings(&settings, &path).unwrap_err();

    assert!(err.contains("Failed to parse existing settings file"));
  }

  #[test]
  fn serialize_preserving_unknown_settings_rejects_non_object_existing_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, "[]").unwrap();

    let settings = models::settings::Settings::default();
    let err = serialize_preserving_unknown_settings(&settings, &path).unwrap_err();

    assert_eq!(err, "Existing settings file must be a JSON object");
  }
}
