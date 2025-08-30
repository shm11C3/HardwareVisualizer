/// アプリケーションを再起動する
#[tauri::command]
#[specta::specta]
pub async fn restart_app(app_handle: tauri::AppHandle) {
  crate::services::system_service::restart_app(&app_handle).await;
}
