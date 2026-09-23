use std::{
  future::Future,
  sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
  },
};

use hardviz_core::monitoring::MonitoringState;
use tokio::sync::watch;

use crate::adapters::tray::TrayAdapter;
use crate::adapters::window::WindowAdapter;

pub struct WorkersState {
  pub monitor: Mutex<Option<hardviz_core::collector::SystemMonitorController>>,
  pub window_adapter: Mutex<Option<WindowAdapter>>,
  pub hw_archive: Mutex<Option<hardviz_core::persistence::ArchiveController>>,
  pub cooling_rollup: Mutex<Option<hardviz_core::persistence::CoolingRollupController>>,
  pub storage_health: Mutex<Option<hardviz_core::persistence::StorageHealthController>>,
  /// The one-shot startup retention cleanup (`hardviz_core::persistence::cleanup_old_data`),
  /// if this boot scheduled one. It is a single pass, not a recurring worker,
  /// but it is still a database producer while it runs: `terminate_all` and
  /// the #2135 conversion driver's pause/drain both need to wait for it
  /// rather than let a boot's cleanup keep deleting rows after either has
  /// otherwise quiesced every writer.
  pub scheduled_cleanup: Mutex<Option<tokio::task::JoinHandle<()>>>,
  /// SwitchBot Meter advertisement scan (#2044). `None` unless the user
  /// turned the ambient source on, which is the default. Held here so
  /// the radio is released on quit rather than at process teardown.
  #[cfg(target_os = "windows")]
  pub switchbot_scan: Mutex<
    Option<
      hardviz_core::infrastructure::providers::switchbot_meter::SwitchBotScanController,
    >,
  >,

  /// The provider the scan feeds, so the settings screen can ask what
  /// devices are in range. `None` for the same reason `switchbot_scan`
  /// is: the ambient source is off until the user turns it on.
  #[cfg(target_os = "windows")]
  pub switchbot_provider: Mutex<
    Option<
      std::sync::Arc<
        hardviz_core::infrastructure::providers::switchbot_meter::SwitchBotMeterProvider,
      >,
    >,
  >,
  /// On-demand Live Storage Health collector (ADR 0006). Not a worker —
  /// no background task, so `terminate_all` leaves it alone. `None` when
  /// Storage Health is disabled at startup or the identity key is
  /// invalid; the command then returns an empty result.
  pub live_storage_health:
    Mutex<Option<std::sync::Arc<hardviz_core::collector::LiveStorageHealthCollector>>>,

  /// Holds the tray icon for as long as the process should display it.
  /// Dropping this releases the OS handle and removes the icon, so it
  /// stays here until shutdown rather than living in the setup
  /// closure's scope.
  pub tray: Mutex<Option<TrayAdapter>>,
  pub shutting_down: AtomicBool,
  shutdown_result: watch::Sender<Option<Result<(), String>>>,
  /// Lifecycle state of sensor collection. Owned here because the
  /// lifecycle module already locks workers on quit; colocating the
  /// state avoids a second `Mutex` around the same critical section.
  /// Phase 5 only writes `Stopped` from `lifecycle::request_quit`;
  /// the `Paused` / `Running` transitions become user-visible with
  /// #1275 and #1401.
  pub monitoring_state: Mutex<MonitoringState>,
}

struct ShutdownWorkers {
  monitor: Option<hardviz_core::collector::SystemMonitorController>,
  window_adapter: Option<WindowAdapter>,
  hw_archive: Option<hardviz_core::persistence::ArchiveController>,
  cooling_rollup: Option<hardviz_core::persistence::CoolingRollupController>,
  storage_health: Option<hardviz_core::persistence::StorageHealthController>,
  scheduled_cleanup: Option<tokio::task::JoinHandle<()>>,
  #[cfg(target_os = "windows")]
  switchbot_scan: Option<
    hardviz_core::infrastructure::providers::switchbot_meter::SwitchBotScanController,
  >,
  tray: Option<TrayAdapter>,
}

