use super::traits::PlatformServices;

#[cfg(target_os = "windows")]
use super::windows::WindowsPlatform;

#[cfg(target_os = "linux")]
use super::linux::LinuxPlatform;

#[cfg(target_os = "macos")]
use super::macos::MacOSPlatform;

pub struct PlatformFactory;

impl PlatformFactory {
  pub fn create() -> Box<dyn PlatformServices> {
    #[cfg(target_os = "windows")]
    return Box::new(WindowsPlatform::new());

    #[cfg(target_os = "linux")]
    return Box::new(LinuxPlatform::new());

    #[cfg(target_os = "macos")]
    return Box::new(MacOSPlatform::new());

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    panic!("Unsupported platform");
  }

  #[allow(dead_code)]
  pub fn get_platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "Windows";

    #[cfg(target_os = "linux")]
    return "Linux";

    #[cfg(target_os = "macos")]
    return "macOS";

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return "Unknown";
  }
}
