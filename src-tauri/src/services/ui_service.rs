use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

const STORE_FILENAME: &str = "store.json";
const KEY_WINDOW_DECORATED: &str = "window_decorated";

/// アプリ起動時に保存された装飾状態を適用
pub fn apply_saved_window_decoration(app: &AppHandle) -> Result<(), String> {
  let store = app
    .store(STORE_FILENAME)
    .map_err(|e| format!("Failed to open store: {e}"))?;

  if let Some(is_decorated) = store.get(KEY_WINDOW_DECORATED).and_then(|v| v.as_bool()) {
    if let Some(window) = app.get_webview_window("main") {
      set_window_decoration(&window, is_decorated)?;
    } else {
      return Err("Main window not found".into());
    }
  }
  Ok(())
}

/// ウィンドウ装飾状態を反映)
pub fn set_window_decoration(
  window: &tauri::WebviewWindow,
  is_decorated: bool,
) -> Result<(), String> {
  window
    .set_fullscreen(!is_decorated)
    .map_err(|e| format!("Failed to set fullscreen: {e}"))
}

///  現在の装飾状態を書き込む
pub fn persist_window_decoration(
  app: &AppHandle,
  is_decorated: bool,
) -> Result<(), String> {
  let store = app
    .store(STORE_FILENAME)
    .map_err(|e| format!("Failed to open store: {e}"))?;

  // tauri-plugin-store v2: set + save
  // set は Result を返さないためそのまま呼び出し
  store.set(KEY_WINDOW_DECORATED, is_decorated);
  store
    .save()
    .map_err(|e| format!("Failed to save store: {e}"))
}
