//! Periodic sampling loop that publishes a [`MetricsSnapshot`] every
//! [`SYSTEM_INFO_INIT_INTERVAL`] seconds to the in-process EventBus.

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{Duration, MissedTickBehavior, interval};

use crate::collector::{HistoryStore, sampling};
use crate::event_bus::EventBus;
use crate::{log_info, log_warn};

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
  pub fn setup(store: Arc<HistoryStore>, bus: EventBus, runtime: Handle) -> Self {
    let (stop_tx, mut stop_rx) = watch::channel(false);

    let handle = runtime.spawn(async move {
      let mut ticker = interval(Duration::from_secs(SYSTEM_INFO_INIT_INTERVAL));
      ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

      // Prime the snapshot once so the first emit lands without waiting a tick.
      if let Some(system_sample) = sampling::sample_system(&store) {
        let gpu_samples = sampling::sample_gpu(&store).await;
        let temperature_sample = sampling::sample_temperatures();
        let snapshot = sampling::build_metrics_snapshot(
          &system_sample,
          &gpu_samples,
          &temperature_sample,
        );
        bus.publish(snapshot);
      }

      loop {
        tokio::select! {
          _ = ticker.tick() => {
            let start = std::time::Instant::now();

            if let Some(system_sample) = sampling::sample_system(&store) {
              let gpu_samples = sampling::sample_gpu(&store).await;
              let temperature_sample = sampling::sample_temperatures();
              let snapshot = sampling::build_metrics_snapshot(
                &system_sample,
                &gpu_samples,
                &temperature_sample,
              );
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
    });

    Self { handle, stop_tx }
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}
