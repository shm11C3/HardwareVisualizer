//! Drives the launched app's main window into the background.
//!
//! The background phase measures the app the way a user leaves it after
//! closing the main window to the tray. Sending the close request to the
//! real window, rather than starting the app hidden, exercises the same
//! close handler, window hide, and WebView suspension a user triggers. The
//! monitor launches the app with `HARDVIZ_CLOSE_TO_BACKGROUND=1`, so the
//! close hides the window regardless of the runner's persisted preference.
//!
//! Only Windows is implemented: it is the only platform the performance
//! workflow runs on, and the only one where the app suspends hidden WebViews.

/// The app's test override that turns a window close into close-to-tray.
pub const CLOSE_TO_BACKGROUND_ENV: &str = "HARDVIZ_CLOSE_TO_BACKGROUND";

#[cfg(target_os = "windows")]
pub use windows_impl::{has_visible_window, request_close};

#[cfg(target_os = "windows")]
mod windows_impl {
  use windows_sys::Win32::Foundation::{HWND, LPARAM};
  use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE,
  };
  use windows_sys::core::BOOL;

  struct WindowSearch {
    pid: u32,
    windows: Vec<HWND>,
  }

  /// Posts `WM_CLOSE` to every visible top-level window `pid` owns. Hidden
  /// windows, such as the tray flyout, are not touched.
  pub fn request_close(pid: u32) -> Result<(), String> {
    let windows = visible_windows(pid);
    if windows.is_empty() {
      return Err(format!("no visible window found for PID {pid}"));
    }

    for hwnd in windows {
      // SAFETY: `hwnd` came from `EnumWindows`; posting to a window that was
      // destroyed since then fails without side effects.
      if unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) } == 0 {
        return Err(format!(
          "failed to post WM_CLOSE to a window of PID {pid}: {}",
          std::io::Error::last_os_error()
        ));
      }
    }

    Ok(())
  }

  pub fn has_visible_window(pid: u32) -> bool {
    !visible_windows(pid).is_empty()
  }

  fn visible_windows(pid: u32) -> Vec<HWND> {
    let mut search = WindowSearch {
      pid,
      windows: Vec::new(),
    };
    // SAFETY: the callback only runs during this call and receives a pointer
    // to `search`, which outlives it. A failed enumeration leaves the windows
    // found so far, which callers treat the same as a fresh empty result.
    unsafe {
      EnumWindows(
        Some(collect_visible_window),
        &mut search as *mut WindowSearch as LPARAM,
      );
    }
    search.windows
  }

  unsafe extern "system" fn collect_visible_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `visible_windows` passes a live `WindowSearch` as `lparam`.
    let search = unsafe { &mut *(lparam as *mut WindowSearch) };
    let mut owner_pid = 0;
    // SAFETY: `hwnd` is supplied by `EnumWindows` for the callback's duration.
    unsafe { GetWindowThreadProcessId(hwnd, &mut owner_pid) };
    if owner_pid == search.pid && unsafe { IsWindowVisible(hwnd) } != 0 {
      search.windows.push(hwnd);
    }
    1
  }
}
