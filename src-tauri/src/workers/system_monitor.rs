use crate::commands::settings;
use crate::enums::settings::TemperatureUnit;
use crate::services::monitoring_service;
use crate::{log_internal, log_warn};
use hwviz_core::event_bus::EventBus;
use hwviz_core::models::{GpuMetric, MetricsSnapshot};
use tauri::Manager as _;

pub struct SystemMonitorController {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: tokio::sync::watch::Sender<bool>,
}

///
/// System information update frequency (seconds)
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1; // TODO move to constants.rs

/// Build `Vec<GpuMetric>` from raw GPU samples, applying temperature
/// unit conversion per GPU.
fn build_gpu_metrics(
  gpu_samples: &[monitoring_service::GpuSample],
  temp_unit: &TemperatureUnit,
) -> Vec<GpuMetric> {
  gpu_samples
    .iter()
    .map(|s| {
      let temperature = s.temperature.map(|t| match temp_unit {
        TemperatureUnit::Celsius => t.round(),
        TemperatureUnit::Fahrenheit => (t * 9.0 / 5.0 + 32.0).round(),
      });
      GpuMetric {
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

fn publish_metrics(
  bus: &EventBus,
  app_handle: &tauri::AppHandle,
  system_sample: Option<&monitoring_service::SystemSample>,
  gpu_samples: &[monitoring_service::GpuSample],
) {
  if let Some(sys) = system_sample {
    let temp_unit = app_handle
      .try_state::<settings::AppState>()
      .map(|state| state.settings.lock().unwrap().temperature_unit.clone())
      .unwrap_or(TemperatureUnit::Celsius);

    let snapshot = MetricsSnapshot {
      cpu_usage: sys.cpu_usage,
      memory_usage: sys.memory_usage,
      processors_usage: sys.processors_usage.clone(),
      gpus: build_gpu_metrics(gpu_samples, &temp_unit),
    };
    bus.publish(snapshot);
  }
}

impl SystemMonitorController {
  ///
  /// ## Initialize system information
  ///
  /// - param resources: shared sample/history bag.
  /// - param app_handle: still required this phase to read the user's
  ///   `temperature_unit` from `settings::AppState`. The collector no
  ///   longer emits Tauri events directly — snapshots are published to
  ///   `bus` and translated to `HardwareMonitorUpdate` by
  ///   `crate::adapters::window::WindowAdapter`. Phase 3 removes the
  ///   `app_handle` parameter when the collector relocates into Core.
  /// - param bus: in-process [`EventBus`] for `MetricsSnapshot` fan-out.
  ///
  /// Updates CPU usage and memory usage every
  /// `SYSTEM_INFO_INIT_INTERVAL` seconds.
  ///
  pub fn setup(
    resources: crate::models::hardware_archive::MonitorResources,
    app_handle: tauri::AppHandle,
    bus: EventBus,
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

        publish_metrics(&bus, &app_handle, system_sample.as_ref(), &gpu_samples);

        loop {
          tokio::select! {
            _ = ticker.tick() =>  {
              let start = std::time::Instant::now();

              let system_sample = monitoring_service::sample_system(&resources);
              let gpu_samples = monitoring_service::sample_gpu(&resources).await;
              publish_metrics(&bus, &app_handle, system_sample.as_ref(), &gpu_samples);

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
    let result = build_gpu_metrics(&[], &TemperatureUnit::Celsius);
    assert!(result.is_empty());
  }

  #[test]
  fn build_gpus_from_single_sample() {
    let samples = vec![make_sample("gpu:0", "RTX 4090", Some(75.3), Some(65.0))];
    let result = build_gpu_metrics(&samples, &TemperatureUnit::Celsius);
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
    let result = build_gpu_metrics(&samples, &TemperatureUnit::Celsius);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].gpu_id, "pci:0:2.0");
    assert_eq!(result[1].gpu_id, "pci:0:3.0");
  }

  #[test]
  fn temperature_conversion_celsius() {
    let samples = vec![make_sample("gpu:0", "GPU", None, Some(100.0))];
    let result = build_gpu_metrics(&samples, &TemperatureUnit::Celsius);
    assert_eq!(result[0].gpu_temperature, Some(100.0));
  }

  #[test]
  fn temperature_conversion_fahrenheit() {
    let samples = vec![make_sample("gpu:0", "GPU", None, Some(100.0))];
    let result = build_gpu_metrics(&samples, &TemperatureUnit::Fahrenheit);
    assert_eq!(result[0].gpu_temperature, Some(212.0)); // 100*9/5+32 = 212
  }

  #[test]
  fn temperature_none_preserved() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(50.0), None)];
    let result = build_gpu_metrics(&samples, &TemperatureUnit::Fahrenheit);
    assert!(result[0].gpu_temperature.is_none());
  }

  #[test]
  fn usage_rounded() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(33.7), None)];
    let result = build_gpu_metrics(&samples, &TemperatureUnit::Celsius);
    assert_eq!(result[0].gpu_usage, Some(34.0));
  }
}