impl Default for WorkersState {
  fn default() -> Self {
    let (shutdown_result, _) = watch::channel(None);
    Self {
      monitor: Mutex::new(None),
      window_adapter: Mutex::new(None),
      hw_archive: Mutex::new(None),
      cooling_rollup: Mutex::new(None),
      storage_health: Mutex::new(None),
      scheduled_cleanup: Mutex::new(None),
      #[cfg(target_os = "windows")]
      switchbot_scan: Mutex::new(None),
      #[cfg(target_os = "windows")]
      switchbot_provider: Mutex::new(None),
      live_storage_health: Mutex::new(None),
      tray: Mutex::new(None),
      shutting_down: AtomicBool::new(false),
      shutdown_result,
      monitoring_state: Mutex::new(MonitoringState::default()),
    }
  }
}

impl WorkersState {
  pub async fn terminate_all(&self) {
    if let Err(error) = self.terminate_all_checked().await {
      hardviz_core::log_error!(
        "Failed to close the native database owner during shutdown",
        "workers::WorkersState::terminate_all",
        Some(error)
      );
    }
  }

  /// Drain every worker and report whether the dispatch owner closed cleanly.
  ///
  /// The shutdown path for a recovered native database uses the result before
  /// restarting, so it can log a close failure while still letting process
  /// exit release any remaining native handles.
  pub async fn terminate_all_checked(&self) -> Result<(), String> {
    run_shutdown_once_with_completion(&self.shutting_down, &self.shutdown_result, || {
      let workers = self.take_for_shutdown();
      async move { terminate_workers(workers).await }
    })
    .await
  }

  fn take_for_shutdown(&self) -> ShutdownWorkers {
    ShutdownWorkers {
      monitor: self.monitor.lock().unwrap().take(),
      window_adapter: self.window_adapter.lock().unwrap().take(),
      hw_archive: self.hw_archive.lock().unwrap().take(),
      cooling_rollup: self.cooling_rollup.lock().unwrap().take(),
      storage_health: self.storage_health.lock().unwrap().take(),
      scheduled_cleanup: self.scheduled_cleanup.lock().unwrap().take(),
      #[cfg(target_os = "windows")]
      switchbot_scan: self.switchbot_scan.lock().unwrap().take(),
      tray: self.tray.lock().unwrap().take(),
    }
  }
}

async fn run_shutdown_once_with_completion<Start, Shutdown>(
  shutting_down: &AtomicBool,
  shutdown_result: &watch::Sender<Option<Result<(), String>>>,
  start_shutdown: Start,
) -> Result<(), String>
where
  Start: FnOnce() -> Shutdown,
  Shutdown: Future<Output = Result<(), String>> + Send + 'static,
{
  let mut result = shutdown_result.subscribe();
  // Keep the drain alive if its first caller is cancelled, and make every
  // later caller wait for that same drain instead of treating the flag as completion.
  if shutting_down
    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
    .is_ok()
  {
    let shutdown = start_shutdown();
    let result_sender = shutdown_result.clone();
    tokio::spawn(async move {
      result_sender.send_replace(Some(shutdown.await));
    });
  }

  loop {
    if let Some(shutdown_result) = result.borrow_and_update().clone() {
      return shutdown_result;
    }
    result.changed().await.map_err(|_| {
      "shutdown completion channel closed before workers stopped".to_owned()
    })?;
  }
}

