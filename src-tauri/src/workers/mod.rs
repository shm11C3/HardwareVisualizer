pub mod hardware_archive;
pub mod system_monitor;
pub mod updater;
pub mod libre_hardware_monitor_import;

use std::sync::{Mutex, atomic::AtomicBool};

#[derive(Default)]
pub struct WorkersState {
  pub monitor: Mutex<Option<system_monitor::SystemMonitorController>>,
  pub hw_archive: Mutex<Option<hardware_archive::HardwareArchiveController>>,
  pub lhm_import: Mutex<Option<libre_hardware_monitor_import::LibreHardwareMonitorImportController>>,
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
    let hw_archive = self.hw_archive.lock().unwrap().take();
    let lhm_import = self.lhm_import.lock().unwrap().take();

    if let Some(monitor) = monitor {
      monitor.terminate().await;
    }

    if let Some(hw_archive) = hw_archive {
      hw_archive.terminate().await;
    }

    if let Some(lhm_import) = lhm_import {
      lhm_import.terminate().await;
    }
  }
}
