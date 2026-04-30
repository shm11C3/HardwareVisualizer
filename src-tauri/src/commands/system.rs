/// Restart the application
#[tauri::command]
#[specta::specta]
pub async fn restart_app(app_handle: tauri::AppHandle) {
  crate::services::system_service::restart_app(&app_handle).await;
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
