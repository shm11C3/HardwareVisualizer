use tauri::{
  AppHandle,
  menu::{Menu, MenuEvent, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

use crate::log_warn;
use crate::tray::widget::TrayFrame;

use super::TraySurface;

const MENU_OPEN_ID: &str = "tray::open";
const MENU_QUIT_ID: &str = "tray::quit";

pub struct TauriTraySurface {
  _tray: TrayIcon,
}

impl TauriTraySurface {
  pub fn new(app: &tauri::App) -> tauri::Result<Self> {
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
      // Keep the "left click restores, right click opens the menu"
      // contract on backends that forward raw tray click events.
      .show_menu_on_left_click(false)
      .on_menu_event(handle_menu_event)
      .on_tray_icon_event(handle_tray_event)
      .build(app)?;

    Ok(Self { _tray: tray })
  }
}

impl TraySurface for TauriTraySurface {
  fn apply_frame(&self, frame: &TrayFrame) {
    // `None` is not a reliable clear operation on macOS in tray-icon
    // 0.21, so use an empty title for disabled / unavailable frames.
    if let Err(e) = self
      ._tray
      .set_title(Some(frame.title.as_deref().unwrap_or("")))
    {
      log_warn!(
        &format!("failed to update tray widget title: {e}"),
        "tray::surface::tauri::apply_frame",
        None::<&str>
      );
    }

    if let Err(e) = self._tray.set_tooltip(Some(frame.tooltip.as_str())) {
      log_warn!(
        &format!("failed to update tray widget tooltip: {e}"),
        "tray::surface::tauri::apply_frame",
        None::<&str>
      );
    }
  }

  fn close(&self) {}
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
  match event.id.as_ref() {
    MENU_OPEN_ID => crate::lifecycle::restore_main_window(app),
    MENU_QUIT_ID => crate::adapters::tray::spawn_quit(app.clone()),
    other => {
      log_warn!(
        &format!("unhandled tray menu event: {other}"),
        "tray::surface::tauri::handle_menu_event",
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
    crate::lifecycle::restore_main_window(tray.app_handle());
  }
}
