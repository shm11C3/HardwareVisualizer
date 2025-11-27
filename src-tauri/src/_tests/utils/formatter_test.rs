#[cfg(test)]
mod tests {
  use crate::utils::formatter::*;

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
    assert_eq!(result, -40); // -40°C = -40°F (special case)
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
    assert_eq!(result, -40); // -40°F = -40°C (special case)
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
    assert_eq!(result, 25); // No conversion when units are the same
  }
}
