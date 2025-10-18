use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;

#[cfg(target_os = "windows")]
use crate::enums;

#[cfg(target_os = "windows")]
use nvapi::Kibibytes;

const KILOBYTE: u64 = 1024;
const MEGABYTE: u64 = KILOBYTE * 1024;
const GIGABYTE: u64 = MEGABYTE * 1024;

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

#[cfg(target_os = "windows")]
pub struct RoundedKibibytes {
  pub kibibytes: Kibibytes,
  pub precision: u32,
}

///
/// ## `Kibibytes` をフォーマット
///
#[cfg(target_os = "windows")]
impl fmt::Display for RoundedKibibytes {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let value = self.kibibytes.0; // Kibibytesの内部値を取得
    if value < 1000 {
      write!(f, "{value} KB")
    } else if value < 1000000 {
      let value_in_mib = value as f32 / 1024.0;
      write!(
        f,
        "{:.precision$} MB",
        value_in_mib,
        precision = self.precision as usize
      ) // 指定された桁数でフォーマット
    } else {
      let value_in_gib = value as f32 / 1048576.0;
      write!(
        f,
        "{:.precision$} GB",
        value_in_gib,
        precision = self.precision as usize
      ) // 指定された桁数でフォーマット
    }
  }
}

///
/// ## 小数を指定桁数で丸める（四捨五入）
///
pub fn round(num: f64, precision: u32) -> f64 {
  Decimal::from_f64(num)
    .map(|d| d.round_dp(precision).to_f64().unwrap_or(0.0))
    .unwrap_or(0.0)
}

///
/// ## バイト数を単位付きの文字列に変換
///
pub fn format_size(bytes: u64, precision: u32) -> String {
  if bytes >= GIGABYTE {
    format!(
      "{:.precision$} GB",
      round(bytes as f64 / GIGABYTE as f64, precision),
      precision = precision as usize
    )
  } else if bytes >= MEGABYTE {
    format!(
      "{:.precision$} MB",
      round(bytes as f64 / MEGABYTE as f64, precision),
      precision = precision as usize
    )
  } else if bytes >= KILOBYTE {
    format!(
      "{:.precision$} KB",
      round(bytes as f64 / KILOBYTE as f64, precision),
      precision = precision as usize
    )
  } else {
    format!("{bytes} bytes")
  }
}

#[derive(Debug, PartialEq)]
pub struct SizeWithUnit {
  pub value: f32,
  pub unit: SizeUnit,
}

///
/// ## バイト数をフォーマットし、単位と合わせて返却
///
pub fn format_size_with_unit(
  bytes: u64,
  precision: u32,
  unit: Option<SizeUnit>,
) -> SizeWithUnit {
  // 単位が指定されている場合は単位で丸めて返却
  if let Some(unit) = unit {
    SizeWithUnit {
      value: round(
        bytes as f64 / 1024.0_f64.powi(unit.clone() as i32),
        precision,
      ) as f32,
      unit,
    }
  } else if bytes >= GIGABYTE {
    SizeWithUnit {
      value: round(bytes as f64 / GIGABYTE as f64, precision) as f32,
      unit: SizeUnit::GBytes,
    }
  } else if bytes >= MEGABYTE {
    SizeWithUnit {
      value: round(bytes as f64 / MEGABYTE as f64, precision) as f32,
      unit: SizeUnit::MBytes,
    }
  } else if bytes >= KILOBYTE {
    SizeWithUnit {
      value: round(bytes as f64 / KILOBYTE as f64, precision) as f32,
      unit: SizeUnit::KBytes,
    }
  } else {
    SizeWithUnit {
      value: bytes as f32,
      unit: SizeUnit::Bytes,
    }
  }
}

///
/// ## ベンダー名をフォーマット
///
pub fn format_vendor_name(vendor_id: &str) -> String {
  match vendor_id {
    "GenuineIntel" => "Intel".to_string(),
    "AuthenticAMD" => "AMD".to_string(),
    _ => vendor_id.to_string(),
  }
}

