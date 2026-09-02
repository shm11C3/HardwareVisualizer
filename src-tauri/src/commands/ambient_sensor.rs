//! Listing the ambient sensors the radio is currently hearing (#2062).
//!
//! Separate from `commands::settings` because this answers a question
//! about the room rather than about stored preferences: it reports what
//! is in range right now, which is live state the settings file knows
//! nothing about.

use crate::models::environmental_sensors::AmbientSensorCandidate;

/// Every SwitchBot device heard since launch, in a stable device-id order. Returns an empty list on a machine where the ambient source is off, where no adapter exists, or where nothing has advertised yet - all of which look the same from here and are equally honest as "nothing to choose from".
#[tauri::command]
#[specta::specta]
#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
pub async fn get_ambient_sensor_candidates(
  state: tauri::State<'_, crate::workers::WorkersState>,
) -> Result<Vec<AmbientSensorCandidate>, String> {
  #[cfg(target_os = "windows")]
  {
    let provider = {
      let guard = state
        .switchbot_provider
        .lock()
        .map_err(|_| "ambient provider lock was poisoned".to_string())?;
      guard.clone()
    };
    let Some(provider) = provider else {
      return Ok(Vec::new());
    };

    let selected = provider.bound_device();
    Ok(
      provider
        .discovered_sensors()
        .into_iter()
        .map(|sensor| {
          let short_id = sensor
            .device_id
            .get(sensor.device_id.len().saturating_sub(4)..)
            .unwrap_or(&sensor.device_id)
            .to_string();
          AmbientSensorCandidate {
            selected: selected.as_deref() == Some(sensor.device_id.as_str()),
            short_id,
            device_id: sensor.device_id,
            temperature_celsius: sensor.temperature_celsius,
            humidity_percent: sensor.humidity_percent,
          }
        })
        .collect(),
    )
  }

  #[cfg(not(target_os = "windows"))]
  {
    Ok(Vec::new())
  }
}
