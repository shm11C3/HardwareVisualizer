//! Periodic sampling loop that publishes a [`MetricsSnapshot`] every
//! [`SYSTEM_INFO_INIT_INTERVAL`] seconds to the in-process EventBus.

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{Duration, MissedTickBehavior, interval};

use crate::collector::{HistoryStore, ProcessGpuCollector, sampling};
use crate::event_bus::EventBus;
use crate::platform::factory::PlatformFactory;
use crate::{log_error, log_info, log_warn};

/// Cadence of the collector loop in seconds.
const SYSTEM_INFO_INIT_INTERVAL: u64 = 1; // TODO move to a shared constants module

pub struct SystemMonitorController {
  handle: JoinHandle<()>,
  stop_tx: watch::Sender<bool>,
}

impl SystemMonitorController {
  /// Start the sampling loop on a background tokio task.
  ///
  /// - `store` is the shared history bag the collector writes into;
  ///   readers use the same `Arc<HistoryStore>` via the App-side
  ///   `tauri::State`.
  /// - `process_gpu` owns per-process GPU usage; while a background consumer
  ///   demands it, the loop refreshes it on its interval and each snapshot
  ///   carries the accumulated usage.
  /// - `bus` is the in-process broadcast channel; the App-side
  ///   `WindowAdapter` (and future tray / overlay adapters) subscribe
  ///   and translate snapshots into Tauri events.
  /// - `runtime` is an explicit tokio runtime handle the loop is
  ///   spawned onto. The Tauri caller passes `tauri::async_runtime::handle().inner().clone()`
  ///   so the collector runs on Tauri's existing runtime instead of
  ///   needing its own.
  ///
  /// The collector publishes raw °C in
  /// `MetricsSnapshot::gpus[i].gpu_temperature`. Temperature unit
  /// conversion is the App-side adapter's job.
  pub fn setup(
    store: Arc<HistoryStore>,
    process_gpu: Arc<ProcessGpuCollector>,
    bus: EventBus,
    runtime: Handle,
  ) -> Self {
    let (stop_tx, mut stop_rx) = watch::channel(false);

    let handle = runtime.spawn(async move {
      #[cfg(target_os = "windows")]
      let (sensor_ticks, sensor_task) = {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let task = tokio::spawn(crate::sensor_service::windows::collect(receiver));
        (sender, task)
      };

      let mut ticker = interval(Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL));
      ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

      // Resolve the shared platform once. While resolution fails, each
      // cycle reports unavailable sensor samples with the error as the
      // reason and the next tick retries, so a transient startup
      // failure cannot degrade the rest of the process lifetime.
      let mut platform = PlatformFactory::shared();
      if let Err(error) = &platform {
        log_error!(
          "platform unavailable for the collector loop",
          "collector::system_monitor",
          Some(error.to_string())
        );
      }

      #[cfg(target_os = "windows")]
      let _ = sensor_ticks.try_send(());

      // Prime the snapshot once so the first emit lands without waiting a tick.
      if let Some(snapshot) = sampling::collect_snapshot(&store, &platform, &process_gpu).await {
        bus.publish(snapshot);
      }

      loop {
        tokio::select! {
          _ = ticker.tick() => {
            let start = std::time::Instant::now();

            if platform.is_err() {
              platform = PlatformFactory::shared();
            }
            #[cfg(target_os = "windows")]
            let _ = sensor_ticks.try_send(());

            refresh_process_gpu_if_due(&process_gpu).await;

            if let Some(snapshot) = sampling::collect_snapshot(&store, &platform, &process_gpu).await {
              bus.publish(snapshot);
            }

            let elapsed = start.elapsed();
            if elapsed > Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL) {
              log_warn!(
                &format!("overrun {:?} (> {}s)", elapsed, SYSTEM_INFO_INIT_INTERVAL),
                "system_monitor",
                None::<&str>
              );
            }
          }
          result = stop_rx.changed() => {
            if result.is_err() || *stop_rx.borrow() {
              log_info!(
                "system monitor shutdown signal received",
                "collector::system_monitor",
                None::<&str>
              );
              break;
            }
          }
        }
      }
      #[cfg(target_os = "windows")]
      {
        drop(sensor_ticks);
        let _ = sensor_task.await;
      }
    });

    Self { handle, stop_tx }
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}

/// Runs the background per-process GPU refresh when one is due. The refresh
/// walks every process (about 160 ms for ~1000 processes on macOS), so it
/// runs on the blocking pool instead of the collector task's worker.
async fn refresh_process_gpu_if_due(process_gpu: &Arc<ProcessGpuCollector>) {
  if !process_gpu.background_refresh_due() {
    return;
  }
  let collector = Arc::clone(process_gpu);
  if let Err(error) =
    tokio::task::spawn_blocking(move || collector.refresh_background()).await
  {
    log_error!(
      "background process GPU refresh failed",
      "collector::system_monitor",
      Some(error.to_string())
    );
  }
}
