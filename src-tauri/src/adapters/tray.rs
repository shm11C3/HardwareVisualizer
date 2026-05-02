//! System-tray menu adapter — #1422 / #1401.
//!
//! Provides a minimal `Open` / `Quit` menu plus a left-click that
//! restores the main window. Calls [`crate::lifecycle::request_quit`]
//! (shipped by Phase 5 of #1402) to drive the lifecycle state machine
//! to `Stopped` on quit. Live metric rendering subscribes to the same
//! Core event stream as the main window, so it keeps updating without
//! involving React while the window is hidden. The Pause / Resume
//! entries belong to #1424.
//!
//! ## Visibility policy
//! The tray is registered unconditionally on app startup. The persisted
//! close-to-tray preference is handled by `crate::lifecycle`; this
//! adapter stays policy-free.
//!
//! ## Platform notes
//! - **Windows / macOS**: left-clicking the icon restores the window;
//!   right-click opens the menu.
//! - **Linux**: tray support depends on the desktop environment.
//!   AppIndicator-based environments (GNOME with the AppIndicator
//!   extension, KDE) usually show the menu on any click and never
//!   forward a raw click event, so the left-click-restores path may be
//!   silently inactive. Users still reach `Open` through the menu, and
//!   `Quit` through the menu always works. Environments without an
//!   indicator implementation may fail to register the tray entirely;
//!   when that happens [`TrayAdapter::setup`] returns the error to the
//!   caller, which logs, disables close-to-tray for the current
//!   session, and continues without a tray.

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt as _;
use tokio::sync::broadcast::{Receiver, error::RecvError};
use tokio::sync::watch;

use crate::log_warn;
use crate::tray::surface::{self, TraySurface};
use crate::tray::widget::{
  STORE_KEY_TRAY_WIDGET, TrayFrame, TrayWidgetSettings, build_frame, should_skip_frame,
};

use hardviz_core::models::MetricsSnapshot;

use crate::commands::settings;
use crate::enums::settings::TemperatureUnit;

/// Owns the tray surface task. The surface itself lives inside the
/// updater task so platform-specific handles stay alive until
/// termination.
pub struct TrayAdapter {
  handle: tauri::async_runtime::JoinHandle<()>,
  stop_tx: watch::Sender<bool>,
}

impl TrayAdapter {
  /// Build the tray icon, menu, and event handlers.
  ///
  /// The default window icon is reused as the tray icon — Phase 6
  /// keeps the visual minimal. A dedicated tray-tuned asset can land
  /// alongside #1401's live widget.
  pub fn setup(app: &tauri::App, rx: Receiver<MetricsSnapshot>) -> tauri::Result<Self> {
    let surface = surface::create(app)?;
    let (handle, stop_tx) = spawn_widget_updater(app.handle().clone(), surface, rx);

    Ok(Self { handle, stop_tx })
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}

fn spawn_widget_updater(
  app_handle: AppHandle,
  surface: Box<dyn TraySurface>,
  mut rx: Receiver<MetricsSnapshot>,
) -> (tauri::async_runtime::JoinHandle<()>, watch::Sender<bool>) {
  let (stop_tx, mut stop_rx) = watch::channel(false);

  let handle = tauri::async_runtime::spawn(async move {
    let mut previous_frame: Option<TrayFrame> = None;
    let mut previous_update_at: Option<std::time::Instant> = None;

    loop {
      tokio::select! {
        result = rx.recv() => match result {
          Ok(snapshot) => {
            let settings = current_widget_settings(&app_handle);
            let temperature_unit = current_temperature_unit(&app_handle);
            let next_frame = build_frame(&snapshot, &settings, &temperature_unit);
            let now = std::time::Instant::now();
            let elapsed = previous_update_at.map(|last| now.duration_since(last));

            if should_skip_frame(
              previous_frame.as_ref(),
              &next_frame,
              elapsed,
              settings.update_interval(),
            ) {
              continue;
            }

            surface.apply_frame(&next_frame);
            previous_frame = Some(next_frame);
            previous_update_at = Some(now);
          }
          Err(RecvError::Lagged(skipped)) => {
            log_warn!(
              &format!("TrayAdapter lagged, dropped {skipped} snapshot(s)"),
              "adapters::tray",
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

    surface.close();
  });

  (handle, stop_tx)
}

fn current_widget_settings(app_handle: &AppHandle) -> TrayWidgetSettings {
  app_handle
    .store("store.json")
    .ok()
    .and_then(|store| store.get(STORE_KEY_TRAY_WIDGET))
    .and_then(|value| serde_json::from_value::<TrayWidgetSettings>(value).ok())
    .unwrap_or_default()
    .normalized()
}

fn current_temperature_unit(app_handle: &AppHandle) -> TemperatureUnit {
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

pub(crate) fn restore_main_window(app: &AppHandle) {
  let Some(window) = app.get_webview_window("main") else {
    log_warn!(
      "main window not found while handling tray Open",
      "adapters::tray::restore_main_window",
      None::<&str>
    );
    return;
  };

  // Best-effort: each call may legitimately fail (window already
  // visible, already in the foreground, OS denying focus) without
  // affecting the others. Logging keeps the trail without aborting.
  if let Err(e) = window.show() {
    log_warn!(
      &format!("failed to show main window from tray: {e}"),
      "adapters::tray::restore_main_window",
      None::<&str>
    );
  }
  if let Err(e) = window.unminimize() {
    log_warn!(
      &format!("failed to unminimize main window from tray: {e}"),
      "adapters::tray::restore_main_window",
      None::<&str>
    );
  }
  if let Err(e) = window.set_focus() {
    log_warn!(
      &format!("failed to focus main window from tray: {e}"),
      "adapters::tray::restore_main_window",
      None::<&str>
    );
  }
}

pub(crate) fn spawn_quit(app: AppHandle) {
  // `request_quit` is async (it awaits worker termination); the menu
  // callback runs on the main thread and must return promptly, so we
  // hand the work off to the tokio runtime.
  tauri::async_runtime::spawn(async move {
    crate::lifecycle::request_quit(app).await;
  });
}
