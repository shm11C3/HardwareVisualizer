#[cfg(test)]
mod tests {
  use crate::platform::factory::{PlatformError, PlatformFactory};

  #[test]
  fn test_create_platform() {
    let result = PlatformFactory::create();
    assert!(result.is_ok(), "Platform creation should succeed");
  }

  #[test]
  fn test_create_platform_method() {
    let result = PlatformFactory::create_platform();
    assert!(result.is_ok(), "Platform creation should succeed");
  }

  #[test]
  fn test_platform_error_display() {
    let error = PlatformError::InitializationFailed("Test error".to_string());
    let display_string = format!("{}", error);
    assert!(display_string.contains("Platform initialization failed"));
    assert!(display_string.contains("Test error"));
  }

  #[test]
  fn test_platform_error_debug() {
    let error = PlatformError::InitializationFailed("Debug test".to_string());
    let debug_string = format!("{:?}", error);
    assert!(debug_string.contains("InitializationFailed"));
    assert!(debug_string.contains("Debug test"));
  }

  #[test]
  fn test_platform_error_clone() {
    let original = PlatformError::InitializationFailed("Clone test".to_string());
    let cloned = original.clone();

    assert!(format!("{}", original) == format!("{}", cloned));
  }

  #[test]
  fn test_platform_error_std_error() {
    let error = PlatformError::InitializationFailed("Error test".to_string());
    let _error_trait: &dyn std::error::Error = &error;
  }
}
