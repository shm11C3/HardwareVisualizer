//! One sampling cycle of system + GPU metrics.
//!
//! Functions in this module read sysinfo / OS-specific provider state and
//! mutate the shared [`crate::collector::history::HistoryStore`].
//! Temperature unit conversion is **not** done here — collectors always
//! publish raw °C in [`crate::models::MetricsSnapshot`]. The App-side
//! `WindowAdapter` applies the user's preferred unit before emitting the
//! Tauri `HardwareMonitorUpdate` event.

use crate::collector::HistoryStore;
use crate::models::{
  ExternalComponentGuidanceCandidate, GpuMetric, GpuSample, MetricsSnapshot,
  MotherboardSensorCollection, MotherboardSensorSample, ProcessSample, SensorTemperature,
  TemperatureSample,
};
use crate::platform::factory::PlatformFactory;

pub struct SystemSample {
  pub cpu_usage: f32,
  pub memory_usage: f32,
  pub processors_usage: Vec<f32>,
  pub processes: Vec<ProcessSample>,
}

pub fn sample_motherboard_sensors() -> MotherboardSensorCollection {
  match PlatformFactory::create() {
    Ok(platform) => platform.sample_motherboard_sensors(),
    Err(error) => MotherboardSensorCollection::unavailable(error.to_string()),
  }
}

pub fn sample_temperatures() -> TemperatureSample {
  match PlatformFactory::create() {
    Ok(platform) => platform.sample_temperatures(),
    Err(error) => TemperatureSample::unavailable(error.to_string()),
  }
}

/// Run one CPU / memory / process refresh and append samples to the
/// store's history rings.
pub fn sample_system(store: &HistoryStore) -> Option<SystemSample> {
  let result = store.system().lock().ok().map(|mut sys| {
    sys.refresh_all();

    let cpu_usage = calculate_average_cpu_usage(sys.cpus());
    let memory_usage =
      calculate_memory_usage_percentage(sys.used_memory(), sys.total_memory());
    let processors_usage: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();

    let processes: Vec<ProcessSample> = sys
      .processes()
      .iter()
      .map(|(pid, process)| ProcessSample {
        pid: pid.as_u32(),
        name: process.name().to_string_lossy().into_owned(),
        cpu_usage: process.cpu_usage(),
        memory_kb: process.memory() as f32 / 1024.0,
        run_time_secs: process.run_time(),
      })
      .collect();

    let process_history_input: Vec<_> = processes
      .iter()
      .map(|p| (sysinfo::Pid::from_u32(p.pid), p.cpu_usage, p.memory_kb))
      .collect();

    (
      cpu_usage,
      memory_usage,
      processors_usage,
      processes,
      process_history_input,
    )
  });

  let (cpu_usage, memory_usage, processors_usage, processes, process_history_input) =
    result?;

  store.push_cpu_history(cpu_usage);
  store.push_memory_history(memory_usage);
  store.update_process_histories(&process_history_input);

  Some(SystemSample {
    cpu_usage,
    memory_usage,
    processors_usage,
    processes,
  })
}

fn calculate_average_cpu_usage(cpus: &[sysinfo::Cpu]) -> f32 {
  match cpus.len() {
    0 => 0.0,
    len => (cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / len as f32).round(),
  }
}

fn calculate_memory_usage_percentage(used: u64, total: u64) -> f32 {
  match total {
    0 => 0.0,
    total => ((used as f64 / total as f64) * 100.0).round() as f32,
  }
}

pub async fn sample_gpu(store: &HistoryStore) -> Vec<GpuSample> {
  let gpu_metrics = match PlatformFactory::create() {
    Ok(platform) => platform.sample_gpus().await,
    Err(_) => Vec::new(),
  };

  if !gpu_metrics.is_empty() {
    store.update_gpu_histories(&gpu_metrics);
  }

  gpu_metrics
}

/// Build the per-GPU metrics carried in [`MetricsSnapshot`]. Temperatures
/// are forwarded as raw °C; the App-side adapter applies the user's
/// preferred unit.
fn build_gpu_metrics(gpu_samples: &[GpuSample]) -> Vec<GpuMetric> {
  gpu_samples
    .iter()
    .map(|s| GpuMetric {
      gpu_id: s.gpu_id.clone(),
      gpu_name: s.name.clone(),
      gpu_usage: s.usage.map(|u| u.round()),
      gpu_temperature: s.temperature.map(|t| t.round()),
      gpu_source: s.source.clone(),
      gpu_dedicated_memory_usage_kb: s.dedicated_memory_kb,
      gpu_cooler_level: s.cooler_level,
    })
    .collect()
}

