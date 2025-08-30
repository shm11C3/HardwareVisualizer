use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specta::Type;

#[derive(Debug, PartialEq, Eq, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
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
