//! Windows single-instance activation across the medium/high integrity boundary.
//!
//! The Tauri single-instance plugin remains the cross-platform owner of its
//! existing behavior. This earlier App-side rendezvous lets a Windows
//! secondary launch restore a running elevated process before Tauri or the
//! database starts. The event carries one action only: Open the existing
//! window.

use std::{
  ffi::c_void,
  os::windows::{
    ffi::OsStrExt,
    io::{AsRawHandle, FromRawHandle, OwnedHandle},
  },
  thread,
};

use windows::{
  Win32::{
    Foundation::{
      ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS,
      GetLastError, HANDLE, HLOCAL, LPARAM, LocalFree, SetLastError, WAIT_FAILED,
      WAIT_OBJECT_0, WPARAM,
    },
    Security::{
      Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        SDDL_REVISION_1,
      },
      GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
      TOKEN_USER, TokenUser,
    },
    System::{
      DataExchange::COPYDATASTRUCT,
      Threading::{
        CREATE_EVENT, CreateEventExW, EVENT_MODIFY_STATE, GetCurrentProcess, OpenEventW,
        OpenMutexW, OpenProcessToken, SYNCHRONIZATION_SYNCHRONIZE, SetEvent,
        WaitForSingleObject,
      },
    },
    UI::WindowsAndMessaging::{
      AllowSetForegroundWindow, FindWindowW, GetWindowThreadProcessId, MB_ICONWARNING,
      MB_OK, MB_SETFOREGROUND, MessageBoxW, SMTO_ABORTIFHUNG, SMTO_BLOCK,
      SendMessageTimeoutW, WM_COPYDATA,
    },
  },
  core::{PCWSTR, PWSTR},
};

pub(crate) const ACTIVATION_FAILED_EXIT_CODE: i32 = 4;

const LEGACY_SINGLE_INSTANCE_DATA: usize = 1542;
const LEGACY_SEND_TIMEOUT_MS: u32 = 5_000;
const ACTIVATION_EVENT_ACCESS: u32 = SYNCHRONIZATION_SYNCHRONIZE.0 | EVENT_MODIFY_STATE.0;
const LEGACY_ACTIVATION_PAYLOAD: &[u8] = b"|\0";

pub(crate) enum StartupInstance {
  Primary(ActivationEvent),
  Forwarded,
}

pub(crate) struct ActivationEvent {
  handle: OwnedHandle,
}

impl ActivationEvent {
  fn raw(&self) -> HANDLE {
    HANDLE(self.handle.as_raw_handle())
  }
}

/// Either claim the new activation event or notify an existing process and
/// stop before App settings and database startup.
pub(crate) fn claim_or_forward(identifier: &str) -> Result<StartupInstance, String> {
  let user_sid = current_user_sid()?;
  let event_name = activation_event_name(identifier, &user_sid);
  let event_name_wide = wide_null(&event_name);

  match unsafe { OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(event_name_wide.as_ptr())) }
  {
    Ok(handle) => {
      if let Some(window) = plugin_instance_window(identifier) {
        allow_foreground_for_window(window);
      }
      signal_and_close(handle, "the existing activation event")?;
      return Ok(StartupInstance::Forwarded);
    }
    Err(error) if is_win32_error(&error, ERROR_FILE_NOT_FOUND) => {}
    Err(error) => {
      return Err(format!(
        "The existing activation event could not be opened ({error:?}). \
         Open HardwareVisualizer from its tray icon, or quit that instance \
         before launching again."
      ));
    }
  }

  let (event, already_exists) = create_activation_event(&event_name, &user_sid)?;

  if already_exists {
    if let Some(window) = plugin_instance_window(identifier) {
      allow_foreground_for_window(window);
    }
    signal_event(event.raw(), "the existing activation event")?;
    return Ok(StartupInstance::Forwarded);
  }

  // Older versions do not have the event listener. Keep their existing
  // plugin rendezvous as a compatibility path, but only after this process
  // has atomically claimed a new event. The plugin's own mutex is not created
  // until Tauri setup, so this probe cannot mistake this process for an older
  // resident.
  if let Some(window) = legacy_instance_window(identifier)? {
    if let Err(error) = send_legacy_activation(window) {
      return Err(format!(
        "A previous HardwareVisualizer instance is running, but Windows did \
         not confirm its activation ({error}). Open that instance from its \
         tray icon, or quit it before launching again. This launch stopped \
         before database startup to avoid a second database owner."
      ));
    }
    return Ok(StartupInstance::Forwarded);
  }

  Ok(StartupInstance::Primary(event))
}

