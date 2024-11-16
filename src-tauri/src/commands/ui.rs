use tauri::App;
use tauri_plugin_store::StoreExt;

#[warn(unused_must_use)]
pub fn init(app: &mut App, window: tauri::WebviewWindow) {
  let store = app.store("store.json").unwrap();

  // 設定値をロードしてウィンドウの初期状態を設定
  if let Some(is_decorated) = store.get("window_decorated").and_then(|v| v.as_bool()) {
    set_decoration(window, is_decorated);
  }
}

#[tauri::command]
pub fn set_decoration(window: tauri::WebviewWindow, is_decorated: bool) {
  let _ = window.set_fullscreen(!is_decorated);
  let _ = window.set_decorations(is_decorated);
}
