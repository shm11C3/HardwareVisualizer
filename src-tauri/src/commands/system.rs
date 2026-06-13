/// Restart the application
#[tauri::command]
#[specta::specta]
pub async fn restart_app(app_handle: tauri::AppHandle) {
  crate::services::system_service::restart_app(&app_handle).await;
}

/// Returns whether the current process is running with administrator privileges.
#[tauri::command]
#[specta::specta]
pub fn is_process_elevated() -> Result<bool, String> {
  crate::services::system_service::is_process_elevated()
}

/// Stop monitoring and exit the process.
///
/// Phase 5 (#1408): the explicit Quit path. Keep it gated to the
/// internal callers that the lifecycle module owns — Phase 6's tray
/// menu and the future "Quit" UX from #1275. Until those land, this
/// command is reachable only from devtools / tauri-specta-generated
/// bindings, which is enough to validate the cleanup ordering.
#[tauri::command]
#[specta::specta]
pub async fn quit_app(app_handle: tauri::AppHandle) {
  crate::lifecycle::request_quit(app_handle).await;
}

/// Returns whether close-to-tray can safely be enabled in this session.
#[tauri::command]
#[specta::specta]
pub fn is_close_to_tray_available(app_handle: tauri::AppHandle) -> Result<bool, String> {
  Ok(crate::lifecycle::is_close_to_tray_available(&app_handle))
}

/// Mark the frontend close-to-tray dialog listener as ready.
#[tauri::command]
#[specta::specta]
pub async fn mark_close_to_tray_listener_ready(
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  crate::lifecycle::mark_close_to_tray_listener_ready(app_handle).await
}
