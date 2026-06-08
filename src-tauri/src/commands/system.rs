const E2E_ENV: &str = "HARDVIZ_E2E";
const E2E_FORCE_CLOSE_TO_TRAY_PROMPT_ENV: &str = "HARDVIZ_E2E_FORCE_CLOSE_TO_TRAY_PROMPT";

fn is_env_enabled(key: &str) -> bool {
  std::env::var(key).as_deref() == Ok("1")
}

fn is_e2e_close_to_tray_prompt_forced() -> bool {
  is_env_enabled(E2E_ENV) && is_env_enabled(E2E_FORCE_CLOSE_TO_TRAY_PROMPT_ENV)
}

fn close_to_tray_available_for_command(
  lifecycle_available: bool,
  force_e2e_prompt: bool,
) -> bool {
  force_e2e_prompt || lifecycle_available
}

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
  let lifecycle_available = crate::lifecycle::is_close_to_tray_available(&app_handle);

  Ok(close_to_tray_available_for_command(
    lifecycle_available,
    is_e2e_close_to_tray_prompt_forced(),
  ))
}

/// Mark the frontend close-to-tray dialog listener as ready.
#[tauri::command]
#[specta::specta]
pub async fn mark_close_to_tray_listener_ready(
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  crate::lifecycle::mark_close_to_tray_listener_ready(app_handle).await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn close_to_tray_command_uses_lifecycle_availability_by_default() {
    assert!(close_to_tray_available_for_command(true, false));
    assert!(!close_to_tray_available_for_command(false, false));
  }

  #[test]
  fn close_to_tray_command_can_force_e2e_prompt_without_lifecycle_availability() {
    assert!(close_to_tray_available_for_command(false, true));
  }
}
