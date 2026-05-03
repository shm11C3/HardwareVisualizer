use crate::tray::widget::TrayFrame;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod tauri_surface;
#[cfg(target_os = "windows")]
mod windows;

pub trait TraySurface: Send {
  fn apply_frame(&self, frame: &TrayFrame);
  fn close(&self);
}

pub fn create(app: &tauri::App) -> tauri::Result<Box<dyn TraySurface>> {
  #[cfg(target_os = "macos")]
  {
    macos::MacosTraySurface::new(app.handle().clone())
      .map(|surface| Box::new(surface) as Box<dyn TraySurface>)
  }

  #[cfg(target_os = "windows")]
  {
    windows::WindowsTraySurface::new(app)
      .map(|surface| Box::new(surface) as Box<dyn TraySurface>)
  }

  #[cfg(not(any(target_os = "macos", target_os = "windows")))]
  {
    tauri_surface::TauriTraySurface::new(app)
      .map(|surface| Box::new(surface) as Box<dyn TraySurface>)
  }
}
