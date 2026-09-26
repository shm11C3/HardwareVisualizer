use std::{
  future::Future,
  sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
  },
};
use tokio::sync::watch;

use hardviz_core::monitoring::MonitoringState;

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
  pub async fn terminate_all(&self) -> Result<(), String> {
    self.terminate_all_with(close_native_database_owner).await
  }

  async fn terminate_all_with<Close, CloseFuture>(
    &self,
    close_native_database: Close,
  ) -> Result<(), String>
  where
    Close: FnOnce() -> CloseFuture + Send + 'static,
    CloseFuture: Future<Output = Result<(), String>> + Send + 'static,
  {
    let mut shutdown_result = self.shutdown_result.subscribe();
    if self
      .shutting_down
      .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
      .is_ok()
    {
      let workers = self.take_for_shutdown();
      let result_sender = self.shutdown_result.clone();
      tokio::spawn(async move {
        let result = terminate_workers(workers, close_native_database).await;
        result_sender.send_replace(Some(result));
      });
    }

    loop {
      if let Some(result) = shutdown_result.borrow_and_update().clone() {
        return result;
      }
      shutdown_result.changed().await.map_err(|_| {
        "shutdown completion channel closed before workers stopped".to_owned()
      })?;
    }
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

async fn terminate_workers<Close, CloseFuture>(
  mut workers: ShutdownWorkers,
  close_native_database: Close,
) -> Result<(), String>
where
  Close: FnOnce() -> CloseFuture,
  CloseFuture: Future<Output = Result<(), String>>,
{
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
  close_native_database().await
}

async fn close_native_database_owner() -> Result<(), String> {
  #[cfg(feature = "duckdb-archive")]
  if let Err(error) = hardviz_core::infrastructure::database::dispatch::shutdown().await {
    hardviz_core::log_error!(
      "Failed to close the native database owner during shutdown",
      "workers::WorkersState::terminate_all",
      Some(error.to_string())
    );
    return Err(format!(
      "failed to close the native database owner: {error}"
    ));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::{sync::Arc, time::Duration};

  use tokio::sync::oneshot;

  use super::WorkersState;

  #[tokio::test]
  async fn concurrent_or_cancelled_termination_waits_for_the_active_drain() {
    let workers = Arc::new(WorkersState::default());
    let (drain_started_tx, drain_started_rx) = oneshot::channel();
    let (finish_drain_tx, finish_drain_rx) = oneshot::channel();
    workers
      .scheduled_cleanup
      .lock()
      .unwrap()
      .replace(tokio::spawn(async move {
        let _ = drain_started_tx.send(());
        let _ = finish_drain_rx.await;
      }));

    let first_workers = Arc::clone(&workers);
    let first = tokio::spawn(async move { first_workers.terminate_all().await });
    drain_started_rx
      .await
      .expect("cleanup should begin draining");

    let second_workers = Arc::clone(&workers);
    let mut second = tokio::spawn(async move { second_workers.terminate_all().await });
    assert!(
      tokio::time::timeout(Duration::from_millis(20), &mut second)
        .await
        .is_err(),
      "concurrent termination must wait until the first drain completes"
    );

    first.abort();
    assert!(first.await.is_err(), "the first waiter should be cancelled");
    assert!(
      tokio::time::timeout(Duration::from_millis(20), &mut second)
        .await
        .is_err(),
      "cancelling the first waiter must not cancel the shared drain"
    );

    finish_drain_tx
      .send(())
      .expect("cleanup should still be running");
    assert!(second.await.unwrap().is_ok());
  }

  #[tokio::test]
  async fn native_database_close_failure_is_returned_to_the_caller() {
    let workers = WorkersState::default();
    let result = workers
      .terminate_all_with(|| async { Err("injected native close failure".to_owned()) })
      .await;

    assert_eq!(result, Err("injected native close failure".to_owned()));
  }
}
