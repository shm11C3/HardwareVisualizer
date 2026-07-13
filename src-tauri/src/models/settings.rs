use crate::enums;
use crate::models;
use crate::services;
use crate::utils;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct LineGraphColorSettings {
  pub cpu: [u8; 3],
  pub memory: [u8; 3],
  pub gpu: [u8; 3],
}

impl Default for LineGraphColorSettings {
  fn default() -> Self {
    Self {
      cpu: [75, 192, 192],
      memory: [255, 99, 132],
      gpu: [255, 206, 86],
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ExternalComponentGuidanceSettings {
  pub acknowledged_keys: Vec<String>,
}

///
/// ## App-owned settings persisted in `settings.json`.
///
/// Core-owned fields live in
/// [`hardviz_core::settings::CoreSettings`] and are persisted to the same
/// file under their own keys — they are intentionally absent here so
/// App-side setters never touch Core fields.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
  pub version: String,
  pub language: String,
  pub theme: enums::settings::Theme,
  pub navigation_layout: enums::settings::NavigationLayout,
  pub ui_announcement_version: u32,
  pub display_targets: Vec<enums::hardware::HardwareType>,
  pub graph_size: enums::settings::GraphSize,
  pub graph_fit_to_window: bool,
  pub graph_margin_px: u32,
  pub line_graph_type: enums::settings::LineGraphType,
  pub line_graph_border: bool,
  pub line_graph_fill: bool,
  pub line_graph_color: LineGraphColorSettings,
  pub line_graph_mix: bool,
  pub line_graph_show_legend: bool,
  pub line_graph_show_scale: bool,
  pub line_graph_show_tooltip: bool,
  pub background_img_opacity: u8,
  pub selected_background_img: Option<String>,
  pub transparent_ui: bool,
  pub window_opacity: u8,
  pub glass_blur: u8,
  pub temperature_unit: enums::settings::TemperatureUnit,
  pub burn_in_shift: bool,
  pub burn_in_shift_mode: enums::settings::BurnInShiftMode,
  pub burn_in_shift_preset: enums::settings::BurnInShiftPreset,
  pub burn_in_shift_idle_only: bool,
  pub burn_in_shift_options: Option<BurnInShiftOptions>,
  pub text_selectable: bool,
  pub close_to_tray: bool,
  pub close_to_tray_choice_made: bool,
  pub external_component_guidance: ExternalComponentGuidanceSettings,
  pub elevated_startup_mode: bool,
  pub tray_widget: crate::tray::widget::TrayWidgetSettings,
}

///
/// Structure of settings to send to client
///
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineGraphColorStringSettings {
  pub cpu: String,
  pub memory: String,
  pub gpu: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClientSettings {
  pub version: String,
  pub language: String,
  pub theme: enums::settings::Theme,
  pub navigation_layout: enums::settings::NavigationLayout,
  pub ui_announcement_version: u32,
  /// Current announcement schema version. This is wire metadata, not a
  /// persisted user preference.
  pub current_ui_announcement_version: u32,
  pub display_targets: Vec<enums::hardware::HardwareType>,
  pub graph_size: enums::settings::GraphSize,
  pub graph_fit_to_window: bool,
  pub graph_margin_px: u32,
  pub line_graph_type: enums::settings::LineGraphType,
  pub line_graph_border: bool,
  pub line_graph_fill: bool,
  pub line_graph_color: LineGraphColorStringSettings,
  pub line_graph_mix: bool,
  pub line_graph_show_legend: bool,
  pub line_graph_show_scale: bool,
  pub line_graph_show_tooltip: bool,
  pub background_img_opacity: u8,
  pub selected_background_img: Option<String>,
  pub transparent_ui: bool,
  pub window_opacity: u8,
  pub glass_blur: u8,
  pub temperature_unit: enums::settings::TemperatureUnit,
  pub hardware_archive: models::hardware_archive::HardwareArchiveSettings,
  pub storage_health: models::storage_health::StorageHealthSettings,
  pub burn_in_shift: bool,
  pub burn_in_shift_mode: enums::settings::BurnInShiftMode,
  pub burn_in_shift_preset: enums::settings::BurnInShiftPreset,
  pub burn_in_shift_idle_only: bool,
  pub burn_in_shift_options: Option<BurnInShiftOptions>,
  pub text_selectable: bool,
  pub close_to_tray: bool,
  pub close_to_tray_choice_made: bool,
  pub external_component_guidance: ExternalComponentGuidanceSettings,
  pub elevated_startup_mode: bool,
  pub tray_widget: crate::tray::widget::TrayWidgetSettings,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      version: utils::tauri::get_app_version(),
      language: services::language_service::get_default_language().to_string(),
      theme: enums::settings::Theme::System,
      navigation_layout: enums::settings::NavigationLayout::Grouped,
      ui_announcement_version: 0,
      display_targets: vec![
        enums::hardware::HardwareType::Cpu,
        enums::hardware::HardwareType::Memory,
        enums::hardware::HardwareType::Gpu,
      ],
      graph_size: enums::settings::GraphSize::XL,
      graph_fit_to_window: false,
      graph_margin_px: 32,
      line_graph_type: enums::settings::LineGraphType::Default,
      line_graph_border: true,
      line_graph_fill: true,
      line_graph_color: LineGraphColorSettings {
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
      transparent_ui: false,
      window_opacity: 86,
      glass_blur: 10,
      temperature_unit: enums::settings::TemperatureUnit::Celsius,
      burn_in_shift: false,
      burn_in_shift_mode: enums::settings::BurnInShiftMode::Jump,
      burn_in_shift_preset: enums::settings::BurnInShiftPreset::Aggressive,
      burn_in_shift_idle_only: true,
      burn_in_shift_options: None,
      text_selectable: false,
      close_to_tray: false,
      close_to_tray_choice_made: false,
      external_component_guidance: ExternalComponentGuidanceSettings::default(),
      elevated_startup_mode: false,
      tray_widget: crate::tray::widget::TrayWidgetSettings::default(),
    }
  }
}
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct BurnInShiftOptions {
  /// Override interval (ms) for jump
  interval_ms: Option<u32>,
  /// Override amplitude (px) for jump [x, y]
  amplitude_px: Option<[u32; 2]>,
  /// Idle threshold in ms
  idle_threshold_ms: Option<u32>,
  /// Drift cycle duration (sec)
  drift_duration_sec: Option<u32>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json;

  #[test]
  fn test_line_graph_color_settings_serialization() {
    let color_settings = LineGraphColorSettings {
      cpu: [75, 192, 192],
      memory: [255, 99, 132],
      gpu: [255, 206, 86],
    };

    let serialized = serde_json::to_string(&color_settings).unwrap();
    let deserialized: LineGraphColorSettings = serde_json::from_str(&serialized).unwrap();

    assert_eq!(color_settings.cpu, deserialized.cpu);
    assert_eq!(color_settings.memory, deserialized.memory);
    assert_eq!(color_settings.gpu, deserialized.gpu);
  }

  #[test]
  fn test_line_graph_color_settings_camel_case() {
    let color_settings = LineGraphColorSettings {
      cpu: [1, 2, 3],
      memory: [4, 5, 6],
      gpu: [7, 8, 9],
    };

    let serialized = serde_json::to_string(&color_settings).unwrap();

    assert!(serialized.contains("\"cpu\""));
    assert!(serialized.contains("\"memory\""));
    assert!(serialized.contains("\"gpu\""));
  }

  #[test]
  fn test_line_graph_color_string_settings() {
    let color_settings = LineGraphColorStringSettings {
      cpu: "#4BC0C0".to_string(),
      memory: "#FF6384".to_string(),
      gpu: "#FFCE56".to_string(),
    };

    let serialized = serde_json::to_string(&color_settings).unwrap();
    let deserialized: LineGraphColorStringSettings =
      serde_json::from_str(&serialized).unwrap();

    assert_eq!(color_settings.cpu, deserialized.cpu);
    assert_eq!(color_settings.memory, deserialized.memory);
    assert_eq!(color_settings.gpu, deserialized.gpu);
  }

  #[test]
  fn test_burn_in_shift_options_serialization() {
    let json_str = r#"{"intervalMs":1000,"amplitudePx":[10,20],"idleThresholdMs":5000,"driftDurationSec":30}"#;
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());
  }

  #[test]
  fn test_burn_in_shift_options_none_values() {
    let json_str = r#"{"intervalMs":null,"amplitudePx":null,"idleThresholdMs":null,"driftDurationSec":null}"#;
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());
  }

