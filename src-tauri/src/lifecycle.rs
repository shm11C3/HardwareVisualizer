//! App-side lifecycle wiring for Core start/stop.
//!
//! Closing the main window is user-configurable: the app can either
//! hide to the tray and keep monitoring, or stop workers and exit. The
//! transitional env-var override from #1408 remains available for tests
//! and debug sessions.

use std::{
  env,
  sync::atomic::{AtomicBool, Ordering},
};

use tauri::{AppHandle, Emitter, Manager, Window};
use tauri_plugin_store::StoreExt;

use crate::log_warn;
use crate::workers::WorkersState;

/// Env var that switches the close-button behavior from "quit" to
/// "hide window, keep monitoring". Off by default — release builds
/// preserve the historical quit-on-close behavior until the
/// user-facing UX in #1275 ships.
pub const CLOSE_TO_BACKGROUND_ENV: &str = "HARDVIZ_CLOSE_TO_BACKGROUND";
const STORE_FILENAME: &str = "store.json";
const KEY_CLOSE_TO_TRAY: &str = "closeToTray";
const KEY_CLOSE_TO_TRAY_CHOICE_MADE: &str = "closeToTrayChoiceMade";
const EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED: &str = "close-to-tray-choice-requested";

/// Session-only lifecycle capability. `tray_available` starts true and
/// is flipped off when tray setup fails, so persisted preferences never
/// hide the only window on platforms without a working tray.
pub struct CloseToTrayRuntimeState {
  tray_available: AtomicBool,
  listener_ready: AtomicBool,
  choice_request_pending: AtomicBool,
}

impl Default for CloseToTrayRuntimeState {
  fn default() -> Self {
    Self {
      tray_available: AtomicBool::new(true),
      listener_ready: AtomicBool::new(false),
      choice_request_pending: AtomicBool::new(false),
    }
  }
}

impl CloseToTrayRuntimeState {
  pub fn disable_for_session(&self) {
    self.tray_available.store(false, Ordering::SeqCst);
  }

  pub fn is_available(&self) -> bool {
    self.tray_available.load(Ordering::SeqCst)
  }

  fn mark_listener_ready(&self) {
    self.listener_ready.store(true, Ordering::SeqCst);
  }

  fn is_listener_ready(&self) -> bool {
    self.listener_ready.load(Ordering::SeqCst)
  }

  fn set_choice_request_pending(&self) {
    self.choice_request_pending.store(true, Ordering::SeqCst);
  }

  fn take_choice_request_pending(&self) -> bool {
    self.choice_request_pending.swap(false, Ordering::SeqCst)
  }
}

/// `true` when the env var holds a truthy value (`1`, `true`, `yes`,
/// case-insensitive). Anything else — unset, empty, or any other
/// value — reads as off.
pub fn should_close_to_background() -> bool {
  env::var(CLOSE_TO_BACKGROUND_ENV)
    .ok()
    .as_deref()
    .map(is_truthy)
    .unwrap_or(false)
}

