use crate::commands::settings;
use crate::enums::settings::TemperatureUnit;
use crate::models::hardware::HardwareMonitorUpdate;
use crate::services::monitoring_service;
use crate::{log_error, models};
use crate::{log_internal, log_warn};
use tauri::Manager as _;
use tauri_specta::Event as _;

pub struct SystemMonitorController {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: tokio::sync::watch::Sender<bool>,
}

struct GpuCapabilities {
  has_dedicated_memory: bool,
  has_cooler: bool,
}

///
/// System information update frequency (seconds)
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1; // TODO move to constants.rs

fn emit_hardware_update(
  app_handle: &tauri::AppHandle,
  system_sample: Option<&monitoring_service::SystemSample>,
  gpu_samples: &[monitoring_service::GpuSample],
  capabilities: &GpuCapabilities,
) {
  if let Some(sys) = system_sample {
    let first_gpu = gpu_samples.first();

    let (gpu_name, gpu_usage, gpu_temperature, gpu_source) = first_gpu
      .map(|s| {
        (
          Some(s.name.clone()),
          s.usage.map(|u| u.round()),
          s.temperature,
          Some(s.source.clone()),
        )
      })
      .unwrap_or((None, None, None, None));

    let gpu_temperature = gpu_temperature.map(|t| {
      let unit = app_handle
        .try_state::<settings::AppState>()
        .map(|state| state.settings.lock().unwrap().temperature_unit.clone())
        .unwrap_or(TemperatureUnit::Celsius);

      match unit {
        TemperatureUnit::Celsius => t.round(),
        TemperatureUnit::Fahrenheit => (t * 9.0 / 5.0 + 32.0).round(),
      }
    });

    let gpu_dedicated_memory_usage_kb = if capabilities.has_dedicated_memory {
      first_gpu.and_then(|s| s.dedicated_memory_kb)
    } else {
      None
    };

    let gpu_cooler_level = if capabilities.has_cooler {
      first_gpu.and_then(|s| s.cooler_level)
    } else {
      None
    };

    let payload = HardwareMonitorUpdate {
      cpu_usage: sys.cpu_usage,
      memory_usage: sys.memory_usage,
      gpu_usage,
      gpu_name,
      gpu_temperature,
      gpu_source,
      processors_usage: sys.processors_usage.clone(),
      gpu_dedicated_memory_usage_kb,
      gpu_cooler_level,
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

    let handle: tauri::async_runtime::JoinHandle<()> = tauri::async_runtime::spawn(
      async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
          SYSTEM_INFO_INIT_INTERVAL,
        ));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let system_sample = monitoring_service::sample_system(&resources);
        let gpu_samples = monitoring_service::sample_gpu(&resources).await;

        let gpu_capabilities = GpuCapabilities {
          has_dedicated_memory: gpu_samples
            .first()
            .is_some_and(|s| s.dedicated_memory_kb.is_some()),
          has_cooler: gpu_samples
            .first()
            .is_some_and(|s| s.cooler_level.is_some()),
        };

        emit_hardware_update(
          &app_handle,
          system_sample.as_ref(),
          &gpu_samples,
          &gpu_capabilities,
        );

        loop {
          tokio::select! {
            _ = ticker.tick() =>  {
              let start = std::time::Instant::now();

              let system_sample = monitoring_service::sample_system(&resources);
              let gpu_samples = monitoring_service::sample_gpu(&resources).await;
              emit_hardware_update(&app_handle, system_sample.as_ref(), &gpu_samples, &gpu_capabilities);

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
      },
    );

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
