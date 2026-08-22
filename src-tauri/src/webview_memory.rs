//! Windows WebView2 suspension for hidden App-owned windows.

use tauri::{Manager, WebviewWindow, Window};

#[cfg(target_os = "windows")]
use crate::{log_debug, log_warn};

pub fn suspend_for_window(window: &Window) {
  let label = window.label().to_string();
  if let Some(webview) = window.app_handle().get_webview_window(&label) {
    suspend(&webview);
  }
}

pub fn suspend(window: &WebviewWindow) {
  set_suspended(window, true);
}

pub fn resume(window: &WebviewWindow) {
  set_suspended(window, false);
}

#[cfg(target_os = "windows")]
fn set_suspended(window: &WebviewWindow, suspended: bool) {
  use webview2_com::{
    Microsoft::Web::WebView2::Win32::ICoreWebView2_3, TrySuspendCompletedHandler,
  };
  use windows_core::Interface;

  let label = window.label().to_string();
  let operation_label = label.clone();
  let dispatch_result = window.with_webview(move |webview| {
    let callback_label = operation_label.clone();
    let operation: windows_core::Result<()> = (|| unsafe {
      let controller = webview.controller();
      let core_webview = controller.CoreWebView2()?;
      match suspended {
        false => {
          // Restore controller visibility before the versioned cast so an
          // unsupported Resume interface cannot leave a shown window blank.
          controller.SetIsVisible(true)?;
          core_webview.cast::<ICoreWebView2_3>()?.Resume()
        }
        true => {
          // Hiding the native Tauri window does not change the WebView2
          // controller's visibility. TrySuspend requires this explicit step.
          controller.SetIsVisible(false)?;
          let handler =
            TrySuspendCompletedHandler::create(Box::new(move |result, success| {
              match result {
                Ok(()) if success => log_debug!(
                  &format!("WebView2 suspended {callback_label}"),
                  "webview_memory::set_suspended",
                  None::<&str>
                ),
                Ok(()) => log_warn!(
                  &format!("WebView2 declined suspension for {callback_label}"),
                  "webview_memory::set_suspended",
                  None::<&str>
                ),
                Err(error) => log_warn!(
                  &format!(
                    "WebView2 suspend callback failed for {callback_label}: {error}"
                  ),
                  "webview_memory::set_suspended",
                  None::<&str>
                ),
              }
              Ok(())
            }));
          core_webview.cast::<ICoreWebView2_3>()?.TrySuspend(&handler)
        }
      }
    })();

    if let Err(error) = operation {
      log_warn!(
        &format!("failed to change WebView2 suspension for {operation_label}: {error}"),
        "webview_memory::set_suspended",
        None::<&str>
      );
    }
  });

  if let Err(error) = dispatch_result {
    log_warn!(
      &format!("failed to access WebView2 for {label}: {error}"),
      "webview_memory::set_suspended",
      None::<&str>
    );
  }
}

#[cfg(not(target_os = "windows"))]
fn set_suspended(_window: &WebviewWindow, _suspended: bool) {}
