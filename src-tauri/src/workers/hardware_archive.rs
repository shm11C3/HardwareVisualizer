use crate::constants::HARDWARE_ARCHIVE_INTERVAL_SECONDS;
use crate::models;
use crate::services::archive_service::ArchiveService;
use crate::{log_internal, log_warn};

pub struct HardwareArchiveController {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: tokio::sync::watch::Sender<bool>,
}

impl HardwareArchiveController {
  /// Starts the hardware archive background service.
  ///
  /// This orchestrates the periodic collection and archiving of hardware data
  /// by coordinating between data collection (service layer) and persistence (database layer).
  pub fn setup(resources: models::hardware_archive::MonitorResources) -> Self {
    let (tx, mut rx) = tokio::sync::watch::channel(false);

    let handle: tauri::async_runtime::JoinHandle<()> = tauri::async_runtime::spawn(
      async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
          HARDWARE_ARCHIVE_INTERVAL_SECONDS,
        ));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
          tokio::select! {
            _ = ticker.tick() =>  {
              let start = std::time::Instant::now();

              ArchiveService::archive_current_snapshot(&resources).await;

              let elapsed = start.elapsed();
              if elapsed > tokio::time::Duration::from_secs(HARDWARE_ARCHIVE_INTERVAL_SECONDS) {
                log_warn!(
                  &format!("overrun {elapsed:?} (> {HARDWARE_ARCHIVE_INTERVAL_SECONDS}s)"),
                  "hardware_archive",
                  None::<&str>
                );
              }
            }
            result = rx.changed() => {
              if result.is_err() || *rx.borrow() {
                eprintln!("[hardware-archive] shutdown signal received");
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

/// Deletes old archived data beyond the specified retention period.
///
/// This function delegates to the service layer for better separation of concerns.
pub async fn batch_delete_old_data(retention_days: u32) {
  ArchiveService::cleanup_old_data(retention_days).await;
}