  #[test]
  fn test_settings_clone() {
    let settings = Settings::default();
    let cloned = settings.clone();

    assert_eq!(settings.language, cloned.language);
    assert_eq!(settings.theme, cloned.theme);
    assert_eq!(settings.graph_size, cloned.graph_size);
    assert_eq!(
      settings.background_img_opacity,
      cloned.background_img_opacity
    );

    assert_eq!(settings.line_graph_color.cpu, cloned.line_graph_color.cpu);
    assert_eq!(
      settings.line_graph_color.memory,
      cloned.line_graph_color.memory
    );
    assert_eq!(settings.line_graph_color.gpu, cloned.line_graph_color.gpu);
  }

  #[test]
  fn test_client_settings_clone() {
    use crate::enums;

    let client_settings = ClientSettings {
      version: "1.0.0".to_string(),
      language: "en".to_string(),
      theme: enums::settings::Theme::Dark,
      navigation_layout: enums::settings::NavigationLayout::Classic,
      ui_announcement_version: 1,
      current_ui_announcement_version: 1,
      display_targets: vec![enums::hardware::HardwareType::Cpu],
      graph_size: enums::settings::GraphSize::XL,
      graph_fit_to_window: false,
      graph_margin_px: 32,
      line_graph_type: enums::settings::LineGraphType::Default,
      line_graph_border: true,
      line_graph_fill: false,
      line_graph_color: LineGraphColorStringSettings {
        cpu: "#FF0000".to_string(),
        memory: "#00FF00".to_string(),
        gpu: "#0000FF".to_string(),
      },
      line_graph_mix: true,
      line_graph_show_legend: true,
      line_graph_show_scale: false,
      line_graph_show_tooltip: true,
      background_img_opacity: 75,
      selected_background_img: Some("test.png".to_string()),
      transparent_ui: false,
      window_opacity: 86,
      glass_blur: 10,
      temperature_unit: enums::settings::TemperatureUnit::Celsius,
      hardware_archive: crate::models::hardware_archive::HardwareArchiveSettings::default(
      ),
      storage_health: crate::models::storage_health::StorageHealthSettings::default(),
      burn_in_shift: false,
      burn_in_shift_mode: enums::settings::BurnInShiftMode::Jump,
      burn_in_shift_preset: enums::settings::BurnInShiftPreset::Aggressive,
      burn_in_shift_idle_only: true,
      burn_in_shift_options: None,
      text_selectable: false,
      close_to_tray: false,
      close_to_tray_choice_made: false,
      external_component_guidance: ExternalComponentGuidanceSettings::default(),
      elevated_startup_mode: false,
      tray_widget: crate::tray::widget::TrayWidgetSettings::default(),
    };

    let cloned = client_settings.clone();
    assert_eq!(client_settings.version, cloned.version);
    assert_eq!(client_settings.language, cloned.language);
    assert_eq!(
      client_settings.line_graph_color.cpu,
      cloned.line_graph_color.cpu
    );
  }

