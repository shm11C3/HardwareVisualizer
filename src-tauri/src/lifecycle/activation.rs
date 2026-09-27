//! Windows "open the running app" channel that crosses the integrity boundary
//! (#2291).
//!
//! `tauri-plugin-single-instance` notifies the running instance with
//! `WM_COPYDATA`. When that instance runs as administrator (Elevated Startup
//! Mode) and the new launch from Start or the taskbar does not, User Interface
//! Privilege Isolation drops the message: `SendMessageW` fails with
//! `ERROR_ACCESS_DENIED`, the plugin ignores the result, and the new process
//! exits without restoring the hidden window.
//!
//! The running instance therefore also owns a named auto-reset event in the
//! session's `Local\` namespace. A new launch that finds it sets it and exits
//! before it loads settings or opens the database, and the running instance
//! restores its main window through the same path as the plugin callback.
//! The event carries no payload, so nothing from the new process's command
//! line reaches the (possibly elevated) running instance. Its protected DACL
//! grants only the current user `SYNCHRONIZE | EVENT_MODIFY_STATE`, and its
//! Medium no-write-up label lets a normal launch set it while low-integrity
//! processes cannot.
//!
//! Why not let the plugin window accept lower-integrity `WM_COPYDATA`
//! (`ChangeWindowMessageFilterEx`): its receiver trusts the payload as a
//! NUL-terminated string, so opening it to lower-integrity senders would widen
//! what they can feed into an elevated process instead of only asking it to
//! show its window.

use std::ffi::c_void;

use tauri::AppHandle;
use windows::Win32::Foundation::{
  CloseHandle, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, GetLastError, HANDLE, HLOCAL,
  LocalFree, WAIT_OBJECT_0,
};
use windows::Win32::Security::Authorization::{
  ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
  SDDL_REVISION_1,
};
use windows::Win32::Security::{
  GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
  TOKEN_USER, TokenUser,
};
use windows::Win32::System::Threading::{
  CreateEventW, EVENT_MODIFY_STATE, GetCurrentProcess, INFINITE, OpenEventW,
  OpenProcessToken, SetEvent, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};
use windows::core::{PCWSTR, PWSTR};

use crate::{log_info, log_warn};

/// `SYNCHRONIZE | EVENT_MODIFY_STATE`: wait on the event and set it, nothing
/// else (no reading or changing its security, no deleting).
const ACTIVATION_EVENT_ACCESS: &str = "0x00100002";

/// The per-session event name for an app identifier. `Local\` keeps it inside
/// the current logon session, which an elevated and a normal process of the
/// same user share.
pub fn activation_event_name(identifier: &str) -> String {
  format!("Local\\{identifier}-activate")
}

/// The event's security descriptor: a protected DACL granting only `user_sid`
/// wait-and-set access, and a Medium mandatory label with no-write-up so a
/// normal-integrity launch can set an event an elevated instance created.
pub fn activation_event_sddl(user_sid: &str) -> String {
  format!("D:P(A;;{ACTIVATION_EVENT_ACCESS};;;{user_sid})S:(ML;;NW;;;ME)")
}

/// What a new launch did with the running instance's activation event.
#[derive(Debug, PartialEq, Eq)]
pub enum Delivery {
  /// No running instance owns the event. Startup continues; the
  /// single-instance plugin still covers an instance that predates this
  /// channel.
  NoRunningInstance,
  /// The running instance was asked to show its window; this launch exits.
  Delivered,
  /// The event exists but could not be set. Startup continues so the
  /// single-instance plugin keeps its previous behavior.
  Failed(String),
}

/// Ask the running instance that owns `event_name` to show its main window.
pub fn signal_running_instance(event_name: &str) -> Delivery {
  let name = wide_null(event_name);
  let event =
    match unsafe { OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(name.as_ptr())) } {
      Ok(handle) => OwnedHandle(handle),
      Err(e) if e.code() == ERROR_FILE_NOT_FOUND.to_hresult() => {
        return Delivery::NoRunningInstance;
      }
      Err(e) => return Delivery::Failed(format!("could not open {event_name}: {e}")),
    };

  // This launch came from the user, so it may hand the foreground right on;
  // the running instance needs it to focus its window. The event does not
  // identify its owner's process, and the right lapses at the next input.
  let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };

  match unsafe { SetEvent(event.0) } {
    Ok(()) => Delivery::Delivered,
    Err(e) => Delivery::Failed(format!("could not set {event_name}: {e}")),
  }
}