/// Compose a [`MetricsSnapshot`] from one cycle's system + GPU +
/// temperature samples. Temperatures are forwarded as raw °C (rounded);
/// the App-side adapter applies the user's preferred unit.
pub fn build_metrics_snapshot(
  system_sample: &SystemSample,
  gpu_samples: &[GpuSample],
  temperature_sample: &TemperatureSample,
  motherboard_sample: &MotherboardSensorSample,
  motherboard_guidance_candidates: &[ExternalComponentGuidanceCandidate],
) -> MetricsSnapshot {
  MetricsSnapshot {
    cpu_usage: system_sample.cpu_usage,
    memory_usage: system_sample.memory_usage,
    processors_usage: system_sample.processors_usage.clone(),
    gpus: build_gpu_metrics(gpu_samples),
    processes: system_sample.processes.clone(),
    cpu_temperature: temperature_sample.cpu_temperature.map(|t| t.round()),
    cpu_temperature_verification: temperature_sample.cpu_temperature_verification,
    sensor_temperatures: temperature_sample
      .sensor_temperatures
      .iter()
      .map(|s| SensorTemperature {
        name: s.name.clone(),
        temperature: s.temperature.round(),
        verification: s.verification,
      })
      .collect(),
    motherboard_temperatures: motherboard_sample
      .temperatures
      .iter()
      .map(|s| crate::models::MotherboardTemperature {
        name: s.name.clone(),
        temperature: s.temperature.round(),
        source: s.source.clone(),
        verification: s.verification,
      })
      .collect(),
    motherboard_fan_speeds: motherboard_sample.fan_speeds.clone(),
    external_component_guidance_candidates: temperature_sample
      .guidance_candidates
      .iter()
      .chain(motherboard_guidance_candidates.iter())
      .cloned()
      .collect(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{SensorAvailability, SensorVerification};

  fn make_sample(
    gpu_id: &str,
    name: &str,
    usage: Option<f32>,
    temp: Option<f32>,
  ) -> GpuSample {
    GpuSample {
      gpu_id: gpu_id.to_string(),
      name: name.to_string(),
      usage,
      temperature: temp,
      dedicated_memory_kb: Some(4096.0),
      cooler_level: Some(60),
      source: "Test".to_string(),
    }
  }

  // ── calculate_memory_usage_percentage ──

  #[test]
  fn memory_usage_percentage_total_zero() {
    assert_eq!(calculate_memory_usage_percentage(1000, 0), 0.0);
  }

  #[test]
  fn memory_usage_percentage_half_used() {
    assert_eq!(calculate_memory_usage_percentage(500, 1000), 50.0);
  }

  #[test]
  fn memory_usage_percentage_fully_used() {
    assert_eq!(calculate_memory_usage_percentage(1000, 1000), 100.0);
  }

  #[test]
  fn memory_usage_percentage_zero_used() {
    assert_eq!(calculate_memory_usage_percentage(0, 1000), 0.0);
  }

  #[test]
  fn memory_usage_percentage_rounding() {
    assert_eq!(calculate_memory_usage_percentage(333, 1000), 33.0);
  }

  #[test]
  fn memory_usage_percentage_large_values() {
    assert_eq!(
      calculate_memory_usage_percentage(16_000_000_000, 32_000_000_000),
      50.0
    );
  }

  // ── build_gpu_metrics ──

  #[test]
  fn build_gpus_from_empty_samples() {
    let result = build_gpu_metrics(&[]);
    assert!(result.is_empty());
  }

  #[test]
  fn build_gpus_from_single_sample() {
    let samples = vec![make_sample("gpu:0", "RTX 4090", Some(75.3), Some(65.0))];
    let result = build_gpu_metrics(&samples);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].gpu_id, "gpu:0");
    assert_eq!(result[0].gpu_name, "RTX 4090");
    assert_eq!(result[0].gpu_usage, Some(75.0)); // rounded
    assert_eq!(result[0].gpu_temperature, Some(65.0));
    assert_eq!(result[0].gpu_dedicated_memory_usage_kb, Some(4096.0));
    assert_eq!(result[0].gpu_cooler_level, Some(60));
  }

  #[test]
  fn build_gpus_passes_through_celsius_unchanged() {
    let samples = vec![make_sample("gpu:0", "GPU", None, Some(100.0))];
    let result = build_gpu_metrics(&samples);
    // Core publishes raw °C; the App-side adapter does any conversion.
    assert_eq!(result[0].gpu_temperature, Some(100.0));
  }

  #[test]
  fn build_gpus_temperature_none_preserved() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(50.0), None)];
    let result = build_gpu_metrics(&samples);
    assert!(result[0].gpu_temperature.is_none());
  }

  #[test]
  fn build_gpus_usage_rounded() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(33.7), None)];
    let result = build_gpu_metrics(&samples);
    assert_eq!(result[0].gpu_usage, Some(34.0));
  }

  #[test]
  fn build_gpus_from_multiple_samples() {
    let samples = vec![
      make_sample("pci:0:2.0", "RTX 4090", Some(80.0), Some(70.0)),
      make_sample("pci:0:3.0", "RX 7900 XTX", Some(50.0), Some(60.0)),
    ];
    let result = build_gpu_metrics(&samples);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].gpu_id, "pci:0:2.0");
    assert_eq!(result[1].gpu_id, "pci:0:3.0");
  }

  // ── build_metrics_snapshot ──

  #[test]
  fn snapshot_carries_system_fields_and_gpus() {
    let sys = SystemSample {
      cpu_usage: 12.5,
      memory_usage: 67.0,
      processors_usage: vec![10.0, 20.0, 30.0, 40.0],
      processes: vec![ProcessSample {
        pid: 1,
        name: "init".into(),
        cpu_usage: 0.0,
        memory_kb: 1024.0,
        run_time_secs: 60,
      }],
    };
    let gpus = vec![make_sample("gpu:0", "RTX 4090", Some(50.0), Some(70.0))];
    let snap = build_metrics_snapshot(
      &sys,
      &gpus,
      &TemperatureSample::default(),
      &MotherboardSensorSample::default(),
      &[],
    );
    assert_eq!(snap.cpu_usage, 12.5);
    assert_eq!(snap.memory_usage, 67.0);
    assert_eq!(snap.processors_usage, vec![10.0, 20.0, 30.0, 40.0]);
    assert_eq!(snap.gpus.len(), 1);
    assert_eq!(snap.gpus[0].gpu_id, "gpu:0");
    assert_eq!(snap.processes.len(), 1);
    assert_eq!(snap.processes[0].pid, 1);
    assert_eq!(snap.processes[0].name, "init");
    assert_eq!(snap.cpu_temperature, None);
    assert!(snap.sensor_temperatures.is_empty());
  }

  #[test]
  fn snapshot_rounds_cpu_and_sensor_temperatures() {
    let sys = SystemSample {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      processes: vec![],
    };
    let temps = TemperatureSample {
      cpu_temperature: Some(49.95),
      cpu_temperature_verification: Some(SensorVerification::Experimental),
      sensor_temperatures: vec![
        SensorTemperature {
          name: "CPUZ".into(),
          temperature: 49.95,
          verification: SensorVerification::Experimental,
        },
        SensorTemperature {
          name: "TZ01".into(),
          temperature: 40.2,
          verification: SensorVerification::Verified,
        },
      ],
      availability: SensorAvailability::Available,
      guidance_candidates: Vec::new(),
    };
    let snap =
      build_metrics_snapshot(&sys, &[], &temps, &MotherboardSensorSample::default(), &[]);
    assert_eq!(snap.cpu_temperature, Some(50.0));
    assert_eq!(
      snap.sensor_temperatures,
      vec![
        SensorTemperature {
          name: "CPUZ".into(),
          temperature: 50.0,
          verification: SensorVerification::Experimental,
        },
        SensorTemperature {
          name: "TZ01".into(),
          temperature: 40.0,
          verification: SensorVerification::Verified,
        },
      ]
    );
  }

  #[test]
  fn snapshot_carries_motherboard_sensors() {
    let sys = SystemSample {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      processes: vec![],
    };
    let motherboard_sample = MotherboardSensorSample {
      temperatures: vec![crate::models::MotherboardTemperature {
        name: "SYSTIN".into(),
        temperature: 39.6,
        source: "NCT6799D / Super I/O".into(),
        verification: SensorVerification::Verified,
      }],
      fan_speeds: vec![crate::models::MotherboardFanSpeed {
        name: "Fan 1".into(),
        rpm: Some(0),
        status: crate::models::FanSpeedStatus::Inactive,
        source: "NCT6799D / Super I/O".into(),
        verification: SensorVerification::Verified,
      }],
    };

    let snap = build_metrics_snapshot(
      &sys,
      &[],
      &TemperatureSample::default(),
      &motherboard_sample,
      &[],
    );

    assert_eq!(snap.motherboard_temperatures.len(), 1);
    assert_eq!(snap.motherboard_temperatures[0].name, "SYSTIN");
    assert_eq!(snap.motherboard_temperatures[0].temperature, 40.0);
    assert_eq!(
      snap.motherboard_temperatures[0].source,
      "NCT6799D / Super I/O"
    );
    assert_eq!(snap.motherboard_fan_speeds, motherboard_sample.fan_speeds);
  }

  #[test]
  fn snapshot_carries_external_component_guidance_candidates() {
    let sys = SystemSample {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      processes: vec![],
    };
    let candidate = ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
      "PawnIOLib.dll not found".to_string(),
    );
    let temps = TemperatureSample {
      cpu_temperature: None,
      cpu_temperature_verification: None,
      sensor_temperatures: vec![],
      availability: SensorAvailability::unavailable(
        "PawnIO unavailable; ACPI thermal zones unavailable".to_string(),
      ),
      guidance_candidates: vec![candidate.clone()],
    };

    let snap =
      build_metrics_snapshot(&sys, &[], &temps, &MotherboardSensorSample::default(), &[]);

    assert_eq!(snap.external_component_guidance_candidates, vec![candidate]);
  }

  #[test]
  fn snapshot_carries_motherboard_external_component_guidance_candidates() {
    let sys = SystemSample {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      processes: vec![],
    };
    let candidate = ExternalComponentGuidanceCandidate::pawnio_motherboard_sensors(
      "pawnio_open failed: 0x80070005".to_string(),
    );

    let snap = build_metrics_snapshot(
      &sys,
      &[],
      &TemperatureSample::default(),
      &MotherboardSensorSample::default(),
      std::slice::from_ref(&candidate),
    );

    assert_eq!(snap.external_component_guidance_candidates, vec![candidate]);
  }
}
