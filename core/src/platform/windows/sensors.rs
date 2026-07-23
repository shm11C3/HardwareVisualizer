use std::sync::atomic::{AtomicBool, Ordering};

use crate::infrastructure::providers::windows::cpu_temperature::{
  CpuPackageTemperature, CpuPackageTemperatureError, CpuTemperatureSource,
};
use crate::log_warn;
use crate::models::{
  ExternalComponentGuidanceCandidate, MotherboardSensorCollection, SensorAvailability,
  SensorTemperature, TemperatureSample,
};

static MOTHERBOARD_SENSOR_FALLBACK_LOGGED: AtomicBool = AtomicBool::new(false);

pub fn sample_motherboard_sensors() -> MotherboardSensorCollection {
  match crate::infrastructure::providers::windows::super_io_motherboard::sample_motherboard_sensors()
  {
    Ok(sample) => MotherboardSensorCollection {
      sample,
      availability: SensorAvailability::Available,
      guidance_candidates: Vec::new(),
    },
    Err(reason) => {
      if !MOTHERBOARD_SENSOR_FALLBACK_LOGGED.swap(true, Ordering::Relaxed) {
        log_warn!(
          "motherboard_sensor_sampling_failed",
          "platform::windows::sensors::sample_motherboard_sensors",
          Some(reason.clone())
        );
      }

      let availability =
        if reason
          == crate::infrastructure::providers::windows::super_io_motherboard::UNSUPPORTED_NUVOTON_HM_PATH_REASON
        {
          SensorAvailability::unsupported(reason.clone())
        } else {
          SensorAvailability::unavailable(reason.clone())
        };

      MotherboardSensorCollection {
        sample: Default::default(),
        availability,
        guidance_candidates: motherboard_guidance_candidates_for_reason(reason),
      }
    }
  }
}

fn motherboard_guidance_candidates_for_reason(
  reason: String,
) -> Vec<ExternalComponentGuidanceCandidate> {
  if reason
    == crate::infrastructure::providers::windows::super_io_motherboard::UNSUPPORTED_NUVOTON_HM_PATH_REASON
  {
    return Vec::new();
  }

  vec![ExternalComponentGuidanceCandidate::pawnio_motherboard_sensors(reason)]
}

/// Read the latest CPU / sensor temperatures.
///
/// Windows: prefer CPU package temperature from a recognized PawnIO source,
/// then fall back to ACPI thermal zones via the WMI sampler thread. When only
/// ACPI is available, the headline CPU value picks a CPU-named zone when one
/// exists, otherwise the hottest zone.
pub fn sample_temperatures() -> TemperatureSample {
  use crate::infrastructure::providers::windows::{cpu_temperature, thermal_zone};

  thermal_zone::init_thermal_zone_sampler();
  let sensor_temperatures = thermal_zone::read_thermal_zones_cached();
  build_temperature_sample(
    cpu_temperature::sample_cpu_package_temperature(),
    sensor_temperatures,
  )
}

fn build_temperature_sample(
  pawnio_cpu_temperature: Result<CpuPackageTemperature, CpuPackageTemperatureError>,
  mut sensor_temperatures: Vec<SensorTemperature>,
) -> TemperatureSample {
  match pawnio_cpu_temperature {
    Ok(sample) => {
      sensor_temperatures.insert(
        0,
        SensorTemperature {
          name: cpu_package_sensor_name(&sample).to_string(),
          temperature: sample.temperature_celsius,
        },
      );
      TemperatureSample {
        cpu_temperature: Some(sample.temperature_celsius),
        sensor_temperatures,
        availability: SensorAvailability::Available,
        guidance_candidates: Vec::new(),
      }
    }
    Err(pawnio_error) => {
      let pawnio_reason = pawnio_error.to_string();
      let component_failed = matches!(
        &pawnio_error,
        CpuPackageTemperatureError::Unavailable { .. }
      );
      let unsupported =
        matches!(&pawnio_error, CpuPackageTemperatureError::Unsupported(_));
      let cpu_temperature =
        crate::utils::thermal::select_cpu_temperature(&sensor_temperatures);
      let availability = if cpu_temperature.is_none() {
        let reason = match &pawnio_error {
          CpuPackageTemperatureError::Unsupported(_) => format!(
            "PawnIO CPU package path unsupported ({pawnio_reason}); ACPI thermal zones unavailable"
          ),
          CpuPackageTemperatureError::Unavailable { .. } => {
            format!(
              "PawnIO unavailable ({pawnio_reason}); ACPI thermal zones unavailable"
            )
          }
          CpuPackageTemperatureError::Internal(_) => format!(
            "CPU temperature sampler internal error ({pawnio_reason}); ACPI thermal zones unavailable"
          ),
        };
        if unsupported {
          SensorAvailability::unsupported(reason)
        } else {
          SensorAvailability::unavailable(reason)
        }
      } else {
        SensorAvailability::Available
      };
      let guidance_candidates = if component_failed && cpu_temperature.is_none() {
        vec![
          ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
            pawnio_reason,
          ),
        ]
      } else {
        Vec::new()
      };
      TemperatureSample {
        cpu_temperature,
        sensor_temperatures,
        availability,
        guidance_candidates,
      }
    }
  }
}

