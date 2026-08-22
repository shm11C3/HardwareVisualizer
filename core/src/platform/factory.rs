use std::sync::{Arc, OnceLock};

use crate::enums::error::PlatformError;
use crate::platform::traits::Platform;

/// Outcome of resolving the shared platform once: the process-wide
/// instance, or the initialization error that consumers report on each
/// use (for example as per-cycle unavailable samples).
pub type PlatformHandle = Result<Arc<dyn Platform>, PlatformError>;

static SHARED: OnceLock<Arc<dyn Platform>> = OnceLock::new();

/// Factory that resolves the process-wide Platform instance
pub struct PlatformFactory;

impl PlatformFactory {
  /// Return the shared Platform for the current OS, constructing it on
  /// first use.
  ///
  /// The instance lives for the rest of the process so platform
  /// implementations can hold long-lived state (probed capabilities,
  /// provider handles). A construction failure is not cached; later
  /// calls retry.
  pub fn shared() -> PlatformHandle {
    if let Some(platform) = SHARED.get() {
      return Ok(Arc::clone(platform));
    }
    let platform: Arc<dyn Platform> = Arc::from(Self::create_platform()?);
    // On a construction race the first stored instance wins; platforms
    // hold no exclusive resources at construction time.
    Ok(Arc::clone(SHARED.get_or_init(|| platform)))
  }

  fn create_platform() -> Result<Box<dyn Platform>, PlatformError> {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn shared_returns_the_same_instance() {
    let first = PlatformFactory::shared().expect("platform must resolve");
    let second = PlatformFactory::shared().expect("platform must resolve");

    assert!(Arc::ptr_eq(&first, &second));
  }
}
