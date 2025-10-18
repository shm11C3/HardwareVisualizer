use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specta::Type;

#[derive(Debug, PartialEq, Eq, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
  System,
  Light,
  Dark,
  DarkPlus,
  Ocean,
  Grove,
  Sunset,
  Nebula,
  Orbit,
  Cappuccino,
  Espresso,
}

impl Serialize for Theme {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = match *self {
      Theme::System => "system",
      Theme::Light => "light",
      Theme::Dark => "dark",
      Theme::DarkPlus => "darkPlus",
      Theme::Ocean => "sky",
      Theme::Grove => "grove",
      Theme::Sunset => "sunset",
      Theme::Nebula => "nebula",
      Theme::Orbit => "orbit",
      Theme::Cappuccino => "cappuccino",
      Theme::Espresso => "espresso",
    };
    serializer.serialize_str(s)
  }
}

impl<'de> Deserialize<'de> for Theme {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?.to_lowercase();
    match s.as_str() {
      "system" => Ok(Theme::System),
      "light" => Ok(Theme::Light),
      "dark" => Ok(Theme::Dark),
      "darkplus" => Ok(Theme::DarkPlus),
      "sky" => Ok(Theme::Ocean),
      "grove" => Ok(Theme::Grove),
      "sunset" => Ok(Theme::Sunset),
      "nebula" => Ok(Theme::Nebula),
      "orbit" => Ok(Theme::Orbit),
      "cappuccino" => Ok(Theme::Cappuccino),
      "espresso" => Ok(Theme::Espresso),
      _ => Err(serde::de::Error::unknown_variant(
        &s,
        &[
          "system",
          "light",
          "dark",
          "darkPlus",
          "sky",
          "grove",
          "sunset",
          "nebula",
          "orbit",
          "cappuccino",
          "espresso",
        ],
      )),
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum GraphSize {
  SM,
  MD,
  LG,
  XL,
  #[serde(rename = "2xl")]
  _2XL,
}

impl Serialize for GraphSize {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = match *self {
      GraphSize::SM => "sm",
      GraphSize::MD => "md",
      GraphSize::LG => "lg",
      GraphSize::XL => "xl",
      GraphSize::_2XL => "2xl",
    };
    serializer.serialize_str(s)
  }
}

impl<'de> Deserialize<'de> for GraphSize {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?.to_lowercase();
    match s.as_str() {
      "sm" => Ok(GraphSize::SM),
      "md" => Ok(GraphSize::MD),
      "lg" => Ok(GraphSize::LG),
      "xl" => Ok(GraphSize::XL),
      "2xl" => Ok(GraphSize::_2XL),
      _ => Err(serde::de::Error::unknown_variant(
        &s,
        &["sm", "md", "lg", "sl", "2xl"],
      )),
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Type)]
pub enum TemperatureUnit {
  #[serde(rename = "C")]
  Celsius,
  #[serde(rename = "F")]
  Fahrenheit,
}

impl Serialize for TemperatureUnit {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = match *self {
      TemperatureUnit::Celsius => "C",
      TemperatureUnit::Fahrenheit => "F",
    };
    serializer.serialize_str(s)
  }
}

impl<'de> Deserialize<'de> for TemperatureUnit {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?.to_lowercase();
    match s.as_str() {
      "C" => Ok(TemperatureUnit::Celsius),
      "c" => Ok(TemperatureUnit::Celsius),
      "F" => Ok(TemperatureUnit::Fahrenheit),
      "f" => Ok(TemperatureUnit::Fahrenheit),
      _ => Err(serde::de::Error::unknown_variant(&s, &["C", "F"])),
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Type, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineGraphType {
  // monotone
  Default,
  Step,
  Linear,
  Basis,
}

#[derive(Debug, PartialEq, Eq, Clone, Type, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BurnInShiftMode {
  Jump,
  Drift,
}

#[derive(Debug, PartialEq, Eq, Clone, Type, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BurnInShiftPreset {
  Gentle,
  Balanced,
  Aggressive,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json;

