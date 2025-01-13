use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specta::Type;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum HardwareType {
  #[serde(rename = "CPU")]
  Cpu,
  Memory,
  #[serde(rename = "GPU")]
  Gpu,
}

impl Serialize for HardwareType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = match *self {
      HardwareType::Cpu => "cpu",
      HardwareType::Memory => "memory",
      HardwareType::Gpu => "gpu",
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
      "cpu" => Ok(HardwareType::Cpu),
      "memory" => Ok(HardwareType::Memory),
      "gpu" => Ok(HardwareType::Gpu),
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
      sysinfo::DiskKind::HDD => DiskKind::Hdd,

      sysinfo::DiskKind::SSD => DiskKind::Ssd,

      _ => DiskKind::Unknown,
    }
  }
}

#[derive(Serialize, Deserialize, Type, Debug, PartialEq, Eq, Clone)]
pub enum DiskKind {
  #[serde(rename = "hdd")]
  Hdd,
  #[serde(rename = "ssd")]
  Ssd,
  #[serde(rename = "other")]
  Unknown,
}

impl fmt::Display for DiskKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match *self {
      DiskKind::Hdd => "HDD",
      DiskKind::Ssd => "SSD",
      _ => "Other",
    })
  }
}
