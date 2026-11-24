//! Wire-format `SizeUnit` for tauri-specta.
//!
//! The runtime helpers (`format_size`, `format_size_with_unit`, `format_vendor_name`,
//! `format_temperature`, `RoundedKibibytes`) live in `hardviz_core::utils::formatter`.
//! This module keeps only the specta-derived enum that is embedded in
//! `crate::models::hardware::StorageInfo` (a Tauri command return) plus
//! conversions between the wire enum and the Core POJO mirror.

use hardviz_core::utils::formatter as core_formatter;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;

#[derive(Serialize, Deserialize, Type, Debug, PartialEq, Eq, Clone)]
pub enum SizeUnit {
  #[serde(rename = "B")]
  Bytes,
  #[serde(rename = "KB")]
  KBytes,
  #[serde(rename = "MB")]
  MBytes,
  #[serde(rename = "GB")]
  GBytes,
}

impl fmt::Display for SizeUnit {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match *self {
      SizeUnit::Bytes => "B",
      SizeUnit::KBytes => "KB",
      SizeUnit::MBytes => "MB",
      SizeUnit::GBytes => "GB",
    })
  }
}

impl From<core_formatter::SizeUnit> for SizeUnit {
  fn from(src: core_formatter::SizeUnit) -> Self {
    match src {
      core_formatter::SizeUnit::Bytes => Self::Bytes,
      core_formatter::SizeUnit::KBytes => Self::KBytes,
      core_formatter::SizeUnit::MBytes => Self::MBytes,
      core_formatter::SizeUnit::GBytes => Self::GBytes,
    }
  }
}

impl From<SizeUnit> for core_formatter::SizeUnit {
  fn from(src: SizeUnit) -> Self {
    match src {
      SizeUnit::Bytes => Self::Bytes,
      SizeUnit::KBytes => Self::KBytes,
      SizeUnit::MBytes => Self::MBytes,
      SizeUnit::GBytes => Self::GBytes,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_size_unit_display() {
    assert_eq!(SizeUnit::Bytes.to_string(), "B");
    assert_eq!(SizeUnit::KBytes.to_string(), "KB");
    assert_eq!(SizeUnit::MBytes.to_string(), "MB");
    assert_eq!(SizeUnit::GBytes.to_string(), "GB");
  }

  #[test]
  fn test_size_unit_round_trip_to_core() {
    let cases = [
      SizeUnit::Bytes,
      SizeUnit::KBytes,
      SizeUnit::MBytes,
      SizeUnit::GBytes,
    ];
    for u in cases {
      let core: core_formatter::SizeUnit = u.clone().into();
      let back: SizeUnit = core.into();
      assert_eq!(u, back);
    }
  }
}
