pub mod hardware_archive;

use std::sync::{Mutex, atomic::AtomicBool};

use crate::adapters::window::WindowAdapter;

#[derive(Default)]
pub struct WorkersState {
  pub monitor: Mutex<Option<hwviz_core::collector::SystemMonitorController>>,
  pub window_adapter: Mutex<Option<WindowAdapter>>,
  pub hw_archive: Mutex<Option<hardware_archive::HardwareArchiveController>>,
  pub shutting_down: AtomicBool,
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

    // Stop the source first so no further snapshots are produced, then
    // drain the adapter, then shut down the archive worker.
    if let Some(monitor) = monitor {
      monitor.terminate().await;
    }

    if let Some(adapter) = window_adapter {
      adapter.terminate().await;
    }

    if let Some(hw_archive) = hw_archive {
      hw_archive.terminate().await;
    }
  }
}
