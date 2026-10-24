//! Native main-window appearance follows the existing Transparent UI preference.

#[cfg(target_os = "macos")]
pub fn apply(app: &tauri::AppHandle) {
  let app_handle = app.clone();
  if let Err(error) = app.run_on_main_thread(move || {
    if let Err(error) = apply_on_main_thread(&app_handle) {
      crate::log_error!(
        "Failed to update native window appearance",
        "window_appearance_service::apply",
        Some(error)
      );
    }
  }) {
    crate::log_error!(
      "Failed to dispatch native window appearance update",
      "window_appearance_service::apply",
      Some(error.to_string())
    );
  }
}

#[cfg(target_os = "macos")]
fn apply_on_main_thread(app: &tauri::AppHandle) -> Result<(), String> {
  use crate::commands::settings::AppState;
  use objc2_app_kit::{NSColor, NSWindow};
  use std::ptr::NonNull;
  use tauri::Manager;
  use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy, clear_vibrancy};

  // Read the latest saved values on the main thread so queued updates cannot
  // reapply an older transparency/blur snapshot after a newer preference.
  let (transparent_ui, glass_blur) = {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap();
    (settings.transparent_ui, settings.glass_blur)
  };
  let window = app
    .get_webview_window("main")
    .ok_or("Main window not found")?;

  // apply_vibrancy adds an effect view rather than replacing one. Remove all
  // existing views before changing material or making the window opaque.
  while clear_vibrancy(&window).map_err(|error| error.to_string())? {}

  let ns_window = NonNull::new(
    window
      .ns_window()
      .map_err(|error| error.to_string())?
      .cast::<NSWindow>(),
  )
  .ok_or("Native main window not found")?;
  // SAFETY: Tauri supplies the live NSWindow owned by `window`. This borrow
  // stays inside the main-thread callback and is not retained after it returns.
  let ns_window = unsafe { ns_window.as_ref() };

  // Clearing vibrancy alone leaves the configured NSWindow transparent. Opaque
  // UI needs an opaque backing too; the frontend paints its theme/image above it.
  // Keep the WebView transparent so enabling Transparent UI is reversible.
  let background = if transparent_ui {
    NSColor::clearColor()
  } else {
    NSColor::blackColor()
  };
  ns_window.setBackgroundColor(Some(&background));
  ns_window.setOpaque(!transparent_ui);

  // A zero blur level still means a transparent window when Transparent UI is
  // enabled; blur intensity must not decide native opacity.
  if transparent_ui {
    let material = match glass_blur {
      0 => return Ok(()),
      1..=10 => NSVisualEffectMaterial::HudWindow,
      11..=20 => NSVisualEffectMaterial::Menu,
      _ => NSVisualEffectMaterial::UnderWindowBackground,
    };
    apply_vibrancy(&window, material, None, None).map_err(|error| error.to_string())?;
  }
  Ok(())
}

/// Native vibrancy/opacity management is only needed for macOS.
#[cfg(not(target_os = "macos"))]
pub fn apply(_app: &tauri::AppHandle) {}
