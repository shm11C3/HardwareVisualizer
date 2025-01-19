use crate::enums;
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
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      version: utils::tauri::get_app_version(&utils::tauri::get_config()),
      language: services::language::get_default_language().to_string(),
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
    }
  }
}
