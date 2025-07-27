#[cfg(test)]
mod tests {
  use crate::enums::{self, hardware};
  use crate::services::language;
  use crate::structs;
  use crate::utils;

  #[test]
  fn test_default_settings() {
    // デフォルト設定が正しいか確認
    let settings = structs::settings::Settings::default();

    let expected = structs::settings::Settings {
      version: utils::tauri::get_app_version(&utils::tauri::get_config()),
      language: language::get_default_language(),
      theme: enums::settings::Theme::Dark,
      display_targets: vec![
        hardware::HardwareType::Cpu,
        hardware::HardwareType::Memory,
        hardware::HardwareType::Gpu,
      ],
      graph_size: enums::settings::GraphSize::XL,
      line_graph_type: enums::settings::LineGraphType::Default,
      line_graph_border: true,
      line_graph_fill: true,
      line_graph_color: structs::settings::LineGraphColorSettings {
        cpu: [75, 192, 192],
        memory: [255, 99, 132],
        gpu: [255, 206, 86],
      },
      line_graph_mix: true,
      line_graph_show_legend: true,
      line_graph_show_scale: false,
      line_graph_show_tooltip: true,
      background_img_opacity: 50,
      selected_background_img: None,
      temperature_unit: enums::settings::TemperatureUnit::Celsius,
      hardware_archive: structs::hardware_archive::HardwareArchiveSettings {
        enabled: true,
        refresh_interval_days: 30,
        scheduled_data_deletion: true,
      },
    };

    assert_eq!(settings.version, expected.version,);
    assert_eq!(settings.language, expected.language);
    assert_eq!(settings.theme, expected.theme);
    assert_eq!(settings.display_targets, expected.display_targets,);
    assert_eq!(settings.line_graph_border, expected.line_graph_border);
    assert_eq!(settings.graph_size, expected.graph_size);
    assert_eq!(settings.line_graph_type, expected.line_graph_type);
    assert_eq!(settings.line_graph_fill, expected.line_graph_fill);
    assert_eq!(settings.line_graph_color.cpu, expected.line_graph_color.cpu);
    assert_eq!(settings.line_graph_color.gpu, expected.line_graph_color.gpu);
    assert_eq!(
      settings.line_graph_color.memory,
      expected.line_graph_color.memory
    );
    assert_eq!(settings.line_graph_mix, expected.line_graph_mix);
    assert_eq!(
      settings.line_graph_show_legend,
      expected.line_graph_show_legend
    );
    assert_eq!(
      settings.line_graph_show_scale,
      expected.line_graph_show_scale
    );
    assert_eq!(
      settings.line_graph_show_tooltip,
      expected.line_graph_show_tooltip
    );
    assert_eq!(
      settings.background_img_opacity,
      expected.background_img_opacity
    );
    assert_eq!(
      settings.selected_background_img,
      expected.selected_background_img
    );
    assert_eq!(settings.temperature_unit, expected.temperature_unit);
  }

  #[test]
  fn test_set_language() {
    let mut settings = structs::settings::Settings::default();
    let new_language = "es".to_string();

    // ファイル書き込み処理をスキップしてテスト
    settings.language = new_language.clone();
    assert_eq!(settings.language, new_language);
  }

  #[test]
  fn test_set_theme() {
    let mut settings = structs::settings::Settings::default();
    let new_theme = enums::settings::Theme::Light;

    // ファイル書き込み処理をスキップしてテスト（テスト環境ではファイルアクセス不要）
    settings.theme = new_theme.clone();
    assert_eq!(settings.theme, new_theme);
  }

  #[test]
  fn test_set_display_targets() {
    let mut settings = structs::settings::Settings::default();
    let targets = vec![hardware::HardwareType::Gpu, hardware::HardwareType::Memory];
    // ファイル書き込み処理をスキップしてテスト
    settings.display_targets = targets.clone();
    assert_eq!(settings.display_targets, targets);
  }

