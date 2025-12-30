use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

const STORE_FILENAME: &str = "store.json";
const KEY_WINDOW_DECORATED: &str = "window_decorated";

/// Apply saved decoration state on app startup
pub fn apply_saved_window_decoration(app: &AppHandle) -> Result<(), String> {
  let store = app
    .store(STORE_FILENAME)
    .map_err(|e| format!("Failed to open store: {e}"))?;

  let Some(window) = app.get_webview_window("main") else {
    return Err("Main window not found".to_string());
  };

  if let Some(is_decorated) = store.get(KEY_WINDOW_DECORATED).and_then(|v| v.as_bool()) {
    set_window_decoration(&window, is_decorated)?;
  } else {
    // When no saved state, default to decorated
    set_window_decoration(&window, true)?;
  }

  Ok(())
}

/// Apply window decoration state
pub fn set_window_decoration(
  window: &tauri::WebviewWindow,
  is_decorated: bool,
) -> Result<(), String> {
  window
    .set_fullscreen(!is_decorated)
    .map_err(|e| format!("Failed to set fullscreen: {e}"))
}

/// Write current decoration state
pub fn persist_window_decoration(
  app: &AppHandle,
  is_decorated: bool,
) -> Result<(), String> {
  let store = app
    .store(STORE_FILENAME)
    .map_err(|e| format!("Failed to open store: {e}"))?;

  // tauri-plugin-store v2: set + save
  // set does not return Result so call it directly
  store.set(KEY_WINDOW_DECORATED, is_decorated);
  store
    .save()
    .map_err(|e| format!("Failed to save store: {e}"))
}
