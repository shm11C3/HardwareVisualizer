use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specta::Type;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum HardwareType {
  CPU,
  Memory,
  GPU,
}

impl Serialize for HardwareType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = match *self {
      HardwareType::CPU => "cpu",
      HardwareType::Memory => "memory",
      HardwareType::GPU => "gpu",
    };
    serializer.serialize_str(s)
  }
}

impl<'de> Deserialize<'de> for HardwareType {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?.to_lowercase();
    match s.as_str() {
      "cpu" => Ok(HardwareType::CPU),
      "memory" => Ok(HardwareType::Memory),
      "gpu" => Ok(HardwareType::GPU),
      _ => Err(serde::de::Error::unknown_variant(
        &s,
        &["cpu", "memory", "gpu"],
      )),
    }
  }
}

impl From<sysinfo::DiskKind> for DiskKind {
  fn from(kind: sysinfo::DiskKind) -> Self {
    match kind {
      sysinfo::DiskKind::HDD => DiskKind::HDD,

      sysinfo::DiskKind::SSD => DiskKind::SSD,

      _ => DiskKind::Unknown,
    }
  }
}

#[derive(Serialize, Deserialize, Type, Debug, PartialEq, Eq, Clone)]
pub enum DiskKind {
  #[serde(rename = "hdd")]
  HDD,
  #[serde(rename = "ssd")]
  SSD,
  #[serde(rename = "other")]
  Unknown,
}

impl fmt::Display for DiskKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match *self {
      DiskKind::HDD => "HDD",
      DiskKind::SSD => "SSD",
      _ => "Other",
    })
  }
}
