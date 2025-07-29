// Platform factory implementation
use crate::platform::traits::Platform;

/// プラットフォーム判定エラー
#[derive(Debug, Clone)]
pub enum PlatformError {
  UnsupportedPlatform(String),
  InitializationFailed(String),
}

impl std::fmt::Display for PlatformError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PlatformError::UnsupportedPlatform(platform) => {
        write!(f, "Unsupported platform: {}", platform)
      }
      PlatformError::InitializationFailed(reason) => {
        write!(f, "Platform initialization failed: {}", reason)
      }
    }
  }
}

impl std::error::Error for PlatformError {}

/// プラットフォームの種類を表す enum
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformType {
  Windows,
  Linux,
  MacOS,
}

impl PlatformType {
  /// 現在の実行環境のプラットフォームを判定
  pub fn detect() -> PlatformType {
    #[cfg(target_os = "windows")]
    return PlatformType::Windows;

    #[cfg(target_os = "linux")]
    return PlatformType::Linux;

    #[cfg(target_os = "macos")]
    return PlatformType::MacOS;
  }

  /// プラットフォーム名を文字列で取得
  pub fn name(&self) -> &'static str {
    match self {
      PlatformType::Windows => "Windows",
      PlatformType::Linux => "Linux",
      PlatformType::MacOS => "macOS",
    }
  }
}

/// Platform インスタンスを生成する Factory
/// 責務: プラットフォーム検出とインスタンス生成のみ
pub struct PlatformFactory;

impl PlatformFactory {
    /// 現在のプラットフォームに適した Platform trait object を作成
  pub fn create_platform() -> Result<Box<dyn crate::platform::traits::Platform>, PlatformError> {
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

  /// 指定されたプラットフォーム用の Platform trait object を作成
  /// テスト用の関数（通常は create_platform を使用）
  pub fn create_platform_for_type(
    platform: PlatformType,
  ) -> Result<Box<dyn Platform>, PlatformError> {
    match platform {
      PlatformType::Windows => {
        #[cfg(target_os = "windows")]
        {
          let platform = crate::platform::windows::WindowsPlatform::new()
            .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
          Ok(Box::new(platform))
        }

        #[cfg(not(target_os = "windows"))]
        Err(PlatformError::UnsupportedPlatform(
          "Windows platform not available on this system".to_string(),
        ))
      }

      PlatformType::Linux => {
        #[cfg(target_os = "linux")]
        {
          let platform = crate::platform::linux::LinuxPlatform::new()
            .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
          Ok(Box::new(platform))
        }

        #[cfg(not(target_os = "linux"))]
        Err(PlatformError::UnsupportedPlatform(
          "Linux platform not available on this system".to_string(),
        ))
      }

      PlatformType::MacOS => {
        #[cfg(target_os = "macos")]
        {
          let platform = crate::platform::macos::MacOSPlatform::new()
            .map_err(|e| PlatformError::InitializationFailed(e.to_string()))?;
          Ok(Box::new(platform))
        }

        #[cfg(not(target_os = "macos"))]
        Err(PlatformError::UnsupportedPlatform(
          "macOS platform not available on this system".to_string(),
        ))
      }
    }
  }

  /// 利用可能なプラットフォームの一覧を取得
  pub fn available_platforms() -> Vec<PlatformType> {
    let mut platforms = Vec::new();

    #[cfg(target_os = "windows")]
    platforms.push(PlatformType::Windows);

    #[cfg(target_os = "linux")]
    platforms.push(PlatformType::Linux);

    #[cfg(target_os = "macos")]
    platforms.push(PlatformType::MacOS);

    platforms
  }

  /// 現在のプラットフォームタイプを取得
  pub fn current_platform_type() -> PlatformType {
    PlatformType::detect()
  }

  /// プラットフォームがサポートされているかチェック
  pub fn is_platform_supported(platform: &PlatformType) -> bool {
    Self::available_platforms().contains(platform)
  }
}
