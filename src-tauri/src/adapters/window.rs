use hwviz_core::models::{GpuMetric, MetricsSnapshot};
use tauri::Manager as _;
use tokio::sync::broadcast::{Receiver, error::RecvError};
use tokio::sync::watch;

use crate::commands::settings;
use crate::enums::settings::TemperatureUnit;
use crate::log_warn;
use crate::models::hardware::{GpuMonitorData, HardwareMonitorUpdate};
use tauri_specta::Event as _;

/// Subscribes to the in-process [`hwviz_core::event_bus::EventBus`] and
/// forwards each [`MetricsSnapshot`] to the main window as the existing
/// [`HardwareMonitorUpdate`] Tauri event. This is the only place that
/// translates Core events into a Tauri emit. It also applies the user's
/// preferred temperature unit (`Celsius` / `Fahrenheit`) — the Core
/// collector always publishes raw °C, and presentation conversion lives
/// here at the App-side boundary.
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
  let temp_unit = current_temperature_unit(app_handle);
  let payload = to_hardware_monitor_update(snapshot, &temp_unit);
  if let Err(e) = payload.emit(app_handle) {
    log_warn!(
      &format!("failed to emit HardwareMonitorUpdate event: {e}"),
      "adapters::window",
      None::<&str>
    );
  }
}

/// Read the user's preferred temperature unit from `settings::AppState`.
/// Defaults to Celsius if the state hasn't been registered yet (early
/// startup) or if the settings mutex is poisoned. We don't want a
/// poisoned lock to bring down the entire snapshot-forwarding loop.
fn current_temperature_unit(app_handle: &tauri::AppHandle) -> TemperatureUnit {
  app_handle
    .try_state::<settings::AppState>()
    .and_then(|state| {
      state
        .settings
        .lock()
        .ok()
        .map(|s| s.temperature_unit.clone())
    })
    .unwrap_or(TemperatureUnit::Celsius)
}

fn to_hardware_monitor_update(
  snapshot: MetricsSnapshot,
  temp_unit: &TemperatureUnit,
) -> HardwareMonitorUpdate {
  HardwareMonitorUpdate {
    cpu_usage: snapshot.cpu_usage,
    memory_usage: snapshot.memory_usage,
    processors_usage: snapshot.processors_usage,
    gpus: snapshot
      .gpus
      .into_iter()
      .map(|g| to_gpu_monitor_data(g, temp_unit))
      .collect(),
  }
}

fn to_gpu_monitor_data(g: GpuMetric, temp_unit: &TemperatureUnit) -> GpuMonitorData {
  GpuMonitorData {
    gpu_id: g.gpu_id,
    gpu_name: g.gpu_name,
    gpu_usage: g.gpu_usage,
    gpu_temperature: g.gpu_temperature.map(|t| convert_temperature(t, temp_unit)),
    gpu_source: g.gpu_source,
    gpu_dedicated_memory_usage_kb: g.gpu_dedicated_memory_usage_kb,
    gpu_cooler_level: g.gpu_cooler_level,
  }
}

/// Convert raw °C from Core into the user's preferred unit, rounded.
fn convert_temperature(celsius: f32, unit: &TemperatureUnit) -> f32 {
  match unit {
    TemperatureUnit::Celsius => celsius.round(),
    TemperatureUnit::Fahrenheit => (celsius * 9.0 / 5.0 + 32.0).round(),
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
      processes: vec![],
    };
    let update = to_hardware_monitor_update(snap, &TemperatureUnit::Celsius);
    assert_eq!(update.cpu_usage, 12.5);
    assert_eq!(update.memory_usage, 67.0);
    assert_eq!(update.processors_usage, vec![10.0, 20.0, 30.0, 40.0]);
    assert!(update.gpus.is_empty());
  }

  #[test]
  fn translation_preserves_per_gpu_fields_in_celsius() {
    let snap = MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      gpus: vec![
        make_metric("pci:0:2.0", "RTX 4090"),
        make_metric("pci:0:3.0", "RX 7900 XTX"),
      ],
      processes: vec![],
    };
    let update = to_hardware_monitor_update(snap, &TemperatureUnit::Celsius);
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
  fn temperature_converted_to_fahrenheit() {
    let snap = MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      gpus: vec![GpuMetric {
        gpu_id: "x".into(),
        gpu_name: "x".into(),
        gpu_usage: None,
        gpu_temperature: Some(100.0),
        gpu_source: "Test".into(),
        gpu_dedicated_memory_usage_kb: None,
        gpu_cooler_level: None,
      }],
      processes: vec![],
    };
    let update = to_hardware_monitor_update(snap, &TemperatureUnit::Fahrenheit);
    assert_eq!(update.gpus[0].gpu_temperature, Some(212.0));
  }

  #[test]
  fn temperature_celsius_rounded() {
    let snap = MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      gpus: vec![GpuMetric {
        gpu_id: "x".into(),
        gpu_name: "x".into(),
        gpu_usage: None,
        gpu_temperature: Some(65.7),
        gpu_source: "Test".into(),
        gpu_dedicated_memory_usage_kb: None,
        gpu_cooler_level: None,
      }],
      processes: vec![],
    };
    let update = to_hardware_monitor_update(snap, &TemperatureUnit::Celsius);
    assert_eq!(update.gpus[0].gpu_temperature, Some(66.0));
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
      processes: vec![],
    };
    let update = to_hardware_monitor_update(snap, &TemperatureUnit::Fahrenheit);
    assert!(update.gpus[0].gpu_usage.is_none());
    assert!(update.gpus[0].gpu_temperature.is_none());
    assert!(update.gpus[0].gpu_dedicated_memory_usage_kb.is_none());
    assert!(update.gpus[0].gpu_cooler_level.is_none());
  }

  #[test]
  fn convert_temperature_celsius_passthrough() {
    assert_eq!(convert_temperature(25.0, &TemperatureUnit::Celsius), 25.0);
    assert_eq!(convert_temperature(25.4, &TemperatureUnit::Celsius), 25.0);
    assert_eq!(convert_temperature(25.5, &TemperatureUnit::Celsius), 26.0);
  }

  #[test]
  fn convert_temperature_fahrenheit() {
    assert_eq!(convert_temperature(0.0, &TemperatureUnit::Fahrenheit), 32.0);
    assert_eq!(
      convert_temperature(100.0, &TemperatureUnit::Fahrenheit),
      212.0
    );
    assert_eq!(
      convert_temperature(-40.0, &TemperatureUnit::Fahrenheit),
      -40.0
    );
  }
}