/// Start the background event waiter after Tauri has created the main window.
/// A signal sent during early startup remains pending on the auto-reset event.
pub(crate) fn start_receiver(
  event: ActivationEvent,
  app: tauri::AppHandle,
) -> Result<(), String> {
  thread::Builder::new()
    .name("windows-activation-event".to_string())
    .spawn(move || {
      loop {
        let wait_result = unsafe { WaitForSingleObject(event.raw(), u32::MAX) };
        if wait_result == WAIT_OBJECT_0 {
          let restore_app = app.clone();
          if let Err(error) = app.run_on_main_thread(move || {
            crate::lifecycle::restore_main_window(&restore_app);
          }) {
            crate::log_warn!(
              &format!("failed to dispatch activation to the main thread: {error}"),
              "windows_activation::start_receiver",
              None::<&str>
            );
          }
        } else {
          let error = if wait_result == WAIT_FAILED {
            format!("Win32 error {:?}", unsafe { GetLastError() })
          } else {
            format!("unexpected wait result {wait_result:?}")
          };
          crate::log_warn!(
            &format!("activation event listener stopped: {error}"),
            "windows_activation::start_receiver",
            None::<&str>
          );
          break;
        }
      }
    })
    .map_err(|error| format!("Could not start the activation event listener: {error}"))?;

  Ok(())
}

pub(crate) fn report_startup_error(message: &str) {
  eprintln!("HardwareVisualizer could not handle this launch: {message}");

  let body = wide_null(message);
  let title = wide_null("HardwareVisualizer");
  unsafe {
    MessageBoxW(
      None,
      PCWSTR(body.as_ptr()),
      PCWSTR(title.as_ptr()),
      MB_OK | MB_ICONWARNING | MB_SETFOREGROUND,
    );
  }
}

fn current_user_sid() -> Result<String, String> {
  let mut token = HANDLE::default();
  unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
    .map_err(|error| format!("Could not read the current Windows user: {error:?}"))?;
  let token = owned_handle(token);

  let mut required = 0u32;
  let _ = unsafe {
    GetTokenInformation(token_handle(&token), TokenUser, None, 0, &mut required)
  };
  if required == 0 {
    return Err("Windows returned an empty user-token record".to_string());
  }

  let words = required.div_ceil(std::mem::size_of::<usize>() as u32) as usize;
  let mut buffer = vec![0usize; words];
  unsafe {
    GetTokenInformation(
      token_handle(&token),
      TokenUser,
      Some(buffer.as_mut_ptr().cast()),
      (buffer.len() * std::mem::size_of::<usize>()) as u32,
      &mut required,
    )
  }
  .map_err(|error| format!("Could not read the current Windows user SID: {error:?}"))?;

  let token_user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
  let mut sid_string = PWSTR::null();
  unsafe { ConvertSidToStringSidW(token_user.User.Sid, &mut sid_string) }.map_err(
    |error| format!("Could not format the current Windows user SID: {error:?}"),
  )?;

  let sid_pointer = sid_string.0;
  let mut length = 0usize;
  unsafe {
    while *sid_pointer.add(length) != 0 {
      length += 1;
    }
  }
  let sid =
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(sid_pointer, length) });
  let _ = unsafe { LocalFree(Some(HLOCAL(sid_pointer as *mut c_void))) };
  Ok(sid)
}

