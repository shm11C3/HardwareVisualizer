use crate::models::hardware::HardwareMonitorUpdate;
use crate::services::monitoring_service;
use crate::{log_error, models};
use crate::{log_internal, log_warn};
use tauri_specta::Event as _;

pub struct SystemMonitorController {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: tokio::sync::watch::Sender<bool>,
}

///
/// System information update frequency (seconds)
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1; // TODO move to constants.rs

fn emit_hardware_update(
  app_handle: &tauri::AppHandle,
  system_sample: Option<&monitoring_service::SystemSample>,
  gpu_samples: &[monitoring_service::GpuSample],
) {
  if let Some(sys) = system_sample {
    let (gpu_usage, gpu_source) = gpu_samples
      .first()
      .and_then(|(_, usage, _, _, source)| {
        usage.map(|u| (Some(u.round()), Some(source.clone())))
      })
      .unwrap_or((None, None));

    let payload = HardwareMonitorUpdate {
      cpu_usage: sys.cpu_usage,
      memory_usage: sys.memory_usage,
      gpu_usage,
      gpu_source,
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