  #[test]
  fn test_settings_serialization_camel_case() {
    let settings = Settings::default();
    let serialized = serde_json::to_string(&settings).unwrap();

    assert!(serialized.contains("\"displayTargets\""));
    assert!(serialized.contains("\"graphSize\""));
    assert!(serialized.contains("\"graphFitToWindow\""));
    assert!(serialized.contains("\"graphMarginPx\""));
    assert!(serialized.contains("\"lineGraphType\""));
    assert!(serialized.contains("\"lineGraphBorder\""));
    assert!(serialized.contains("\"backgroundImgOpacity\""));
    assert!(serialized.contains("\"transparentUi\""));
    assert!(serialized.contains("\"windowOpacity\""));
    assert!(serialized.contains("\"glassBlur\""));
    assert!(serialized.contains("\"externalComponentGuidance\""));
    assert!(serialized.contains("\"elevatedStartupMode\""));
  }

  #[test]
  fn test_burn_in_shift_options_camel_case_serialization() {
    let json_str = r#"{"intervalMs":1000,"amplitudePx":[5,10],"idleThresholdMs":2000,"driftDurationSec":60}"#;
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());

    let invalid_json = r#"{"invalidField":1000}"#;
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(invalid_json);
    assert!(result.is_ok());
  }

  #[test]
  fn test_rgb_color_values_valid() {
    let color_settings = LineGraphColorSettings {
      cpu: [0, 255, 128],
      memory: [255, 0, 255],
      gpu: [128, 128, 128],
    };

    assert_eq!(color_settings.cpu.len(), 3);
    assert_eq!(color_settings.memory.len(), 3);
    assert_eq!(color_settings.gpu.len(), 3);

    assert_eq!(color_settings.cpu, [0, 255, 128]);
    assert_eq!(color_settings.memory, [255, 0, 255]);
    assert_eq!(color_settings.gpu, [128, 128, 128]);
  }

  #[test]
  fn test_settings_deserialization_with_missing_fields() {
    // Simulate an old settings.json that lacks fields added in newer versions
    // (e.g., burnInShift*, textSelectable). With #[serde(default)], missing
    // fields should fall back to their defaults instead of failing entirely.
    // The `hardwareArchive` key here is a Core-owned key; the App-side
    // Settings struct ignores it via `#[serde(deny_unknown_fields)]`-free
    // defaults — this asserts we don't break when both buckets share a file.
    let old_json = r#"{
      "version": "0.1.0",
      "language": "en",
      "theme": "dark",
      "displayTargets": ["cpu", "memory"],
      "graphSize": "xl",
      "lineGraphType": "default",
      "lineGraphBorder": true,
      "lineGraphFill": false,
      "lineGraphColor": { "cpu": [10,20,30], "memory": [40,50,60], "gpu": [70,80,90] },
      "lineGraphMix": false,
      "lineGraphShowLegend": false,
      "lineGraphShowScale": true,
      "lineGraphShowTooltip": false,
      "backgroundImgOpacity": 80,
      "selectedBackgroundImg": null,
      "temperatureUnit": "F",
      "hardwareArchive": { "enabled": false, "scheduledDataDeletion": false, "retentionDays": 7 }
    }"#;

    let settings: Settings = serde_json::from_str(old_json).unwrap();

    // Existing fields should be preserved
    assert_eq!(settings.version, "0.1.0");
    assert_eq!(settings.language, "en");
    assert_eq!(settings.theme, enums::settings::Theme::Dark);
    assert_eq!(settings.display_targets.len(), 2);
    assert!(!settings.line_graph_fill);
    assert_eq!(settings.line_graph_color.cpu, [10, 20, 30]);
    assert!(!settings.line_graph_mix);
    assert!(settings.line_graph_show_scale);
    assert_eq!(settings.background_img_opacity, 80);
    assert_eq!(
      settings.temperature_unit,
      enums::settings::TemperatureUnit::Fahrenheit
    );

    // Missing fields should have default values
    let defaults = Settings::default();
    assert_eq!(settings.graph_fit_to_window, defaults.graph_fit_to_window);
    assert_eq!(settings.graph_margin_px, defaults.graph_margin_px);
    assert_eq!(settings.burn_in_shift, defaults.burn_in_shift);
    assert_eq!(settings.burn_in_shift_mode, defaults.burn_in_shift_mode);
    assert_eq!(settings.burn_in_shift_preset, defaults.burn_in_shift_preset);
    assert_eq!(
      settings.burn_in_shift_idle_only,
      defaults.burn_in_shift_idle_only
    );
    assert!(settings.burn_in_shift_options.is_none());
    assert_eq!(settings.text_selectable, defaults.text_selectable);
    assert_eq!(settings.transparent_ui, defaults.transparent_ui);
    assert_eq!(settings.window_opacity, defaults.window_opacity);
    assert_eq!(settings.glass_blur, defaults.glass_blur);
    assert_eq!(
      settings.elevated_startup_mode,
      defaults.elevated_startup_mode
    );
  }

  #[test]
  fn test_settings_deserialization_minimal_json() {
    // Even an empty JSON object should deserialize using all defaults
    let minimal_json = "{}";
    let settings: Settings = serde_json::from_str(minimal_json).unwrap();
    let defaults = Settings::default();

    assert_eq!(settings.theme, defaults.theme);
    assert_eq!(settings.navigation_layout, defaults.navigation_layout);
    assert_eq!(
      settings.ui_announcement_version,
      defaults.ui_announcement_version
    );
    assert_eq!(settings.graph_size, defaults.graph_size);
    assert_eq!(settings.line_graph_border, defaults.line_graph_border);
    assert_eq!(settings.burn_in_shift, defaults.burn_in_shift);
    assert_eq!(settings.text_selectable, defaults.text_selectable);
    assert_eq!(settings.transparent_ui, defaults.transparent_ui);
    assert_eq!(settings.window_opacity, defaults.window_opacity);
    assert_eq!(settings.glass_blur, defaults.glass_blur);
  }

  #[test]
  fn external_component_guidance_settings_default_to_no_acknowledged_keys() {
    let settings = Settings::default();

    assert!(
      settings
        .external_component_guidance
        .acknowledged_keys
        .is_empty()
    );
  }

  #[test]
  fn merge_from_json_str_recovers_external_component_guidance_keys() {
    let mut settings = Settings::default();

    settings
      .merge_from_json_str(
        r#"{"externalComponentGuidance":{"acknowledgedKeys":["smartctl:storage-health:v1"]}}"#,
      )
      .unwrap();

    assert_eq!(
      settings.external_component_guidance.acknowledged_keys,
      vec!["smartctl:storage-health:v1".to_string()]
    );
  }

  #[test]
  fn test_merge_from_json_str_recovers_valid_fields() {
    // JSON with an invalid theme value — full deserialization would fail,
    // but field-level recovery should preserve all other valid fields.
    let json_with_invalid_theme = r#"{
      "version": "0.5.0",
      "language": "ja",
      "theme": "nonexistent_theme",
      "graphSize": "lg",
      "lineGraphBorder": false,
      "lineGraphFill": false,
      "backgroundImgOpacity": 90,
      "transparentUi": true,
      "windowOpacity": 64,
      "glassBlur": 18,
      "temperatureUnit": "F",
      "burnInShift": true
    }"#;

    let mut settings = Settings::default();
    let result = settings.merge_from_json_str(json_with_invalid_theme);
    assert!(result.is_ok());

    // Valid fields should be recovered
    assert_eq!(settings.version, "0.5.0");
    assert_eq!(settings.language, "ja");
    assert_eq!(settings.graph_size, enums::settings::GraphSize::LG);
    assert!(!settings.line_graph_border);
    assert!(!settings.line_graph_fill);
    assert_eq!(settings.background_img_opacity, 90);
    assert!(settings.transparent_ui);
    assert_eq!(settings.window_opacity, 64);
    assert_eq!(settings.glass_blur, 18);
    assert_eq!(
      settings.temperature_unit,
      enums::settings::TemperatureUnit::Fahrenheit
    );
    assert!(settings.burn_in_shift);

    // Invalid theme should fall back to default
    let defaults = Settings::default();
    assert_eq!(settings.theme, defaults.theme);

    // Missing fields should remain at defaults
    assert_eq!(settings.line_graph_mix, defaults.line_graph_mix);
    assert_eq!(settings.text_selectable, defaults.text_selectable);
  }

  #[test]
  fn merge_from_json_str_keeps_grouped_default_for_invalid_navigation_layout() {
    let mut settings = Settings::default();

    settings
      .merge_from_json_str(r#"{"navigationLayout":"future"}"#)
      .unwrap();

    assert_eq!(
      settings.navigation_layout,
      enums::settings::NavigationLayout::Grouped
    );
  }

  #[test]
  fn test_merge_from_json_str_clamps_window_opacity() {
    let mut settings = Settings::default();
    settings
      .merge_from_json_str(r#"{"windowOpacity": 5}"#)
      .unwrap();
    assert_eq!(settings.window_opacity, 20);

    settings
      .merge_from_json_str(r#"{"windowOpacity": 150}"#)
      .unwrap();
    assert_eq!(settings.window_opacity, 100);
  }

  #[test]
  fn test_merge_from_json_str_clamps_glass_blur() {
    let mut settings = Settings::default();
    settings
      .merge_from_json_str(r#"{"glassBlur": 200}"#)
      .unwrap();
    assert_eq!(settings.glass_blur, 30);
  }

  #[test]
  fn test_merge_from_json_str_invalid_json() {
    let mut settings = Settings::default();
    let result = settings.merge_from_json_str("not valid json {{{");
    assert!(result.is_err());
  }
}