fn create_activation_event(
  event_name: &str,
  user_sid: &str,
) -> Result<(ActivationEvent, bool), String> {
  let sddl = activation_security_descriptor(user_sid);
  let sddl_wide = wide_null(&sddl);
  let mut descriptor = PSECURITY_DESCRIPTOR::default();
  unsafe {
    ConvertStringSecurityDescriptorToSecurityDescriptorW(
      PCWSTR(sddl_wide.as_ptr()),
      SDDL_REVISION_1,
      &mut descriptor,
      None,
    )
  }
  .map_err(|error| {
    format!("Could not create activation-event security rules: {error:?}")
  })?;
  let descriptor = LocalSecurityDescriptor(descriptor);
  let attributes = SECURITY_ATTRIBUTES {
    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
    lpSecurityDescriptor: descriptor.0.0,
    bInheritHandle: false.into(),
  };
  let event_name_wide = wide_null(event_name);

  // CreateEventExW reports ERROR_ALREADY_EXISTS through thread last-error even
  // though it successfully opens the preexisting event. Clear it so the result
  // cannot be confused with an unrelated earlier Win32 call.
  unsafe { SetLastError(ERROR_SUCCESS) };
  let event = unsafe {
    CreateEventExW(
      Some(&attributes),
      PCWSTR(event_name_wide.as_ptr()),
      CREATE_EVENT(0),
      ACTIVATION_EVENT_ACCESS,
    )
  }
  .map_err(|error| {
    format!(
      "Could not create or open the activation event ({error:?}). \
       Open HardwareVisualizer from its tray icon, or quit that instance \
       before launching again."
    )
  })?;
  let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
  let event = ActivationEvent {
    handle: unsafe { OwnedHandle::from_raw_handle(event.0) },
  };

  Ok((event, already_exists))
}

fn plugin_instance_window(identifier: &str) -> Option<windows::Win32::Foundation::HWND> {
  let class_name = wide_null(&format!("{identifier}-sic"));
  let window_name = wide_null(&format!("{identifier}-siw"));
  unsafe { FindWindowW(PCWSTR(class_name.as_ptr()), PCWSTR(window_name.as_ptr())) }.ok()
}

fn allow_foreground_for_window(window: windows::Win32::Foundation::HWND) {
  let mut process_id = 0;
  unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
  if process_id != 0
    && let Err(error) = unsafe { AllowSetForegroundWindow(process_id) }
  {
    crate::log_warn!(
      &format!(
        "could not grant foreground permission to the resident process: {error:?}"
      ),
      "windows_activation::allow_foreground_for_window",
      None::<&str>
    );
  }
}

fn legacy_instance_window(
  identifier: &str,
) -> Result<Option<windows::Win32::Foundation::HWND>, String> {
  let mutex_name = wide_null(&format!(r"Local\{identifier}-sim"));
  let mut mutex_present = false;
  match unsafe {
    OpenMutexW(
      SYNCHRONIZATION_SYNCHRONIZE,
      false,
      PCWSTR(mutex_name.as_ptr()),
    )
  } {
    Ok(handle) => {
      drop(owned_handle(handle));
      mutex_present = true;
    }
    Err(error) if is_win32_error(&error, ERROR_FILE_NOT_FOUND) => {}
    Err(error) if is_win32_error(&error, ERROR_ACCESS_DENIED) => {
      // An elevated legacy process may deny this probe. Treat that as a
      // possible existing instance and require successful window delivery.
      mutex_present = true;
    }
    Err(error) => {
      return Err(format!(
        "Could not check for an older HardwareVisualizer instance ({error:?}). \
         The launch stopped before database startup."
      ));
    }
  }

  let window = plugin_instance_window(identifier);

  if let Some(window) = window {
    allow_foreground_for_window(window);
    return Ok(Some(window));
  }
  if mutex_present {
    return Err(
      "An older HardwareVisualizer instance appears to be starting, but its \
       activation window is not available. Wait for it to finish starting, or \
       quit it from the tray and launch again. This launch stopped before \
       database startup."
        .to_string(),
    );
  }
  Ok(None)
}

fn send_legacy_activation(
  window: windows::Win32::Foundation::HWND,
) -> Result<(), String> {
  // The legacy plugin parses this as cwd|args. Empty fields trigger only its
  // existing callback; no command line or path crosses into the elevated app.
  let copy_data = COPYDATASTRUCT {
    dwData: LEGACY_SINGLE_INSTANCE_DATA,
    cbData: LEGACY_ACTIVATION_PAYLOAD.len() as u32,
    lpData: LEGACY_ACTIVATION_PAYLOAD.as_ptr() as *mut c_void,
  };
  let mut receiver_result = 0usize;
  unsafe { SetLastError(ERROR_SUCCESS) };
  let send_result = unsafe {
    SendMessageTimeoutW(
      window,
      WM_COPYDATA,
      WPARAM(0),
      LPARAM(&copy_data as *const COPYDATASTRUCT as isize),
      SMTO_ABORTIFHUNG | SMTO_BLOCK,
      LEGACY_SEND_TIMEOUT_MS,
      Some(&mut receiver_result),
    )
  };
  if send_result.0 == 0 {
    return Err(format!(
      "the bounded WM_COPYDATA send failed with {:?}",
      unsafe { GetLastError() }
    ));
  }
  if receiver_result != 1 {
    return Err(format!(
      "the legacy receiver returned {receiver_result} instead of confirming activation"
    ));
  }
  Ok(())
}

