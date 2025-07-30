#[cfg(test)]
mod tests {
  use crate::platform::{PlatformError, PlatformFactory, PlatformType};

  #[test]
  fn test_platform_type_detect() {
    let detected_platform = PlatformType::detect();

    #[cfg(target_os = "windows")]
    assert_eq!(detected_platform, PlatformType::Windows);

    #[cfg(target_os = "linux")]
    assert_eq!(detected_platform, PlatformType::Linux);

    #[cfg(target_os = "macos")]
    assert_eq!(detected_platform, PlatformType::MacOS);
  }

  #[test]
  fn test_platform_type_name() {
    assert_eq!(PlatformType::Windows.name(), "Windows");
    assert_eq!(PlatformType::Linux.name(), "Linux");
    assert_eq!(PlatformType::MacOS.name(), "macOS");
  }

  #[test]
  fn test_current_platform_type() {
    let current = PlatformFactory::current_platform_type();
    let detected = PlatformType::detect();
    assert_eq!(current, detected);
  }

  #[test]
  fn test_available_platforms() {
    let platforms = PlatformFactory::available_platforms();
    assert!(!platforms.is_empty());

    // 現在のプラットフォームが利用可能リストに含まれているか確認
    let current = PlatformType::detect();
    assert!(platforms.contains(&current));
  }

  #[test]
  fn test_is_platform_supported() {
    let current = PlatformType::detect();
    assert!(PlatformFactory::is_platform_supported(&current));

    // 他のプラットフォームの対応状況をテスト
    #[cfg(target_os = "windows")]
    {
      assert!(PlatformFactory::is_platform_supported(
        &PlatformType::Windows
      ));
      assert!(!PlatformFactory::is_platform_supported(
        &PlatformType::Linux
      ));
      assert!(!PlatformFactory::is_platform_supported(
        &PlatformType::MacOS
      ));
    }

    #[cfg(target_os = "linux")]
    {
      assert!(!PlatformFactory::is_platform_supported(
        &PlatformType::Windows
      ));
      assert!(PlatformFactory::is_platform_supported(&PlatformType::Linux));
      assert!(!PlatformFactory::is_platform_supported(
        &PlatformType::MacOS
      ));
    }

    #[cfg(target_os = "macos")]
    {
      assert!(!PlatformFactory::is_platform_supported(
        &PlatformType::Windows
      ));
      assert!(!PlatformFactory::is_platform_supported(
        &PlatformType::Linux
      ));
      assert!(PlatformFactory::is_platform_supported(&PlatformType::MacOS));
    }
  }

  #[test]
  fn test_create_for_unsupported_platform() {
    // 現在のプラットフォーム以外を指定してエラーになることを確認
    #[cfg(target_os = "windows")]
    {
      let result = PlatformFactory::create_platform_for_type(PlatformType::Linux);
      assert!(result.is_err());
      match result {
        Err(PlatformError::UnsupportedPlatform(_)) => {
          // Expected error type
        }
        _ => panic!("Expected UnsupportedPlatform error"),
      }
    }

    #[cfg(target_os = "linux")]
    {
      let result = PlatformFactory::create_platform_for_type(PlatformType::Windows);
      assert!(result.is_err());
      match result {
        Err(PlatformError::UnsupportedPlatform(_)) => {
          // Expected error type
        }
        _ => panic!("Expected UnsupportedPlatform error"),
      }
    }
  }

  #[test]
  fn test_create_for_current_platform() {
    // 現在のプラットフォーム用のインスタンス作成は実装未完了のためエラーになる
    let current = PlatformType::detect();
    let result = PlatformFactory::create_platform_for_type(current);
    assert!(result.is_err());
    match result {
      Err(PlatformError::InitializationFailed(_)) => {
        // Expected error type - implementation not yet available
      }
      _ => panic!("Expected InitializationFailed error"),
    }
  }

  #[test]
  fn test_platform_error_display() {
    let unsupported_error = PlatformError::UnsupportedPlatform("TestOS".to_string());
    assert_eq!(
      unsupported_error.to_string(),
      "Unsupported platform: TestOS"
    );

    let init_error = PlatformError::InitializationFailed("Test reason".to_string());
    assert_eq!(
      init_error.to_string(),
      "Platform initialization failed: Test reason"
    );
  }

  #[test]
  fn test_platform_error_is_error() {
    let error = PlatformError::UnsupportedPlatform("test".to_string());
    // std::error::Error trait が実装されていることを確認
    assert!(std::error::Error::source(&error).is_none());
  }
}