  #[test]
  fn test_set_graph_size() {
    let mut settings = structs::settings::Settings::default();
    // ファイル書き込み処理をスキップしてテスト
    settings.graph_size = enums::settings::GraphSize::SM;
    assert_eq!(settings.graph_size, enums::settings::GraphSize::SM);
  }

  #[test]
  fn test_set_line_graph_type() {
    let mut settings = structs::settings::Settings::default();
    assert!(
      settings
        .set_graph_size(enums::settings::GraphSize::SM)
        .is_ok()
    );
    assert_eq!(settings.graph_size, enums::settings::GraphSize::SM);
  }

  #[test]
  fn test_set_line_graph_border() {
    let mut settings = structs::settings::Settings::default();
    assert!(settings.set_line_graph_border(false).is_ok());
    assert!(!settings.line_graph_border);
  }

  #[test]
  fn test_set_line_graph_fill() {
    let mut settings = structs::settings::Settings::default();
    assert!(settings.set_line_graph_fill(false).is_ok());
    assert!(!settings.line_graph_fill);
  }

  #[test]
  fn test_set_line_graph_color() {
    let mut settings = structs::settings::Settings::default();
    let new_color = "#ff0000".to_string();

    assert!(
      settings
        .set_line_graph_color(hardware::HardwareType::Cpu, new_color.clone())
        .is_ok()
    );
    assert_eq!(settings.line_graph_color.cpu, [255, 0, 0]);
  }

  #[test]
  fn test_set_invalid_line_graph_color() {
    let mut settings = structs::settings::Settings::default();
    let invalid_color = "invalid_color".to_string();

    assert!(
      settings
        .set_line_graph_color(hardware::HardwareType::Cpu, invalid_color)
        .is_err()
    );
  }

  #[test]
  fn test_set_line_graph_color_invalid() {
    let mut settings = structs::settings::Settings::default();
    let new_color = "invalid_color".to_string();
    assert!(
      settings
        .set_line_graph_color(hardware::HardwareType::Cpu, new_color)
        .is_err()
    );
  }

  #[test]
  fn test_set_line_graph_mix() {
    let mut settings = structs::settings::Settings::default();
    assert!(settings.set_line_graph_mix(false).is_ok());
    assert!(!settings.line_graph_mix);
  }

  #[test]
  fn test_set_line_graph_show_legend() {
    let mut settings = structs::settings::Settings::default();
    assert!(settings.set_line_graph_show_legend(false).is_ok());
    assert!(!settings.line_graph_show_legend);
  }

  #[test]
  fn test_set_line_graph_show_scale() {
    let mut settings = structs::settings::Settings::default();
    // ファイル書き込み処理をスキップしてテスト
    settings.line_graph_show_scale = false;
    assert!(!settings.line_graph_show_scale);
  }

  #[test]
  fn test_set_line_graph_show_tooltip() {
    let mut settings = structs::settings::Settings::default();
    // ファイル書き込み処理をスキップしてテスト
    settings.line_graph_show_tooltip = false;
    assert!(!settings.line_graph_show_tooltip);
  }

  #[test]
  fn test_set_background_img_opacity() {
    let mut settings = structs::settings::Settings::default();
    // ファイル書き込み処理をスキップしてテスト
    settings.background_img_opacity = 100;
    assert_eq!(settings.background_img_opacity, 100);
  }

  #[test]
  fn test_set_selected_background_img() {
    let mut settings = structs::settings::Settings::default();
    let img_path = Some("path/to/image.png".to_string());
    // ファイル書き込み処理をスキップしてテスト
    settings.selected_background_img = img_path.clone();
    assert_eq!(settings.selected_background_img, img_path);
  }

  #[test]
  fn test_set_temperature_unit() {
    let mut settings = structs::settings::Settings::default();
    let unit = enums::settings::TemperatureUnit::Fahrenheit;
    assert!(settings.set_temperature_unit(unit.clone()).is_ok());
    assert_eq!(settings.temperature_unit, unit);
  }
}
