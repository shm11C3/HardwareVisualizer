/// Platform detection error
#[derive(Debug, Clone)]
pub enum PlatformError {
  InitializationFailed(String),
}

impl std::fmt::Display for PlatformError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PlatformError::InitializationFailed(reason) => {
        write!(f, "Platform initialization failed: {reason}")
      }
    }
  }
}

impl std::error::Error for PlatformError {}

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
      let platform = crate::platform::windows::WindowsPlatform::new()
        .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
      Ok(Box::new(platform))
    }

    #[cfg(target_os = "linux")]
    {
      let platform = crate::platform::linux::LinuxPlatform::new()
        .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
      Ok(Box::new(platform))
    }

    #[cfg(target_os = "macos")]
    {
      let platform = crate::platform::macos::MacOSPlatform::new()
        .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
      Ok(Box::new(platform))
    }
  }
}
