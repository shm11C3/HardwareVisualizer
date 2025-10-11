use crate::infrastructure::providers::api::libre_hardware_monitor_provider::{
  LibreHardwareMonitorError, LibreHardwareMonitorProvider,
};
use crate::models;
use crate::{log_error, log_internal, log_warn};
use tauri::Manager;

/// Fetch interval in seconds
const LHM_FETCH_INTERVAL_SECONDS: u64 = 1;

pub struct LibreHardwareMonitorImportController {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: tokio::sync::watch::Sender<bool>,
}

impl LibreHardwareMonitorImportController {
  /// Setup background task that periodically fetches data.json
  /// from LibreHardwareMonitor and stores it into tauri state.
  pub fn setup(
    app: tauri::AppHandle,
    settings: models::settings::LibreHardwareMonitorImportSettings,
  ) -> Result<Self, LibreHardwareMonitorError> {
    let provider = LibreHardwareMonitorProvider::from_settings(&settings)?;

    let (tx, mut rx) = tokio::sync::watch::channel(false);
    let handle = tauri::async_runtime::spawn(async move {
      let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
        LHM_FETCH_INTERVAL_SECONDS,
      ));
      ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

      loop {
        tokio::select! {
          _ = ticker.tick() => {
            let start = std::time::Instant::now();

            match provider.fetch_data().await {
              Ok(node) => {
                // Store latest node to tauri state
                let state = app.state::<models::libre_hardware_monitor_state::LibreHardwareMonitorDataState>();
                if let Ok(mut latest) = state.latest.lock() {
                  *latest = Some(node);
                }
              }
              Err(e) => {
                log_warn!(&format!("LHM fetch failed: {}", e), "lhm_import", None::<&str>);
              }
            }

            let elapsed = start.elapsed();
            if elapsed > tokio::time::Duration::from_secs(LHM_FETCH_INTERVAL_SECONDS) {
              log_warn!(
                &format!("overrun {:?} (> {}s)", elapsed, LHM_FETCH_INTERVAL_SECONDS),
                "lhm_import",
                None::<&str>
              );
            }
          }
          result = rx.changed() => {
            if result.is_err() || *rx.borrow() {
              log_error!("[lhm-import] shutdown signal received", "lhm_import", None::<&str>);
              break;
            }
          }
        }
      }
    });

    Ok(Self {
      handle,
      stop_tx: tx,
    })
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}