fn signal_and_close(handle: HANDLE, description: &str) -> Result<(), String> {
  let handle = owned_handle(handle);
  signal_event(token_handle(&handle), description)
}

fn signal_event(handle: HANDLE, description: &str) -> Result<(), String> {
  unsafe { SetEvent(handle) }
    .map_err(|error| format!("Could not signal {description}: {error:?}"))
}

fn activation_event_name(identifier: &str, user_sid: &str) -> String {
  format!(r"Local\{identifier}-activate-{user_sid}")
}

fn activation_security_descriptor(user_sid: &str) -> String {
  // Medium IL is intentional: a medium-integrity launcher must be able to
  // signal an event owned by the elevated process. Only this user's SID is
  // granted the event synchronization and modify-state rights.
  format!("D:P(A;;0x00100002;;;{user_sid})S:(ML;;NW;;;ME)")
}

fn is_win32_error(
  error: &windows::core::Error,
  expected: windows::Win32::Foundation::WIN32_ERROR,
) -> bool {
  error.code().0 as u32 == (0x8007_0000 | expected.0)
}

fn wide_null(value: &str) -> Vec<u16> {
  std::ffi::OsStr::new(value)
    .encode_wide()
    .chain(std::iter::once(0))
    .collect()
}

fn owned_handle(handle: HANDLE) -> OwnedHandle {
  unsafe { OwnedHandle::from_raw_handle(handle.0) }
}

fn token_handle(handle: &OwnedHandle) -> HANDLE {
  HANDLE(handle.as_raw_handle())
}

struct LocalSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl Drop for LocalSecurityDescriptor {
  fn drop(&mut self) {
    let _ = unsafe { LocalFree(Some(HLOCAL(self.0.0))) };
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use windows::Win32::Foundation::WAIT_TIMEOUT;

  #[test]
  fn activation_event_is_scoped_to_the_user_and_session() {
    assert_eq!(
      activation_event_name("HardwareVisualizer", "S-1-5-21-42"),
      r"Local\HardwareVisualizer-activate-S-1-5-21-42"
    );
  }

  #[test]
  fn event_acl_allows_only_the_user_and_medium_integrity_writes() {
    assert_eq!(
      activation_security_descriptor("S-1-5-21-42"),
      "D:P(A;;0x00100002;;;S-1-5-21-42)S:(ML;;NW;;;ME)"
    );
  }

  #[test]
  fn legacy_activation_sends_only_the_empty_open_signal() {
    assert_eq!(LEGACY_ACTIVATION_PAYLOAD, b"|\0");
    assert_eq!(LEGACY_SINGLE_INSTANCE_DATA, 1542);
    assert_eq!(LEGACY_SEND_TIMEOUT_MS, 5_000);
  }

  #[test]
  fn auto_reset_activation_event_round_trips_between_launches() {
    let user_sid = current_user_sid().expect("read test process SID");
    let identifier = format!(
      "HardwareVisualizer-test-{}-{}",
      std::process::id(),
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after Unix epoch")
        .as_nanos()
    );
    let event_name = activation_event_name(&identifier, &user_sid);
    let (primary, was_present) =
      create_activation_event(&event_name, &user_sid).expect("create primary event");
    assert!(!was_present);
    let (secondary, was_present) =
      create_activation_event(&event_name, &user_sid).expect("open secondary event");
    assert!(was_present);

    signal_event(secondary.raw(), "the test activation event").expect("signal event");
    assert_eq!(
      unsafe { WaitForSingleObject(primary.raw(), 1_000) },
      WAIT_OBJECT_0,
      "the primary launch should receive the secondary launch signal"
    );
    assert_eq!(
      unsafe { WaitForSingleObject(primary.raw(), 0) },
      WAIT_TIMEOUT,
      "an auto-reset event should consume its signal after one receiver"
    );
  }
}
