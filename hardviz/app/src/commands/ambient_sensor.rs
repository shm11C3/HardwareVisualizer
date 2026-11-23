//! Listing the ambient sensors the radio is currently hearing (#2062).
//!
//! Separate from `commands::settings` because this answers a question
//! about the room rather than about stored preferences: it reports what
//! is in range right now, which is live state the settings file knows
//! nothing about.

#[cfg(any(target_os = "windows", test))]
use crate::enums::settings::TemperatureUnit;
use crate::models::environmental_sensors::AmbientSensorCandidate;

/// Every SwitchBot device heard within the last few minutes, in a stable device-id order, each with its reading in the preferred temperature unit; a device that has gone quiet drops out rather than showing an old reading as current. Returns an empty list on a machine where the ambient source is off, where no adapter exists, or where nothing has advertised yet - all of which look the same from here and are equally honest as "nothing to choose from".
#[tauri::command]
#[specta::specta]
#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
pub async fn get_ambient_sensor_candidates(
  settings: tauri::State<'_, crate::commands::settings::AppState>,
  workers: tauri::State<'_, crate::workers::WorkersState>,
) -> Result<Vec<AmbientSensorCandidate>, String> {
  #[cfg(target_os = "windows")]
  {
    let provider = {
      let guard = workers
        .switchbot_provider
        .lock()
        .map_err(|_| "ambient provider lock was poisoned".to_string())?;
      guard.clone()
    };
    let Some(provider) = provider else {
      return Ok(Vec::new());
    };

    let temperature_unit = settings
      .settings
      .lock()
      .map_err(|_| "settings lock was poisoned".to_string())?
      .temperature_unit
      .clone();

    let selected = provider.bound_device();
    Ok(
      provider
        .discovered_sensors(chrono::Utc::now())
        .into_iter()
        .map(|sensor| {
          let selected = selected.as_deref() == Some(sensor.device_id.as_str());
          to_candidate(sensor, selected, &temperature_unit)
        })
        .collect(),
    )
  }

  #[cfg(not(target_os = "windows"))]
  {
    Ok(Vec::new())
  }
}

/// One heard device as the settings screen shows it.
///
/// Core keeps the reading in Celsius; which unit the user reads in is a
/// presentation preference, so it is applied here at the App boundary
/// and named on the wire rather than assumed by the screen. Nothing is
/// rounded: a tenth of a degree is what tells two devices in one room
/// apart.
#[cfg(any(target_os = "windows", test))]
fn to_candidate(
  sensor: hardviz_core::infrastructure::providers::switchbot_meter::DiscoveredSensor,
  selected: bool,
  temperature_unit: &TemperatureUnit,
) -> AmbientSensorCandidate {
  let temperature = match temperature_unit {
    TemperatureUnit::Celsius => sensor.temperature_celsius,
    TemperatureUnit::Fahrenheit => sensor.temperature_celsius * 9.0 / 5.0 + 32.0,
  };
  let short_id = sensor
    .device_id
    .get(sensor.device_id.len().saturating_sub(4)..)
    .unwrap_or(&sensor.device_id)
    .to_string();

  AmbientSensorCandidate {
    selected,
    short_id,
    device_id: sensor.device_id,
    temperature,
    temperature_unit: temperature_unit.clone(),
    humidity_percent: sensor.humidity_percent,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Utc;
  use hardviz_core::infrastructure::providers::switchbot_meter::DiscoveredSensor;

  fn heard(temperature_celsius: f32) -> DiscoveredSensor {
    DiscoveredSensor {
      device_id: "d051fa0f2cd0".to_string(),
      temperature_celsius,
      humidity_percent: Some(54.0),
      last_seen: Utc::now(),
    }
  }

  /// Core keeps the reading in Celsius; the unit is a presentation
  /// preference, so it is applied here and named on the wire rather
  /// than assumed by the screen.
  #[test]
  fn a_candidate_carries_the_reading_in_the_preferred_unit() {
    let fahrenheit = to_candidate(heard(25.0), false, &TemperatureUnit::Fahrenheit);
    assert_eq!(fahrenheit.temperature, 77.0);
    assert_eq!(fahrenheit.temperature_unit, TemperatureUnit::Fahrenheit);

    let celsius = to_candidate(heard(25.0), false, &TemperatureUnit::Celsius);
    assert_eq!(celsius.temperature, 25.0);
    assert_eq!(celsius.temperature_unit, TemperatureUnit::Celsius);
  }

  /// A tenth of a degree is what tells two devices in one room apart, so
  /// the conversion must not round it away.
  #[test]
  fn a_converted_reading_keeps_its_tenths() {
    let candidate = to_candidate(heard(25.2), false, &TemperatureUnit::Fahrenheit);
    assert!((candidate.temperature - 77.36).abs() < 0.01);
  }

  #[test]
  fn a_candidate_is_named_by_the_tail_of_its_address() {
    let candidate = to_candidate(heard(25.0), true, &TemperatureUnit::Celsius);
    assert_eq!(candidate.device_id, "d051fa0f2cd0");
    assert_eq!(candidate.short_id, "2cd0");
    assert!(candidate.selected);
    assert_eq!(candidate.humidity_percent, Some(54.0));
  }
}