  #[test]
  fn test_theme_serialization() {
    let test_cases = vec![
      (Theme::Light, "light"),
      (Theme::Dark, "dark"),
      (Theme::DarkPlus, "darkPlus"),
      (Theme::Ocean, "sky"),
      (Theme::Grove, "grove"),
      (Theme::Sunset, "sunset"),
      (Theme::Nebula, "nebula"),
      (Theme::Orbit, "orbit"),
      (Theme::Cappuccino, "cappuccino"),
      (Theme::Espresso, "espresso"),
    ];

    for (theme, expected_json) in test_cases {
      let serialized = serde_json::to_string(&theme).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_theme_deserialization() {
    let test_cases = vec![
      ("\"light\"", Theme::Light),
      ("\"dark\"", Theme::Dark),
      ("\"darkplus\"", Theme::DarkPlus),
      ("\"sky\"", Theme::Ocean),
      ("\"grove\"", Theme::Grove),
      ("\"sunset\"", Theme::Sunset),
      ("\"nebula\"", Theme::Nebula),
      ("\"orbit\"", Theme::Orbit),
      ("\"cappuccino\"", Theme::Cappuccino),
      ("\"espresso\"", Theme::Espresso),
      // 大文字小文字の変換テスト
      ("\"LIGHT\"", Theme::Light),
      ("\"DARK\"", Theme::Dark),
      ("\"SKY\"", Theme::Ocean),
    ];

    for (json_str, expected_theme) in test_cases {
      let deserialized: Theme = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_theme);
    }
  }

  #[test]
  fn test_theme_deserialization_invalid() {
    let invalid_cases = vec!["\"invalid\"", "\"purple\"", "\"red\""];

    for invalid_json in invalid_cases {
      let result: Result<Theme, _> = serde_json::from_str(invalid_json);
      assert!(result.is_err());
    }
  }

  #[test]
  fn test_graph_size_serialization() {
    let test_cases = vec![
      (GraphSize::SM, "sm"),
      (GraphSize::MD, "md"),
      (GraphSize::LG, "lg"),
      (GraphSize::XL, "xl"),
      (GraphSize::_2XL, "2xl"),
    ];

    for (size, expected_json) in test_cases {
      let serialized = serde_json::to_string(&size).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_graph_size_deserialization() {
    let test_cases = vec![
      ("\"sm\"", GraphSize::SM),
      ("\"md\"", GraphSize::MD),
      ("\"lg\"", GraphSize::LG),
      ("\"xl\"", GraphSize::XL),
      ("\"2xl\"", GraphSize::_2XL),
      // 大文字小文字の変換テスト
      ("\"SM\"", GraphSize::SM),
      ("\"MD\"", GraphSize::MD),
      ("\"LG\"", GraphSize::LG),
      ("\"XL\"", GraphSize::XL),
      ("\"2XL\"", GraphSize::_2XL),
    ];

    for (json_str, expected_size) in test_cases {
      let deserialized: GraphSize = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_size);
    }
  }

  #[test]
  fn test_graph_size_deserialization_invalid() {
    let invalid_cases = vec!["\"xs\"", "\"3xl\"", "\"large\""];

    for invalid_json in invalid_cases {
      let result: Result<GraphSize, _> = serde_json::from_str(invalid_json);
      assert!(result.is_err());
    }
  }

  #[test]
  fn test_temperature_unit_serialization() {
    let test_cases = vec![
      (TemperatureUnit::Celsius, "C"),
      (TemperatureUnit::Fahrenheit, "F"),
    ];

    for (unit, expected_json) in test_cases {
      let serialized = serde_json::to_string(&unit).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_temperature_unit_deserialization() {
    let test_cases = vec![
      ("\"C\"", TemperatureUnit::Celsius),
      ("\"c\"", TemperatureUnit::Celsius),
      ("\"F\"", TemperatureUnit::Fahrenheit),
      ("\"f\"", TemperatureUnit::Fahrenheit),
    ];

    for (json_str, expected_unit) in test_cases {
      let deserialized: TemperatureUnit = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_unit);
    }
  }

  #[test]
  fn test_temperature_unit_deserialization_invalid() {
    let invalid_cases = vec!["\"celsius\"", "\"fahrenheit\"", "\"K\""];

    for invalid_json in invalid_cases {
      let result: Result<TemperatureUnit, _> = serde_json::from_str(invalid_json);
      assert!(result.is_err());
    }
  }

  #[test]
  fn test_line_graph_type_serialization() {
    let test_cases = vec![
      (LineGraphType::Default, "default"),
      (LineGraphType::Step, "step"),
      (LineGraphType::Linear, "linear"),
      (LineGraphType::Basis, "basis"),
    ];

    for (graph_type, expected_json) in test_cases {
      let serialized = serde_json::to_string(&graph_type).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_line_graph_type_deserialization() {
    let test_cases = vec![
      ("\"default\"", LineGraphType::Default),
      ("\"step\"", LineGraphType::Step),
      ("\"linear\"", LineGraphType::Linear),
      ("\"basis\"", LineGraphType::Basis),
    ];

    for (json_str, expected_type) in test_cases {
      let deserialized: LineGraphType = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_type);
    }
  }

  #[test]
  fn test_burn_in_shift_mode_serialization() {
    let test_cases = vec![
      (BurnInShiftMode::Jump, "jump"),
      (BurnInShiftMode::Drift, "drift"),
    ];

    for (mode, expected_json) in test_cases {
      let serialized = serde_json::to_string(&mode).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_burn_in_shift_mode_deserialization() {
    let test_cases = vec![
      ("\"jump\"", BurnInShiftMode::Jump),
      ("\"drift\"", BurnInShiftMode::Drift),
    ];

    for (json_str, expected_mode) in test_cases {
      let deserialized: BurnInShiftMode = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_mode);
    }
  }

  #[test]
  fn test_burn_in_shift_preset_serialization() {
    let test_cases = vec![
      (BurnInShiftPreset::Gentle, "gentle"),
      (BurnInShiftPreset::Balanced, "balanced"),
      (BurnInShiftPreset::Aggressive, "aggressive"),
    ];

    for (preset, expected_json) in test_cases {
      let serialized = serde_json::to_string(&preset).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_burn_in_shift_preset_deserialization() {
    let test_cases = vec![
      ("\"gentle\"", BurnInShiftPreset::Gentle),
      ("\"balanced\"", BurnInShiftPreset::Balanced),
      ("\"aggressive\"", BurnInShiftPreset::Aggressive),
    ];

    for (json_str, expected_preset) in test_cases {
      let deserialized: BurnInShiftPreset = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_preset);
    }
  }

  #[test]
  fn test_theme_clone_and_eq() {
    let original = Theme::Dark;
    let cloned = original.clone();
    assert_eq!(original, cloned);

    assert_eq!(Theme::Light, Theme::Light);
    assert_ne!(Theme::Light, Theme::Dark);
  }

  #[test]
  fn test_all_themes_covered() {
    // すべてのThemeバリアントがテストされていることを確認
    let all_themes = vec![
      Theme::Light,
      Theme::Dark,
      Theme::DarkPlus,
      Theme::Ocean,
      Theme::Grove,
      Theme::Sunset,
      Theme::Nebula,
      Theme::Orbit,
      Theme::Cappuccino,
      Theme::Espresso,
    ];

    // 各テーマがシリアライズ可能であることを確認
    for theme in all_themes {
      assert!(serde_json::to_string(&theme).is_ok());
    }
  }

  #[test]
  fn test_all_graph_sizes_covered() {
    // すべてのGraphSizeバリアントがテストされていることを確認
    let all_sizes = vec![
      GraphSize::SM,
      GraphSize::MD,
      GraphSize::LG,
      GraphSize::XL,
      GraphSize::_2XL,
    ];

    // 各サイズがシリアライズ可能であることを確認
    for size in all_sizes {
      assert!(serde_json::to_string(&size).is_ok());
    }
  }
}
