//! System-tray menu adapter — Phase 6 skeleton (#1409).
//!
//! The tray gives users a way back to the main window once Phase 5
//! stops auto-quitting on close. This phase ships the bare minimum:
//! `Open` and `Quit` menu items plus a left-click that restores the
//! window. Live metric rendering belongs to #1401, and the Pause /
//! Resume entry belongs to #1275.
//!
//! Setup is gated by [`crate::lifecycle::should_close_to_background`]
//! so the user-visible behavior stays unchanged from Phase 5: in
//! release builds without `HARDVIZ_CLOSE_TO_BACKGROUND=1`, no tray is
//! registered. The flag flip — and the persisted setting that owns it
//! — ships with #1275.
//!
//! ## Platform notes
//! - **Windows / macOS**: left-clicking the icon restores the window;
//!   right-click opens the menu.
//! - **Linux**: tray support depends on the desktop environment.
//!   AppIndicator-based environments (GNOME with the AppIndicator
//!   extension, KDE) usually show the menu on any click and never
//!   forward a raw click event, so the left-click-restores path may be
//!   silently inactive. Users still reach `Open` through the menu, and
//!   `Quit` through the menu always works. Environments without an
//!   indicator implementation may fail to register the tray entirely;
//!   when that happens [`TrayAdapter::setup`] logs and returns the
//!   error to the caller, who falls back to running without a tray.

use tauri::{
  AppHandle, Manager,
  menu::{Menu, MenuEvent, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

use crate::log_warn;

const MENU_OPEN_ID: &str = "tray::open";
const MENU_QUIT_ID: &str = "tray::quit";

/// Owns the [`TrayIcon`] handle. Dropping the adapter removes the
/// icon, so it must live for as long as the tray should be visible —
/// `lib.rs` stashes it in `WorkersState`.
pub struct TrayAdapter {
  _tray: TrayIcon,
}

impl TrayAdapter {
  /// Build the tray icon, menu, and event handlers.
  ///
  /// The default window icon is reused as the tray icon — Phase 6
  /// keeps the visual minimal. A dedicated tray-tuned asset can land
  /// alongside #1401's live widget.
  pub fn setup(app: &tauri::App) -> tauri::Result<Self> {
    let open_item = MenuItem::with_id(app, MENU_OPEN_ID, "Open", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let icon = app
      .default_window_icon()
      .cloned()
      .ok_or_else(|| tauri::Error::AssetNotFound("default_window_icon".into()))?;

    let tray = TrayIconBuilder::new()
      .icon(icon)
      .menu(&menu)
      // Without this, macOS pops the menu on left-click instead of
      // forwarding the click event we use to restore the window.
      // Windows defaults to false; setting it explicitly keeps the
      // behavior consistent across platforms.
      .show_menu_on_left_click(false)
      .on_menu_event(handle_menu_event)
      .on_tray_icon_event(handle_tray_event)
      .build(app)?;

    Ok(Self { _tray: tray })
  }
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
  match event.id.as_ref() {
    MENU_OPEN_ID => restore_main_window(app),
    MENU_QUIT_ID => spawn_quit(app.clone()),
    other => {
      log_warn!(
        &format!("unhandled tray menu event: {other}"),
        "adapters::tray::handle_menu_event",
        None::<&str>
      );
    }
  }
}

fn handle_tray_event(tray: &TrayIcon, event: TrayIconEvent) {
  // Restore on a *released* left click. Filtering on `Up` matches the
  // intuitive "click to focus" gesture and avoids firing twice on
  // platforms that emit both press and release events.
  if let TrayIconEvent::Click {
    button: MouseButton::Left,
    button_state: MouseButtonState::Up,
    ..
  } = event
  {
    restore_main_window(tray.app_handle());
  }
}

fn restore_main_window(app: &AppHandle) {
  let Some(window) = app.get_webview_window("main") else {
    log_warn!(
      "main window not found while handling tray Open",
      "adapters::tray::restore_main_window",
      None::<&str>
    );
    return;
  };

  // Best-effort: each call may legitimately fail (window already
  // visible, already in the foreground, OS denying focus) without
  // affecting the others. Logging keeps the trail without aborting.
  if let Err(e) = window.show() {
    log_warn!(
      &format!("failed to show main window from tray: {e}"),
      "adapters::tray::restore_main_window",
      None::<&str>
    );
  }
  if let Err(e) = window.unminimize() {
    log_warn!(
      &format!("failed to unminimize main window from tray: {e}"),
      "adapters::tray::restore_main_window",
      None::<&str>
    );
  }
  if let Err(e) = window.set_focus() {
    log_warn!(
      &format!("failed to focus main window from tray: {e}"),
      "adapters::tray::restore_main_window",
      None::<&str>
    );
  }
}

fn spawn_quit(app: AppHandle) {
  // `request_quit` is async (it awaits worker termination); the menu
  // callback runs on the main thread and must return promptly, so we
  // hand the work off to the tokio runtime.
  tauri::async_runtime::spawn(async move {
    crate::lifecycle::request_quit(app).await;
  });
}
