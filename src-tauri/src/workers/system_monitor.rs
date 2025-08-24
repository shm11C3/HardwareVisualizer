use crate::models;
use crate::services::monitoring_service;
use crate::{log_internal, log_warn};

pub struct SystemMonitorController {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: tokio::sync::watch::Sender<bool>,
}

///
/// システム情報の更新頻度（秒）
///
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1; // TODO move to constants.rs

impl SystemMonitorController {
  ///
  /// ## システム情報の初期化
  ///
  /// - param system: `Arc<Mutex<System>>` システム情報
  ///
  /// - `SYSTEM_INFO_INIT_INTERVAL` 秒ごとにCPU使用率とメモリ使用率を更新
  ///
  pub fn setup(resources: models::hardware_archive::MonitorResources) -> Self {
    let (tx, mut rx) = tokio::sync::watch::channel(false);

    let handle: tauri::async_runtime::JoinHandle<()> =
      tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
          SYSTEM_INFO_INIT_INTERVAL,
        ));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        #[cfg(target_os = "windows")]
        {
          monitoring_service::sample_system(&resources);
          monitoring_service::sample_gpu(&resources);
        }
        #[cfg(target_os = "linux")]
        {
          monitoring_service::sample_system(&resources);
        }

        loop {
          tokio::select! {
            _ = ticker.tick() =>  {
              let start = std::time::Instant::now();

              #[cfg(target_os = "windows")]
              {
                monitoring_service::sample_system(&resources);
                monitoring_service::sample_gpu(&resources);
              }
              #[cfg(target_os = "linux")]
              {
                monitoring_service::sample_system(&resources);
              }

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
