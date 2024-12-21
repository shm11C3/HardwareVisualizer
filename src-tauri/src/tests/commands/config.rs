#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  use crate::commands::config::*;
  use crate::enums::{self, hardware};
  use crate::services::language;
  use crate::utils;

  #[test]
  fn test_default_settings() {
    // デフォルト設定が正しいか確認
    let settings = Settings::default();

    assert_eq!(
      settings.version,
      utils::tauri::get_app_version(&utils::tauri::get_config())
    );
    assert_eq!(settings.language, language::get_default_language());
    assert_eq!(settings.theme, enums::config::Theme::Dark);
    assert_eq!(
      settings.display_targets,
      vec![
        hardware::HardwareType::CPU,
        hardware::HardwareType::Memory,
        hardware::HardwareType::GPU
      ]
    );
    assert!(settings.line_graph_border);
    assert!(settings.line_graph_fill);
  }

  #[test]
  fn test_set_language() {
    let mut settings = Settings::default();
    let new_language = "es".to_string();

    assert!(settings.set_language(new_language.clone()).is_ok());
    assert_eq!(settings.language, new_language);
  }

  #[test]
  fn test_set_theme() {
    let mut settings = Settings::default();
    let new_theme = enums::config::Theme::Light;

    assert!(settings.set_theme(new_theme.clone()).is_ok());
    assert_eq!(settings.theme, new_theme);
  }

  #[test]
  fn test_set_display_targets() {
    let mut settings = Settings::default();
    let targets = vec![hardware::HardwareType::GPU, hardware::HardwareType::Memory];
    assert!(settings.set_display_targets(targets.clone()).is_ok());
    assert_eq!(settings.display_targets, targets);
  }

  #[test]
  fn test_set_graph_size() {
    let mut settings = Settings::default();
    //assert!(settings
    //  .set_graph_size(enums::config::GraphSize::SM)
    //  .is_ok());
    assert_eq!(settings.graph_size, enums::config::GraphSize::SM);
  }

  #[test]
  fn test_set_line_graph_border() {
    let mut settings = Settings::default();
    assert!(settings.set_line_graph_border(false).is_ok());
    assert!(!settings.line_graph_border);
  }

  #[test]
  fn test_set_line_graph_fill() {
    let mut settings = Settings::default();
    assert!(settings.set_line_graph_fill(false).is_ok());
    assert!(!settings.line_graph_fill);
  }

  #[test]
  fn test_set_line_graph_color() {
    let mut settings = Settings::default();
    let new_color = "#ff0000".to_string();

    assert!(settings
      .set_line_graph_color(hardware::HardwareType::CPU, new_color.clone())
      .is_ok());
    assert_eq!(settings.line_graph_color.cpu, [255, 0, 0]);
  }

  #[test]
  fn test_set_invalid_line_graph_color() {
    let mut settings = Settings::default();
    let invalid_color = "invalid_color".to_string();

    assert!(settings
      .set_line_graph_color(hardware::HardwareType::CPU, invalid_color)
      .is_err());
  }

  #[test]
  fn test_set_line_graph_color_invalid() {
    let mut settings = Settings::default();
    let new_color = "invalid_color".to_string();
    assert!(settings
      .set_line_graph_color(hardware::HardwareType::CPU, new_color)
      .is_err());
  }

  #[test]
  fn test_set_line_graph_mix() {
    let mut settings = Settings::default();
    assert!(settings.set_line_graph_mix(false).is_ok());
    assert!(!settings.line_graph_mix);
  }

  #[test]
  fn test_set_line_graph_show_legend() {
    let mut settings = Settings::default();
    assert!(settings.set_line_graph_show_legend(false).is_ok());
    assert!(!settings.line_graph_show_legend);
  }

  #[test]
  fn test_set_line_graph_show_scale() {
    let mut settings = Settings::default();
    assert!(settings.set_line_graph_show_scale(false).is_ok());
    assert!(!settings.line_graph_show_scale);
  }

  #[test]
  fn test_set_background_img_opacity() {
    let mut settings = Settings::default();
    assert!(settings.set_background_img_opacity(100).is_ok());
    assert_eq!(settings.background_img_opacity, 100);
  }

  #[test]
  fn test_set_selected_background_img() {
    let mut settings = Settings::default();
    let img_path = Some("path/to/image.png".to_string());
    assert!(settings
      .set_selected_background_img(img_path.clone())
      .is_ok());
    assert_eq!(settings.selected_background_img, img_path);
  }
}
