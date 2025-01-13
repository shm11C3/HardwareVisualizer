use crate::enums;
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
  pub theme: enums::config::Theme,
  pub display_targets: Vec<enums::hardware::HardwareType>,
  pub graph_size: enums::config::GraphSize,
  pub line_graph_border: bool,
  pub line_graph_fill: bool,
  pub line_graph_color: LineGraphColorSettings,
  pub line_graph_mix: bool,
  pub line_graph_show_legend: bool,
  pub line_graph_show_scale: bool,
  pub background_img_opacity: u8,
  pub selected_background_img: Option<String>,
  pub temperature_unit: enums::config::TemperatureUnit,
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
  pub language: String,
  pub theme: enums::config::Theme,
  pub display_targets: Vec<enums::hardware::HardwareType>,
  pub graph_size: enums::config::GraphSize,
  pub line_graph_border: bool,
  pub line_graph_fill: bool,
  pub line_graph_color: LineGraphColorStringSettings,
  pub line_graph_mix: bool,
  pub line_graph_show_legend: bool,
  pub line_graph_show_scale: bool,
  pub background_img_opacity: u8,
  pub selected_background_img: Option<String>,
  pub temperature_unit: enums::config::TemperatureUnit,
}
