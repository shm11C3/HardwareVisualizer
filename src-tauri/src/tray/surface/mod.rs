use crate::tray::widget::TrayFrame;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod tauri_surface;

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

  #[cfg(not(target_os = "macos"))]
  {
    tauri_surface::TauriTraySurface::new(app)
      .map(|surface| Box::new(surface) as Box<dyn TraySurface>)
  }
}