fn is_truthy(s: &str) -> bool {
  matches!(s.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes")
}

/// Hook into `on_window_event(CloseRequested)`. The Tauri callback
/// runs on the UI thread, so we always spawn the work onto the async
/// runtime — `terminate_all` awaits inside.
pub fn on_close_requested(window: &Window) {
  let app = window.app_handle().clone();
  let win = window.clone();
  tauri::async_runtime::spawn(async move {
    handle_close_request(app, win).await;
  });
}

/// Decide what closing the main window means.
async fn handle_close_request(app: AppHandle, window: Window) {
  if should_close_to_background() {
    if let Err(e) = hide_window_on_close(&window) {
      log_warn!(
        &format!(
          "failed to hide main window on env override close; quitting instead: {e}"
        ),
        "lifecycle::handle_close_request",
        None::<&str>
      );
      request_quit(app).await;
    }
    return;
  }

  if !is_close_to_tray_available(&app) {
    request_quit(app).await;
    return;
  }

  match read_close_to_tray_settings(&app) {
    Ok(settings) if !settings.choice_made => {
      let state = app.state::<CloseToTrayRuntimeState>();
      state.set_choice_request_pending();

      if state.is_listener_ready() {
        match emit_close_to_tray_choice_request(&app) {
          Ok(()) => {
            state.take_choice_request_pending();
          }
          Err(e) => {
            state.take_choice_request_pending();
            log_warn!(
              &format!("failed to emit close-to-tray choice request: {e}"),
              "lifecycle::handle_close_request",
              None::<&str>
            );
            request_quit(app).await;
          }
        }
      }
    }
    Ok(settings) if settings.close_to_tray => {
      if let Err(e) = hide_window_on_close(&window) {
        log_warn!(
          &format!("failed to hide main window on close; quitting instead: {e}"),
          "lifecycle::handle_close_request",
          None::<&str>
        );
        request_quit(app).await;
      }
    }
    Ok(_) => request_quit(app).await,
    Err(e) => {
      log_warn!(
        &format!("failed to read close-to-tray setting; quitting on close: {e}"),
        "lifecycle::handle_close_request",
        None::<&str>
      );
      request_quit(app).await;
    }
  }
}

pub async fn mark_close_to_tray_listener_ready(app: AppHandle) -> Result<(), String> {
  let state = app.state::<CloseToTrayRuntimeState>();
  state.mark_listener_ready();

  if state.take_choice_request_pending()
    && let Err(e) = emit_close_to_tray_choice_request(&app)
  {
    log_warn!(
      &format!("failed to emit pending close-to-tray choice request: {e}"),
      "lifecycle::mark_close_to_tray_listener_ready",
      None::<&str>
    );
    request_quit(app).await;
    return Err(e);
  }

  Ok(())
}

fn emit_close_to_tray_choice_request(app: &AppHandle) -> Result<(), String> {
  app
    .emit(EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED, ())
    .map_err(|e| e.to_string())
}

pub fn is_close_to_tray_available(app: &AppHandle) -> bool {
  app
    .try_state::<CloseToTrayRuntimeState>()
    .map(|state| state.is_available())
    .unwrap_or(true)
}

fn hide_window_on_close(window: &Window) -> Result<(), tauri::Error> {
  window.hide()
}

struct CloseToTraySettings {
  close_to_tray: bool,
  choice_made: bool,
}

fn read_close_to_tray_settings(app: &AppHandle) -> Result<CloseToTraySettings, String> {
  let store = app
    .store(STORE_FILENAME)
    .map_err(|e| format!("failed to open store: {e}"))?;

  Ok(CloseToTraySettings {
    close_to_tray: store
      .get(KEY_CLOSE_TO_TRAY)
      .and_then(|value| value.as_bool())
      .unwrap_or(false),
    choice_made: store
      .get(KEY_CLOSE_TO_TRAY_CHOICE_MADE)
      .and_then(|value| value.as_bool())
      .unwrap_or(false),
  })
}

/// Stop Core cleanly and exit the process.
///
/// Order matters:
/// 1. Move the monitoring state to `Stopped`. Best-effort: a second
///    call returns `InvalidTransition`, which we drop — terminal
///    states are idempotent for callers.
/// 2. `WorkersState::terminate_all` shuts the collector first, then
///    the window adapter, then the archive worker. The archive worker
///    writes a final summary on the way out so DB writes are flushed.
/// 3. `app.exit(0)`.
pub async fn request_quit(app: AppHandle) {
  let ws = app.state::<WorkersState>();
  // The state-machine error here means another caller already moved
  // us to Stopped; the workers terminate guard handles double-call
  // safety, so we don't need to short-circuit on it.
  let _ = ws.monitoring_state.lock().unwrap().stop();
  ws.terminate_all().await;
  app.exit(0);
}

#[cfg(test)]
mod tests {
  use super::*;

  // The env-var check uses process-global state; running multiple
  // tests against it in parallel would race. We can still test the
  // pure parser without touching the environment.

  #[test]
  fn truthy_values_match_case_insensitively() {
    assert!(is_truthy("1"));
    assert!(is_truthy("true"));
    assert!(is_truthy("TRUE"));
    assert!(is_truthy("True"));
    assert!(is_truthy("yes"));
    assert!(is_truthy("YES"));
  }

  #[test]
  fn truthy_tolerates_whitespace() {
    assert!(is_truthy(" 1 "));
    assert!(is_truthy("\ttrue\n"));
  }

  #[test]
  fn other_values_read_as_off() {
    assert!(!is_truthy(""));
    assert!(!is_truthy("0"));
    assert!(!is_truthy("false"));
    assert!(!is_truthy("no"));
    assert!(!is_truthy("on")); // intentionally not accepted — keep the set tight
    assert!(!is_truthy("anything"));
  }
}