async fn terminate_workers(mut workers: ShutdownWorkers) -> Result<(), String> {
  // Stop the source first so no further realtime snapshots are produced, then
  // drain the adapter, then shut down the archive worker.
  if let Some(monitor) = workers.monitor.take() {
    monitor.terminate().await;
  }

  // Stopped alongside the collector, and before the archive: it is
  // another reading source, and there is no point accepting ambient
  // advertisements for an archive that is about to write its final
  // summary.
  #[cfg(target_os = "windows")]
  if let Some(switchbot_scan) = workers.switchbot_scan.take() {
    switchbot_scan.terminate().await;
  }

  if let Some(adapter) = workers.window_adapter.take() {
    adapter.terminate().await;
  }

  if let Some(tray) = workers.tray.take() {
    tray.terminate().await;
  }

  if let Some(hw_archive) = workers.hw_archive.take() {
    hw_archive.terminate().await;
  }

  if let Some(cooling_rollup) = workers.cooling_rollup.take() {
    cooling_rollup.terminate().await;
  }

  if let Some(storage_health) = workers.storage_health.take() {
    storage_health.terminate().await;
  }

  // Next to last: it only ever deletes rows the writers above already
  // wrote, so waiting for it after the writers are gone cannot race a
  // write against the cleanup it is racing today (fire-and-forget,
  // unjoined).
  if let Some(scheduled_cleanup) = workers.scheduled_cleanup.take() {
    let _ = scheduled_cleanup.await;
  }

  // Last, now that every database-backed worker above - including the
  // scheduled cleanup pass, whose deletes route through the same
  // boundary - has drained and stopped writing: close the native
  // database dispatch boundary's live owner, if any (#2134). This joins
  // the two DuckDB lane threads deliberately - and lets the file
  // checkpoint cleanly - instead of letting them be torn down with the
  // process. Every caller of `terminate_all` (`lifecycle::request_quit`,
  // `services::system_service::restart_app[_elevated]`) reaches this, so
  // it is a single site rather than one per quit/restart path. A no-op
  // with the feature disabled or when nothing was ever selected. Must
  // stay last: closing the boundary's owner before the writers above
  // have drained would close the file out from under a still-running
  // archive/cooling/storage-health/cleanup write.
  #[cfg(feature = "duckdb-archive")]
  if let Err(error) = hardviz_core::infrastructure::database::dispatch::shutdown().await {
    return Err(error.to_string());
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  };

  use tokio::sync::{oneshot, watch};

  use super::run_shutdown_once_with_completion;

  #[tokio::test]
  async fn concurrent_shutdown_waits_for_and_receives_the_shared_result() {
    let shutting_down = Arc::new(AtomicBool::new(false));
    let (shutdown_result, _) = watch::channel(None);
    let (started_tx, started_rx) = oneshot::channel();
    let (finish_tx, finish_rx) = oneshot::channel();

    let initiator = {
      let shutting_down = shutting_down.clone();
      let shutdown_result = shutdown_result.clone();
      tokio::spawn(async move {
        run_shutdown_once_with_completion(
          &shutting_down,
          &shutdown_result,
          || async move {
            let _ = started_tx.send(());
            let _ = finish_rx.await;
            Err("native owner close failed".to_owned())
          },
        )
        .await
      })
    };
    started_rx.await.expect("shutdown should start");

    let second_start_called = Arc::new(AtomicBool::new(false));
    let concurrent_call = {
      let shutting_down = shutting_down.clone();
      let shutdown_result = shutdown_result.clone();
      let second_start_called = second_start_called.clone();
      tokio::spawn(async move {
        run_shutdown_once_with_completion(&shutting_down, &shutdown_result, || {
          second_start_called.store(true, Ordering::SeqCst);
          async { Ok(()) }
        })
        .await
      })
    };
    tokio::task::yield_now().await;
    assert!(!concurrent_call.is_finished());
    assert!(!second_start_called.load(Ordering::SeqCst));

    finish_tx
      .send(())
      .expect("shutdown should still be running");
    assert_eq!(
      initiator.await.expect("initiator should complete"),
      Err("native owner close failed".to_owned())
    );
    assert_eq!(
      concurrent_call
        .await
        .expect("concurrent caller should complete"),
      Err("native owner close failed".to_owned())
    );
  }
}