/// Create the activation event for this running instance and restore the
/// main window whenever a new launch sets it. Failures are logged and leave
/// the single-instance plugin as the only reopen path.
pub fn listen(app: &AppHandle, event_name: &str) {
  let event = match ActivationEvent::create(event_name) {
    Ok(event) => event,
    Err(e) => {
      log_warn!(
        &format!("Start and taskbar launches cannot reopen this instance: {e}"),
        "lifecycle::activation::listen",
        None::<&str>
      );
      return;
    }
  };

  let app = app.clone();
  let spawned = std::thread::Builder::new()
    .name("activation-listener".into())
    .spawn(move || {
      loop {
        if let Err(e) = event.wait() {
          log_warn!(
            &format!("Stopped listening for Start and taskbar launches: {e}"),
            "lifecycle::activation::listen",
            None::<&str>
          );
          return;
        }
        log_info!(
          "A new launch asked this instance to show its window",
          "lifecycle::activation::listen",
          None::<&str>
        );
        let handle = app.clone();
        if let Err(e) = app.run_on_main_thread(move || super::on_second_instance(&handle))
        {
          log_warn!(
            &format!("could not restore the main window for a new launch: {e}"),
            "lifecycle::activation::listen",
            None::<&str>
          );
        }
      }
    });
  if let Err(e) = spawned {
    log_warn!(
      &format!("could not start the activation listener: {e}"),
      "lifecycle::activation::listen",
      None::<&str>
    );
  }
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
  fn drop(&mut self) {
    let _ = unsafe { CloseHandle(self.0) };
  }
}

/// The auto-reset event a running instance owns.
struct ActivationEvent(OwnedHandle);

// SAFETY: a kernel event handle is valid process-wide, and waiting on it from
// the listener thread does not share any other state.
unsafe impl Send for ActivationEvent {}

impl ActivationEvent {
  /// Create the event with [`activation_event_sddl`]. An event that already
  /// exists under the name is refused rather than reused: its security was
  /// set by whoever created it, not by this instance.
  fn create(event_name: &str) -> Result<Self, String> {
    let sddl = wide_null(&activation_event_sddl(&current_user_sid()?));
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
      ConvertStringSecurityDescriptorToSecurityDescriptorW(
        PCWSTR(sddl.as_ptr()),
        SDDL_REVISION_1,
        &mut descriptor,
        None,
      )
    }
    .map_err(|e| format!("security descriptor failed: {e}"))?;
    let attributes = SECURITY_ATTRIBUTES {
      nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
      lpSecurityDescriptor: descriptor.0,
      bInheritHandle: false.into(),
    };
    let name = wide_null(event_name);
    let created =
      unsafe { CreateEventW(Some(&attributes), false, false, PCWSTR(name.as_ptr())) };
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    let event =
      OwnedHandle(created.map_err(|e| format!("could not create {event_name}: {e}"))?);
    if already_exists {
      return Err(format!("{event_name} already exists"));
    }
    Ok(Self(event))
  }

  /// Block until a new launch sets the event.
  fn wait(&self) -> Result<(), String> {
    let result = unsafe { WaitForSingleObject(self.0.0, INFINITE) };
    if result == WAIT_OBJECT_0 {
      Ok(())
    } else {
      Err(format!("wait returned {:#x}: {}", result.0, unsafe {
        GetLastError().to_hresult().message()
      }))
    }
  }
}

/// The string SID of the user this process runs as. An elevated process runs
/// as the same user as a normal launch, so both resolve the same SID.
fn current_user_sid() -> Result<String, String> {
  let mut token = HANDLE::default();
  unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
    .map_err(|e| format!("could not open the process token: {e}"))?;
  let token = OwnedHandle(token);

  let mut length = 0u32;
  let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut length) };
  let mut buffer = vec![0u8; length as usize];
  unsafe {
    GetTokenInformation(
      token.0,
      TokenUser,
      Some(buffer.as_mut_ptr() as *mut c_void),
      length,
      &mut length,
    )
  }
  .map_err(|e| format!("could not read the token user: {e}"))?;
  // SAFETY: GetTokenInformation(TokenUser) filled `buffer` with a TOKEN_USER
  // whose SID points into the same buffer, which outlives this read.
  let sid = unsafe { (*(buffer.as_ptr() as *const TOKEN_USER)).User.Sid };

  let mut sid_string = PWSTR::null();
  unsafe { ConvertSidToStringSidW(sid, &mut sid_string) }
    .map_err(|e| format!("could not format the user SID: {e}"))?;
  let result = unsafe { sid_string.to_string() }
    .map_err(|e| format!("could not read the user SID: {e}"));
  let _ = unsafe { LocalFree(Some(HLOCAL(sid_string.0 as *mut c_void))) };
  result
}

