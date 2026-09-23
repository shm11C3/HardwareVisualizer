use std::sync::{Mutex, atomic::AtomicBool};

use hardviz_core::monitoring::MonitoringState;

use crate::adapters::tray::TrayAdapter;
use crate::adapters::window::WindowAdapter;

#[derive(Default)]
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
  /// Lifecycle state of sensor collection. Owned here because the
  /// lifecycle module already locks workers on quit; colocating the
  /// state avoids a second `Mutex` around the same critical section.
  /// Phase 5 only writes `Stopped` from `lifecycle::request_quit`;
  /// the `Paused` / `Running` transitions become user-visible with
  /// #1275 and #1401.
  pub monitoring_state: Mutex<MonitoringState>,
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
    if self
      .shutting_down
      .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
      return Ok(());
    }
    let monitor = self.monitor.lock().unwrap().take();
    let window_adapter = self.window_adapter.lock().unwrap().take();
    let hw_archive = self.hw_archive.lock().unwrap().take();
    let cooling_rollup = self.cooling_rollup.lock().unwrap().take();
    let storage_health = self.storage_health.lock().unwrap().take();
    let scheduled_cleanup = self.scheduled_cleanup.lock().unwrap().take();
    #[cfg(target_os = "windows")]
    let switchbot_scan = self.switchbot_scan.lock().unwrap().take();
    let tray = self.tray.lock().unwrap().take();

    // Stop the source first so no further realtime snapshots are produced, then
    // drain the adapter, then shut down the archive worker.
    if let Some(monitor) = monitor {
      monitor.terminate().await;
    }

    // Stopped alongside the collector, and before the archive: it is
    // another reading source, and there is no point accepting ambient
    // advertisements for an archive that is about to write its final
    // summary.
    #[cfg(target_os = "windows")]
    if let Some(switchbot_scan) = switchbot_scan {
      switchbot_scan.terminate().await;
    }

    if let Some(adapter) = window_adapter {
      adapter.terminate().await;
    }

    if let Some(tray) = tray {
      tray.terminate().await;
    }

    if let Some(hw_archive) = hw_archive {
      hw_archive.terminate().await;
    }

    if let Some(cooling_rollup) = cooling_rollup {
      cooling_rollup.terminate().await;
    }

    if let Some(storage_health) = storage_health {
      storage_health.terminate().await;
    }

    // Next to last: it only ever deletes rows the writers above already
    // wrote, so waiting for it after the writers are gone cannot race a
    // write against the cleanup it is racing today (fire-and-forget,
    // unjoined).
    if let Some(scheduled_cleanup) = scheduled_cleanup {
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
    if let Err(e) = hardviz_core::infrastructure::database::dispatch::shutdown().await {
      return Err(e.to_string());
    }

    Ok(())
  }
}
