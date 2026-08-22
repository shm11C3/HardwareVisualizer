use crate::enums::error::PlatformError;

/// Factory that creates Platform instances
pub struct PlatformFactory;

impl PlatformFactory {
  /// Create a Platform trait object suitable for the current platform
  pub fn create() -> Result<Box<dyn crate::platform::traits::Platform>, PlatformError> {
    Self::create_platform()
  }

  /// Create a Platform trait object suitable for the current platform
  pub fn create_platform()
  -> Result<Box<dyn crate::platform::traits::Platform>, PlatformError> {
    #[cfg(target_os = "windows")]
    {
      Ok(Box::new(crate::platform::windows::WindowsPlatform::new()?))
    }

    #[cfg(target_os = "linux")]
    {
      Ok(Box::new(crate::platform::linux::LinuxPlatform::new()?))
    }

    #[cfg(target_os = "macos")]
    {
      Ok(Box::new(crate::platform::macos::MacOSPlatform::new()?))
    }
  }
}
