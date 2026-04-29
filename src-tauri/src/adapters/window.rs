use hwviz_core::models::{GpuMetric, MetricsSnapshot};
use tokio::sync::broadcast::{Receiver, error::RecvError};
use tokio::sync::watch;

use crate::log_warn;
use crate::models::hardware::{GpuMonitorData, HardwareMonitorUpdate};
use tauri_specta::Event as _;

/// Subscribes to the in-process [`hwviz_core::event_bus::EventBus`] and
/// forwards each [`MetricsSnapshot`] to the main window as the existing
/// [`HardwareMonitorUpdate`] Tauri event. This is the only place that
/// translates Core events into a Tauri emit.
pub struct WindowAdapter {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: watch::Sender<bool>,
}

impl WindowAdapter {
  pub fn setup(app_handle: tauri::AppHandle, mut rx: Receiver<MetricsSnapshot>) -> Self {
    let (stop_tx, mut stop_rx) = watch::channel(false);

    let handle = tauri::async_runtime::spawn(async move {
      loop {
        tokio::select! {
          result = rx.recv() => match result {
            Ok(snapshot) => emit_snapshot(&app_handle, snapshot),
            Err(RecvError::Lagged(skipped)) => {
              log_warn!(
                &format!("WindowAdapter lagged, dropped {skipped} snapshot(s)"),
                "adapters::window",
                None::<&str>
              );
            }
            Err(RecvError::Closed) => break,
          },
          changed = stop_rx.changed() => {
            if changed.is_err() || *stop_rx.borrow() {
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

fn emit_snapshot(app_handle: &tauri::AppHandle, snapshot: MetricsSnapshot) {
  let payload = to_hardware_monitor_update(snapshot);
  if let Err(e) = payload.emit(app_handle) {
    log_warn!(
      &format!("failed to emit HardwareMonitorUpdate event: {e}"),
      "adapters::window",
      None::<&str>
    );
  }
}

fn to_hardware_monitor_update(snapshot: MetricsSnapshot) -> HardwareMonitorUpdate {
  HardwareMonitorUpdate {
    cpu_usage: snapshot.cpu_usage,
    memory_usage: snapshot.memory_usage,
    processors_usage: snapshot.processors_usage,
    gpus: snapshot.gpus.into_iter().map(to_gpu_monitor_data).collect(),
  }
}

fn to_gpu_monitor_data(g: GpuMetric) -> GpuMonitorData {
  GpuMonitorData {
    gpu_id: g.gpu_id,
    gpu_name: g.gpu_name,
    gpu_usage: g.gpu_usage,
    gpu_temperature: g.gpu_temperature,
    gpu_source: g.gpu_source,
    gpu_dedicated_memory_usage_kb: g.gpu_dedicated_memory_usage_kb,
    gpu_cooler_level: g.gpu_cooler_level,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_metric(gpu_id: &str, name: &str) -> GpuMetric {
    GpuMetric {
      gpu_id: gpu_id.to_string(),
      gpu_name: name.to_string(),
      gpu_usage: Some(60.0),
      gpu_temperature: Some(55.0),
      gpu_source: "Test".to_string(),
      gpu_dedicated_memory_usage_kb: Some(1024.0),
      gpu_cooler_level: Some(40),
    }
  }

  #[test]
  fn translation_preserves_top_level_fields() {
    let snap = MetricsSnapshot {
      cpu_usage: 12.5,
      memory_usage: 67.0,
      processors_usage: vec![10.0, 20.0, 30.0, 40.0],
      gpus: vec![],
    };
    let update = to_hardware_monitor_update(snap);
    assert_eq!(update.cpu_usage, 12.5);
    assert_eq!(update.memory_usage, 67.0);
    assert_eq!(update.processors_usage, vec![10.0, 20.0, 30.0, 40.0]);
    assert!(update.gpus.is_empty());
  }

  #[test]
  fn translation_preserves_per_gpu_fields() {
    let snap = MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      gpus: vec![
        make_metric("pci:0:2.0", "RTX 4090"),
        make_metric("pci:0:3.0", "RX 7900 XTX"),
      ],
    };
    let update = to_hardware_monitor_update(snap);
    assert_eq!(update.gpus.len(), 2);
    assert_eq!(update.gpus[0].gpu_id, "pci:0:2.0");
    assert_eq!(update.gpus[0].gpu_name, "RTX 4090");
    assert_eq!(update.gpus[0].gpu_usage, Some(60.0));
    assert_eq!(update.gpus[0].gpu_temperature, Some(55.0));
    assert_eq!(update.gpus[0].gpu_source, "Test");
    assert_eq!(update.gpus[0].gpu_dedicated_memory_usage_kb, Some(1024.0));
    assert_eq!(update.gpus[0].gpu_cooler_level, Some(40));
    assert_eq!(update.gpus[1].gpu_id, "pci:0:3.0");
  }

  #[test]
  fn translation_passes_through_none_optionals() {
    let snap = MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      gpus: vec![GpuMetric {
        gpu_id: "x".into(),
        gpu_name: "x".into(),
        gpu_usage: None,
        gpu_temperature: None,
        gpu_source: "x".into(),
        gpu_dedicated_memory_usage_kb: None,
        gpu_cooler_level: None,
      }],
    };
    let update = to_hardware_monitor_update(snap);
    assert!(update.gpus[0].gpu_usage.is_none());
    assert!(update.gpus[0].gpu_temperature.is_none());
    assert!(update.gpus[0].gpu_dedicated_memory_usage_kb.is_none());
    assert!(update.gpus[0].gpu_cooler_level.is_none());
  }
}
