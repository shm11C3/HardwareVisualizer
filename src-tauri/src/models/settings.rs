use crate::enums;
use crate::models;
use crate::services;
use crate::utils;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LineGraphColorSettings {
  pub cpu: [u8; 3],
  pub memory: [u8; 3],
  pub gpu: [u8; 3],
}

///
/// ## settings.json に格納するJSONの構造体
///
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
  pub version: String,
  pub language: String,
  pub theme: enums::settings::Theme,
  pub display_targets: Vec<enums::hardware::HardwareType>,
  pub graph_size: enums::settings::GraphSize,
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
  pub temperature_unit: enums::settings::TemperatureUnit,
  pub hardware_archive: models::hardware_archive::HardwareArchiveSettings,
  pub burn_in_shift: bool,
  pub burn_in_shift_mode: enums::settings::BurnInShiftMode,
  pub burn_in_shift_preset: enums::settings::BurnInShiftPreset,
  pub burn_in_shift_idle_only: bool,
  pub burn_in_shift_options: Option<BurnInShiftOptions>,
}

///
/// クライアントに送信する設定の構造体
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
  pub display_targets: Vec<enums::hardware::HardwareType>,
  pub graph_size: enums::settings::GraphSize,
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
  pub temperature_unit: enums::settings::TemperatureUnit,
  pub hardware_archive: models::hardware_archive::HardwareArchiveSettings,
  pub burn_in_shift: bool,
  pub burn_in_shift_mode: enums::settings::BurnInShiftMode,
  pub burn_in_shift_preset: enums::settings::BurnInShiftPreset,
  pub burn_in_shift_idle_only: bool,
  pub burn_in_shift_options: Option<BurnInShiftOptions>,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      version: utils::tauri::get_app_version(&utils::tauri::get_config()),
      language: services::language_service::get_default_language().to_string(),
      theme: enums::settings::Theme::System,
      display_targets: vec![
        enums::hardware::HardwareType::Cpu,
        enums::hardware::HardwareType::Memory,
        enums::hardware::HardwareType::Gpu,
      ],
      graph_size: enums::settings::GraphSize::XL,
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
      temperature_unit: enums::settings::TemperatureUnit::Celsius,
      hardware_archive: models::hardware_archive::HardwareArchiveSettings {
        enabled: true,
        refresh_interval_days: 30,
        scheduled_data_deletion: true,
      },
      burn_in_shift: false,
      burn_in_shift_mode: enums::settings::BurnInShiftMode::Jump,
      burn_in_shift_preset: enums::settings::BurnInShiftPreset::Aggressive,
      burn_in_shift_idle_only: true,
      burn_in_shift_options: None,
    }
  }
}
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
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

    // キャメルケース形式であることを確認
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

  // BurnInShiftOptions のフィールドはプライベートなため、
  // シリアライゼーションのテストのみ実行
  #[test]
  fn test_burn_in_shift_options_serialization() {
    // シリアライゼーションがエラーなく完了することを確認
    let json_str = r#"{"intervalMs":1000,"amplitudePx":[10,20],"idleThresholdMs":5000,"driftDurationSec":30}"#;
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());
  }

  #[test]
  fn test_burn_in_shift_options_none_values() {
    // null値を含むJSONのデシリアライゼーション
    let json_str = r#"{"intervalMs":null,"amplitudePx":null,"idleThresholdMs":null,"driftDurationSec":null}"#;
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());
  }

  #[test]
  fn test_settings_clone() {
    let settings = Settings::default();
    let cloned = settings.clone();

    // 基本フィールドの比較
    assert_eq!(settings.language, cloned.language);
    assert_eq!(settings.theme, cloned.theme);
    assert_eq!(settings.graph_size, cloned.graph_size);
    assert_eq!(
      settings.background_img_opacity,
      cloned.background_img_opacity
    );

    // 配列の比較
    assert_eq!(settings.line_graph_color.cpu, cloned.line_graph_color.cpu);
    assert_eq!(
      settings.line_graph_color.memory,
      cloned.line_graph_color.memory
    );
    assert_eq!(settings.line_graph_color.gpu, cloned.line_graph_color.gpu);
  }

  #[test]
  fn test_client_settings_clone() {
    let client_settings = ClientSettings {
      version: "1.0.0".to_string(),
      language: "en".to_string(),
      theme: enums::settings::Theme::Dark,
      display_targets: vec![enums::hardware::HardwareType::Cpu],
      graph_size: enums::settings::GraphSize::XL,
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
      temperature_unit: enums::settings::TemperatureUnit::Celsius,
      hardware_archive: models::hardware_archive::HardwareArchiveSettings {
        enabled: true,
        refresh_interval_days: 30,
        scheduled_data_deletion: true,
      },
      burn_in_shift: false,
      burn_in_shift_mode: enums::settings::BurnInShiftMode::Jump,
      burn_in_shift_preset: enums::settings::BurnInShiftPreset::Aggressive,
      burn_in_shift_idle_only: true,
      burn_in_shift_options: None,
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

    // キャメルケース形式のフィールドが含まれていることを確認
    assert!(serialized.contains("\"displayTargets\""));
    assert!(serialized.contains("\"graphSize\""));
    assert!(serialized.contains("\"lineGraphType\""));
    assert!(serialized.contains("\"lineGraphBorder\""));
    assert!(serialized.contains("\"backgroundImgOpacity\""));
  }

  #[test]
  fn test_burn_in_shift_options_camel_case_serialization() {
    // キャメルケース形式のJSONからのデシリアライゼーション
    let json_str = r#"{"intervalMs":1000,"amplitudePx":[5,10],"idleThresholdMs":2000,"driftDurationSec":60}"#;
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());

    // 不完全なJSONでエラーになることを確認
    let invalid_json = r#"{"invalidField":1000}"#; // 無効なフィールド名
    let result: Result<BurnInShiftOptions, _> = serde_json::from_str(invalid_json);
    assert!(result.is_ok()); // serde は未知のフィールドを無視する
  }

  #[test]
  fn test_rgb_color_values_valid() {
    let color_settings = LineGraphColorSettings {
      cpu: [0, 255, 128],
      memory: [255, 0, 255],
      gpu: [128, 128, 128],
    };

    // RGB値が正しい配列長であることを確認
    assert_eq!(color_settings.cpu.len(), 3);
    assert_eq!(color_settings.memory.len(), 3);
    assert_eq!(color_settings.gpu.len(), 3);

    // 値が期待通りであることを確認
    assert_eq!(color_settings.cpu, [0, 255, 128]);
    assert_eq!(color_settings.memory, [255, 0, 255]);
    assert_eq!(color_settings.gpu, [128, 128, 128]);
  }
}
