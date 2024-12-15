use nvapi::Kibibytes;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;

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

pub struct RoundedKibibytes {
  pub kibibytes: Kibibytes,
  pub precision: u32,
}

///
/// ## `Kibibytes` をフォーマット
///
impl fmt::Display for RoundedKibibytes {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let value = self.kibibytes.0; // Kibibytesの内部値を取得
    if value < 1000 {
      write!(f, "{} KB", value)
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
  } else {
    format!("{} bytes", bytes)
  }
}

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
      unit: unit,
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