fn cpu_package_sensor_name(sample: &CpuPackageTemperature) -> &'static str {
  match sample.source {
    CpuTemperatureSource::IntelDtsPackageMsr => "CPU Package (PawnIO Intel DTS)",
    CpuTemperatureSource::AmdZenSmnTctl => "CPU Package (PawnIO AMD SMN)",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn motherboard_guidance_suppresses_unsupported_super_io_path() {
    let candidates = motherboard_guidance_candidates_for_reason(
      crate::infrastructure::providers::windows::super_io_motherboard::UNSUPPORTED_NUVOTON_HM_PATH_REASON
        .to_string(),
    );

    assert!(candidates.is_empty());
  }

  #[test]
  fn motherboard_guidance_remains_for_pawnio_access_failure() {
    let candidates = motherboard_guidance_candidates_for_reason(
      "pawnio_open failed: 0x80070005".to_string(),
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(
      candidates[0].key,
      crate::models::external_component_guidance::PAWNIO_MOTHERBOARD_SENSORS_KEY
    );
    assert_eq!(
      candidates[0].reason_kind,
      crate::models::ExternalComponentReasonKind::Permission
    );
  }

  #[test]
  fn temperature_sample_prefers_pawnio_cpu_package() {
    let sample = build_temperature_sample(
      Ok(
        crate::infrastructure::providers::windows::cpu_temperature::CpuPackageTemperature {
          temperature_celsius: 61.25,
          source: crate::infrastructure::providers::windows::cpu_temperature::CpuTemperatureSource::IntelDtsPackageMsr,
        },
      ),
      vec![SensorTemperature {
        name: "TZ00".into(),
        temperature: 45.0,
      }],
    );

    assert_eq!(sample.cpu_temperature, Some(61.25));
    assert_eq!(
      sample.sensor_temperatures[0].name,
      "CPU Package (PawnIO Intel DTS)"
    );
    assert_eq!(sample.sensor_temperatures[0].temperature, 61.25);
    assert_eq!(sample.availability, SensorAvailability::Available);
  }

  #[test]
  fn temperature_sample_keeps_the_standard_amd_source_label() {
    let sample = build_temperature_sample(
      Ok(CpuPackageTemperature {
        temperature_celsius: 63.5,
        source: CpuTemperatureSource::AmdZenSmnTctl,
      }),
      Vec::new(),
    );

    assert_eq!(sample.cpu_temperature, Some(63.5));
    assert_eq!(sample.availability, SensorAvailability::Available);
    assert_eq!(
      sample.sensor_temperatures[0].name,
      "CPU Package (PawnIO AMD SMN)"
    );
    assert!(sample.guidance_candidates.is_empty());
  }

  #[test]
  fn temperature_sample_falls_back_to_acpi_cpu_zone() {
    let sample = build_temperature_sample(
      Err(CpuPackageTemperatureError::Unavailable {
        reason: "PawnIOLib.dll not found".to_string(),
        enablement: crate::models::SensorEnablement::Verified,
      }),
      vec![
        SensorTemperature {
          name: "TZ00".into(),
          temperature: 70.0,
        },
        SensorTemperature {
          name: "CPUZ".into(),
          temperature: 51.0,
        },
      ],
    );

    assert_eq!(sample.cpu_temperature, Some(51.0));
    assert_eq!(sample.availability, SensorAvailability::Available);
  }

  #[test]
  fn temperature_sample_returns_unavailable_reason_when_all_sources_fail() {
    let sample = build_temperature_sample(
      Err(CpuPackageTemperatureError::Unavailable {
        reason: "pawnio_open failed".to_string(),
        enablement: crate::models::SensorEnablement::Verified,
      }),
      Vec::new(),
    );

    assert_eq!(sample.cpu_temperature, None);
    assert_eq!(
      sample.availability,
      SensorAvailability::unavailable(
        "PawnIO unavailable (pawnio_open failed); ACPI thermal zones unavailable"
      )
    );
  }

  #[test]
  fn temperature_sample_returns_guidance_when_pawnio_and_acpi_fail() {
    let sample = build_temperature_sample(
      Err(CpuPackageTemperatureError::Unavailable {
        reason: "PawnIOLib.dll not found".to_string(),
        enablement: crate::models::SensorEnablement::Verified,
      }),
      Vec::new(),
    );

    assert_eq!(sample.cpu_temperature, None);
    assert_eq!(sample.guidance_candidates.len(), 1);
    assert_eq!(
      sample.guidance_candidates[0].key,
      "pawnio:cpu-package-temperature:v1"
    );
    assert_eq!(
      sample.guidance_candidates[0].missing_signals,
      vec!["cpu-temperature".to_string()]
    );
  }

  #[test]
  fn temperature_sample_identifies_an_experimental_failure_when_all_sources_fail() {
    let sample = build_temperature_sample(
      Err(CpuPackageTemperatureError::Unavailable {
        reason: "CPU temperature decode failed".to_string(),
        enablement: crate::models::SensorEnablement::Experimental,
      }),
      Vec::new(),
    );

    assert_eq!(
      sample.availability,
      SensorAvailability::unavailable(
        "PawnIO unavailable (experimental CPU package temperature attempt failed: CPU temperature decode failed); ACPI thermal zones unavailable"
      )
    );
    assert_eq!(
      sample.guidance_candidates[0].diagnostic_detail.as_deref(),
      Some(
        "experimental CPU package temperature attempt failed: CPU temperature decode failed"
      )
    );
  }

  #[test]
  fn temperature_sample_suppresses_guidance_for_unsupported_cpu_path() {
    let sample = build_temperature_sample(
      Err(CpuPackageTemperatureError::Unsupported(
        crate::infrastructure::providers::windows::cpu_temperature::CpuTemperatureFallbackReason::AmdFamilyUnsupported(0x16),
      )),
      Vec::new(),
    );

    assert_eq!(sample.cpu_temperature, None);
    assert!(matches!(
      sample.availability,
      SensorAvailability::Unsupported { .. }
    ));
    assert!(sample.guidance_candidates.is_empty());
  }

  #[test]
  fn temperature_sample_suppresses_guidance_for_internal_failure() {
    let sample = build_temperature_sample(
      Err(CpuPackageTemperatureError::Internal(
        "CPU temperature sampler lock poisoned".to_string(),
      )),
      Vec::new(),
    );

    assert_eq!(sample.cpu_temperature, None);
    assert_eq!(
      sample.availability,
      SensorAvailability::unavailable(
        "CPU temperature sampler internal error (CPU temperature sampler lock poisoned); ACPI thermal zones unavailable"
      )
    );
    assert!(sample.guidance_candidates.is_empty());
  }

  #[test]
  fn temperature_sample_does_not_return_guidance_when_acpi_fallback_succeeds() {
    let sample = build_temperature_sample(
      Err(CpuPackageTemperatureError::Unavailable {
        reason: "PawnIOLib.dll not found".to_string(),
        enablement: crate::models::SensorEnablement::Verified,
      }),
      vec![SensorTemperature {
        name: "CPUZ".into(),
        temperature: 51.0,
      }],
    );

    assert_eq!(sample.cpu_temperature, Some(51.0));
    assert!(sample.guidance_candidates.is_empty());
  }
}
