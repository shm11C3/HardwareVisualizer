use crate::enums::settings::TemperatureUnit;
use crate::models::hardware::{DimmTemperature, MemoryDetails};
use hardviz_core::enums::error::PlatformError;
use hardviz_core::models::hardware as core_hardware;
use hardviz_core::platform::factory::PlatformFactory;

pub async fn fetch_memory_detail(
  temperature_unit: TemperatureUnit,
) -> Result<MemoryDetails, PlatformError> {
  let platform = PlatformFactory::shared()?;
  let details = platform.get_memory_details().await?;
  Ok(MemoryDetails {
    memory: details.memory.into(),
    dimm_temperature: format_dimm_temperature(details.dimm_temperature, temperature_unit),
  })
}

fn format_dimm_temperature(
  reading: core_hardware::DimmTemperature,
  temperature_unit: TemperatureUnit,
) -> DimmTemperature {
  match reading {
    core_hardware::DimmTemperature::Unsupported => DimmTemperature::Unsupported,
    core_hardware::DimmTemperature::Unavailable => DimmTemperature::Unavailable,
    core_hardware::DimmTemperature::Available {
      locator,
      temperature_celsius,
    } => {
      let temperature = match temperature_unit {
        TemperatureUnit::Celsius => temperature_celsius,
        TemperatureUnit::Fahrenheit => temperature_celsius * 9.0 / 5.0 + 32.0,
      };
      DimmTemperature::Available {
        locator,
        temperature,
        temperature_unit,
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn available_dimm_reading_keeps_the_locator_and_requested_unit() {
    let reading = core_hardware::DimmTemperature::Available {
      locator: "DIMM 0".to_owned(),
      temperature_celsius: 40.25,
    };
    let converted = format_dimm_temperature(reading, TemperatureUnit::Fahrenheit);
    let DimmTemperature::Available {
      locator,
      temperature,
      temperature_unit,
    } = converted
    else {
      panic!("expected a converted temperature");
    };
    assert_eq!(locator, "DIMM 0");
    assert!((temperature - 104.45).abs() < 0.001);
    assert_eq!(temperature_unit, TemperatureUnit::Fahrenheit);
  }

  #[test]
  fn unavailable_dimm_reading_has_no_value_or_identity() {
    let converted = format_dimm_temperature(
      core_hardware::DimmTemperature::Unavailable,
      TemperatureUnit::Celsius,
    );
    assert_eq!(
      serde_json::to_value(converted).unwrap(),
      serde_json::json!({ "status": "unavailable" }),
    );
  }
}