fn wide_null(value: &str) -> Vec<u16> {
  value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo,
    SE_KERNEL_OBJECT,
  };
  use windows::Win32::Security::{DACL_SECURITY_INFORMATION, LABEL_SECURITY_INFORMATION};

  /// A name no other test run or real instance uses.
  fn unique_event_name(test: &str) -> String {
    activation_event_name(&format!(
      "HardwareVisualizerTest-{test}-{}-{:?}",
      std::process::id(),
      std::time::SystemTime::now()
    ))
  }

  #[test]
  fn the_event_name_is_session_local_and_per_identifier() {
    assert_eq!(
      activation_event_name("HardwareVisualizer"),
      "Local\\HardwareVisualizer-activate"
    );
  }

  #[test]
  fn the_security_descriptor_grants_only_the_user_wait_and_set_at_medium() {
    assert_eq!(
      activation_event_sddl("S-1-5-21-1-2-3-1001"),
      "D:P(A;;0x00100002;;;S-1-5-21-1-2-3-1001)S:(ML;;NW;;;ME)"
    );
  }

  #[test]
  fn a_launch_without_a_running_instance_starts_normally() {
    assert_eq!(
      signal_running_instance(&unique_event_name("absent")),
      Delivery::NoRunningInstance
    );
  }

  #[test]
  fn a_launch_with_a_running_instance_wakes_its_listener() {
    let name = unique_event_name("round-trip");
    let event = ActivationEvent::create(&name).expect("the event is created");

    assert_eq!(signal_running_instance(&name), Delivery::Delivered);
    assert_eq!(event.wait(), Ok(()));
  }

  #[test]
  fn an_event_someone_else_created_is_not_adopted() {
    let name = unique_event_name("squatted");
    let wide = wide_null(&name);
    // A same-session process that created the name first, with its own
    // (default) security instead of the reviewed descriptor.
    let _squatter = OwnedHandle(
      unsafe { CreateEventW(None, false, false, PCWSTR(wide.as_ptr())) }
        .expect("the squatting event is created"),
    );

    let adopted = ActivationEvent::create(&name);

    assert!(adopted.is_err_and(|e| e.contains("already exists")));
  }

  #[test]
  fn a_second_running_instance_cannot_take_over_the_event() {
    let name = unique_event_name("second-owner");
    let _first = ActivationEvent::create(&name).expect("the event is created");

    // The reviewed DACL does not grant the full access a create asks for.
    assert!(ActivationEvent::create(&name).is_err());
  }

  /// Render a descriptor's DACL and label as SDDL, so the expected and the
  /// stored descriptor are compared in the same canonical form (Windows
  /// abbreviates well-known SIDs, such as `LA` for the built-in
  /// Administrator a CI runner may run as).
  fn dacl_and_label_sddl(descriptor: PSECURITY_DESCRIPTOR) -> String {
    let mut sddl = PWSTR::null();
    unsafe {
      ConvertSecurityDescriptorToStringSecurityDescriptorW(
        descriptor,
        SDDL_REVISION_1,
        DACL_SECURITY_INFORMATION | LABEL_SECURITY_INFORMATION,
        &mut sddl,
        None,
      )
    }
    .expect("the descriptor converts to SDDL");
    let rendered = unsafe { sddl.to_string() }.expect("the SDDL is UTF-16");
    let _ = unsafe { LocalFree(Some(HLOCAL(sddl.0 as *mut c_void))) };
    // The kernel marks a stored SACL auto-inherited (`AI`); that flag does
    // not change the label.
    rendered.replacen("S:AI(", "S:(", 1)
  }

  #[test]
  fn the_created_event_carries_the_reviewed_dacl_and_label() {
    let name = unique_event_name("security");
    let event = ActivationEvent::create(&name).expect("the event is created");

    let mut stored = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
      GetSecurityInfo(
        event.0.0,
        SE_KERNEL_OBJECT,
        DACL_SECURITY_INFORMATION | LABEL_SECURITY_INFORMATION,
        None,
        None,
        None,
        None,
        Some(&mut stored),
      )
    };
    assert!(status.is_ok(), "GetSecurityInfo failed: {status:?}");
    let actual = dacl_and_label_sddl(stored);
    let _ = unsafe { LocalFree(Some(HLOCAL(stored.0))) };

    let user_sid = current_user_sid().expect("the user SID resolves");
    let reviewed = wide_null(&activation_event_sddl(&user_sid));
    let mut expected = PSECURITY_DESCRIPTOR::default();
    unsafe {
      ConvertStringSecurityDescriptorToSecurityDescriptorW(
        PCWSTR(reviewed.as_ptr()),
        SDDL_REVISION_1,
        &mut expected,
        None,
      )
    }
    .expect("the reviewed SDDL parses");
    let expected_sddl = dacl_and_label_sddl(expected);
    let _ = unsafe { LocalFree(Some(HLOCAL(expected.0))) };

    assert_eq!(actual, expected_sddl);
    assert!(actual.starts_with("D:P(A;;0x100002;;;"), "{actual}");
    assert!(actual.ends_with(")S:(ML;;NW;;;ME)"), "{actual}");
  }
}
