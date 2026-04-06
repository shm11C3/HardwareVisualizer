use crate::commands::settings;
use crate::enums::settings::TemperatureUnit;
use crate::models::hardware::{GpuMonitorData, HardwareMonitorUpdate};
use crate::services::monitoring_service;
use crate::{log_error, models};
use crate::{log_internal, log_warn};
use tauri::Manager as _;
use tauri_specta::Event as _;

pub struct SystemMonitorController {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: tokio::sync::watch::Sender<bool>,
}

///
/// System information update frequency (seconds)
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1; // TODO move to constants.rs

/// Build `Vec<GpuMonitorData>` from raw GPU samples, applying temperature
/// unit conversion per GPU.
fn build_gpu_payloads(
  gpu_samples: &[monitoring_service::GpuSample],
  temp_unit: &TemperatureUnit,
) -> Vec<GpuMonitorData> {
  gpu_samples
    .iter()
    .map(|s| {
      let temperature = s.temperature.map(|t| match temp_unit {
        TemperatureUnit::Celsius => t.round(),
        TemperatureUnit::Fahrenheit => (t * 9.0 / 5.0 + 32.0).round(),
      });
      GpuMonitorData {
        gpu_id: s.gpu_id.clone(),
        gpu_name: s.name.clone(),
        gpu_usage: s.usage.map(|u| u.round()),
        gpu_temperature: temperature,
        gpu_source: s.source.clone(),
        gpu_dedicated_memory_usage_kb: s.dedicated_memory_kb,
        gpu_cooler_level: s.cooler_level,
      }
    })
    .collect()
}

fn emit_hardware_update(
  app_handle: &tauri::AppHandle,
  system_sample: Option<&monitoring_service::SystemSample>,
  gpu_samples: &[monitoring_service::GpuSample],
) {
  if let Some(sys) = system_sample {
    let temp_unit = app_handle
      .try_state::<settings::AppState>()
      .map(|state| state.settings.lock().unwrap().temperature_unit.clone())
      .unwrap_or(TemperatureUnit::Celsius);

    let gpus = build_gpu_payloads(gpu_samples, &temp_unit);

    let payload = HardwareMonitorUpdate {
      cpu_usage: sys.cpu_usage,
      memory_usage: sys.memory_usage,
      gpus,
      processors_usage: sys.processors_usage.clone(),
    };
    if let Err(e) = payload.emit(app_handle) {
      log_error!(
        &format!("failed to emit HardwareMonitorUpdate event: {}", e),
        "system_monitor",
        None::<&str>
      );
    }
  }
}

impl SystemMonitorController {
  ///
  /// ## Initialize system information
  ///
  /// - param system: `Arc<Mutex<System>>` System information
  ///
  /// - Updates CPU usage and memory usage every `SYSTEM_INFO_INIT_INTERVAL` seconds
  ///
  pub fn setup(
    resources: models::hardware_archive::MonitorResources,
    app_handle: tauri::AppHandle,
  ) -> Self {
    let (tx, mut rx) = tokio::sync::watch::channel(false);

    let handle: tauri::async_runtime::JoinHandle<()> =
      tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
          SYSTEM_INFO_INIT_INTERVAL,
        ));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let system_sample = monitoring_service::sample_system(&resources);
        let gpu_samples = monitoring_service::sample_gpu(&resources).await;

        emit_hardware_update(&app_handle, system_sample.as_ref(), &gpu_samples);

        loop {
          tokio::select! {
            _ = ticker.tick() =>  {
              let start = std::time::Instant::now();

              let system_sample = monitoring_service::sample_system(&resources);
              let gpu_samples = monitoring_service::sample_gpu(&resources).await;
              emit_hardware_update(&app_handle, system_sample.as_ref(), &gpu_samples);

              let elapsed = start.elapsed();
              if elapsed > tokio::time::Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL) {
                log_warn!(
                  &format!("overrun {:?} (> {}s)", elapsed, SYSTEM_INFO_INIT_INTERVAL),
                  "system_monitor",
                  None::<&str>
                );
              }
            }
            result = rx.changed() => {
              if result.is_err() || *rx.borrow() {
                eprintln!("[system-monitor] shutdown signal received");
                break;
              }
            }
          }
        }
      });

    Self {
      stop_tx: tx,
      handle,
    }
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_sample(
    gpu_id: &str,
    name: &str,
    usage: Option<f32>,
    temp: Option<f32>,
  ) -> monitoring_service::GpuSample {
    monitoring_service::GpuSample {
      gpu_id: gpu_id.to_string(),
      name: name.to_string(),
      usage,
      temperature: temp,
      dedicated_memory_kb: Some(4096.0),
      cooler_level: Some(60),
      source: "Test".to_string(),
    }
  }

  #[test]
  fn build_gpus_from_empty_samples() {
    let result = build_gpu_payloads(&[], &TemperatureUnit::Celsius);
    assert!(result.is_empty());
  }

  #[test]
  fn build_gpus_from_single_sample() {
    let samples = vec![make_sample("gpu:0", "RTX 4090", Some(75.3), Some(65.0))];
    let result = build_gpu_payloads(&samples, &TemperatureUnit::Celsius);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].gpu_id, "gpu:0");
    assert_eq!(result[0].gpu_name, "RTX 4090");
    assert_eq!(result[0].gpu_usage, Some(75.0)); // rounded
    assert_eq!(result[0].gpu_temperature, Some(65.0));
    assert_eq!(result[0].gpu_dedicated_memory_usage_kb, Some(4096.0));
    assert_eq!(result[0].gpu_cooler_level, Some(60));
  }

  #[test]
  fn build_gpus_from_multiple_samples() {
    let samples = vec![
      make_sample("pci:0:2.0", "RTX 4090", Some(80.0), Some(70.0)),
      make_sample("pci:0:3.0", "RX 7900 XTX", Some(50.0), Some(60.0)),
    ];
    let result = build_gpu_payloads(&samples, &TemperatureUnit::Celsius);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].gpu_id, "pci:0:2.0");
    assert_eq!(result[1].gpu_id, "pci:0:3.0");
  }

  #[test]
  fn temperature_conversion_celsius() {
    let samples = vec![make_sample("gpu:0", "GPU", None, Some(100.0))];
    let result = build_gpu_payloads(&samples, &TemperatureUnit::Celsius);
    assert_eq!(result[0].gpu_temperature, Some(100.0));
  }

  #[test]
  fn temperature_conversion_fahrenheit() {
    let samples = vec![make_sample("gpu:0", "GPU", None, Some(100.0))];
    let result = build_gpu_payloads(&samples, &TemperatureUnit::Fahrenheit);
    assert_eq!(result[0].gpu_temperature, Some(212.0)); // 100*9/5+32 = 212
  }

  #[test]
  fn temperature_none_preserved() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(50.0), None)];
    let result = build_gpu_payloads(&samples, &TemperatureUnit::Fahrenheit);
    assert!(result[0].gpu_temperature.is_none());
  }

  #[test]
  fn usage_rounded() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(33.7), None)];
    let result = build_gpu_payloads(&samples, &TemperatureUnit::Celsius);
    assert_eq!(result[0].gpu_usage, Some(34.0));
  }
}