#[cfg(target_os = "windows")]
pub fn format_temperature(
  current_unit: enums::settings::TemperatureUnit,
  unit: enums::settings::TemperatureUnit,
  value: i32,
) -> i32 {
  match (current_unit, unit) {
    (
      enums::settings::TemperatureUnit::Celsius,
      enums::settings::TemperatureUnit::Fahrenheit,
    ) => (value * 9 / 5) + 32,
    (
      enums::settings::TemperatureUnit::Fahrenheit,
      enums::settings::TemperatureUnit::Celsius,
    ) => (value - 32) * 5 / 9,
    _ => value,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(target_os = "windows")]
  use crate::enums;
  #[cfg(target_os = "windows")]
  use nvapi::Kibibytes;

  #[test]
  fn test_round() {
    assert_eq!(round(123.456, 2), 123.46);
    assert_eq!(round(123.451, 2), 123.45);
    assert_eq!(round(0.0, 2), 0.0);
  }

  #[test]
  fn test_format_size() {
    assert_eq!(format_size(500, 2), "500 bytes");
    assert_eq!(format_size(1024, 2), "1.00 KB");
    assert_eq!(format_size(1048576, 2), "1.00 MB");
    assert_eq!(format_size(1073741824, 2), "1.00 GB");
  }

  #[test]
  fn test_format_size_with_unit() {
    assert_eq!(
      format_size_with_unit(1024, 2, Some(SizeUnit::KBytes)),
      SizeWithUnit {
        value: 1.0,
        unit: SizeUnit::KBytes
      }
    );
    assert_eq!(
      format_size_with_unit(1048576, 2, Some(SizeUnit::MBytes)),
      SizeWithUnit {
        value: 1.0,
        unit: SizeUnit::MBytes
      }
    );
    assert_eq!(
      format_size_with_unit(1073741824, 2, None),
      SizeWithUnit {
        value: 1.0,
        unit: SizeUnit::GBytes
      }
    );
    assert_eq!(
      format_size_with_unit(500, 2, None),
      SizeWithUnit {
        value: 500.0,
        unit: SizeUnit::Bytes
      }
    );
  }

  #[test]
  fn test_format_vendor_name() {
    assert_eq!(format_vendor_name("GenuineIntel"), "Intel");
    assert_eq!(format_vendor_name("AuthenticAMD"), "AMD");
    assert_eq!(format_vendor_name("UnknownVendor"), "UnknownVendor");
  }

  #[test]
  fn test_size_unit_display() {
    assert_eq!(SizeUnit::Bytes.to_string(), "B");
    assert_eq!(SizeUnit::KBytes.to_string(), "KB");
    assert_eq!(SizeUnit::MBytes.to_string(), "MB");
    assert_eq!(SizeUnit::GBytes.to_string(), "GB");
  }

  #[test]
  #[cfg(target_os = "windows")]
  fn test_rounded_kibibytes_display() {
    let kibibytes = RoundedKibibytes {
      kibibytes: Kibibytes(500),
      precision: 2,
    };
    assert_eq!(kibibytes.to_string(), "500 KB");

    let kibibytes = RoundedKibibytes {
      kibibytes: Kibibytes(2048),
      precision: 2,
    };
    assert_eq!(kibibytes.to_string(), "2.00 MB");

    let kibibytes = RoundedKibibytes {
      kibibytes: Kibibytes(2 * 1024 * 1024),
      precision: 2,
    };
    assert_eq!(kibibytes.to_string(), "2.00 GB");
  }

  #[test]
  #[cfg(target_os = "windows")]
  fn test_celsius_to_fahrenheit() {
    let value = 100; // 100°C
    let result = format_temperature(
      enums::settings::TemperatureUnit::Celsius,
      enums::settings::TemperatureUnit::Fahrenheit,
      value,
    );
    assert_eq!(result, 212); // 100°C = 212°F
  }

  #[test]
  #[cfg(target_os = "windows")]
  fn test_fahrenheit_to_celsius() {
    let value = 212; // 212°F
    let result = format_temperature(
      enums::settings::TemperatureUnit::Fahrenheit,
      enums::settings::TemperatureUnit::Celsius,
      value,
    );
    assert_eq!(result, 100); // 212°F = 100°C
  }

  #[test]
  #[cfg(target_os = "windows")]
  fn test_celsius_to_fahrenheit_negative() {
    let value = -40; // -40°C
    let result = format_temperature(
      enums::settings::TemperatureUnit::Celsius,
      enums::settings::TemperatureUnit::Fahrenheit,
      value,
    );
    assert_eq!(result, -40); // -40°C = -40°F (特殊なケース)
  }

  #[test]
  #[cfg(target_os = "windows")]
  fn test_fahrenheit_to_celsius_negative() {
    let value = -40; // -40°F
    let result = format_temperature(
      enums::settings::TemperatureUnit::Fahrenheit,
      enums::settings::TemperatureUnit::Celsius,
      value,
    );
    assert_eq!(result, -40); // -40°F = -40°C (特殊なケース)
  }

  #[test]
  #[cfg(target_os = "windows")]
  fn test_no_conversion() {
    let value = 25; // 25°C
    let result = format_temperature(
      enums::settings::TemperatureUnit::Celsius,
      enums::settings::TemperatureUnit::Celsius,
      value,
    );
    assert_eq!(result, 25); // 単位が同じ場合、変換しない
  }
}
