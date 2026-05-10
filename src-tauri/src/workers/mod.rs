use std::sync::{Mutex, atomic::AtomicBool};

use hardviz_core::monitoring::MonitoringState;

use crate::adapters::tray::TrayAdapter;
use crate::adapters::window::WindowAdapter;

#[derive(Default)]
pub struct WorkersState {
  pub monitor: Mutex<Option<hardviz_core::collector::SystemMonitorController>>,
  pub window_adapter: Mutex<Option<WindowAdapter>>,
  pub hw_archive: Mutex<Option<hardviz_core::persistence::ArchiveController>>,
  pub storage_smart: Mutex<Option<hardviz_core::persistence::StorageSmartController>>,

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
    if self
      .shutting_down
      .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
      return;
    }
    let monitor = self.monitor.lock().unwrap().take();
    let window_adapter = self.window_adapter.lock().unwrap().take();
    let hw_archive = self.hw_archive.lock().unwrap().take();
    let storage_smart = self.storage_smart.lock().unwrap().take();
    let tray = self.tray.lock().unwrap().take();

    // Stop the source first so no further snapshots are produced, then
    // drain the adapter, then shut down the archive worker.
    if let Some(monitor) = monitor {
      monitor.terminate().await;
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

    if let Some(storage_smart) = storage_smart {
      storage_smart.terminate().await;
    }
  }
}
