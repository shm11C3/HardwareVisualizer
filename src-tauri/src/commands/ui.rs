use tauri::App;

use crate::services::ui_service;

pub fn init(app: &mut App) {
  let app_handle = app.handle();
  let _ = ui_service::apply_saved_window_decoration(app_handle);
}

///
/// ウィンドウの装飾状態を設定
///
#[tauri::command]
#[specta::specta]
pub fn set_decoration(
  window: tauri::WebviewWindow,
  is_decorated: bool,
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  ui_service::set_window_decoration(&window, is_decorated)?;
  ui_service::persist_window_decoration(&app_handle, is_decorated)?;

  Ok(())
}
