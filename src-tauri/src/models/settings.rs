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
      theme: enums::settings::Theme::Dark,
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
