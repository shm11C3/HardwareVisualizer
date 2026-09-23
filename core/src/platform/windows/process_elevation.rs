use crate::enums::error::PlatformError;
use crate::log_warn;
use crate::platform::traits::{
  ElevatedProcessRun, ElevationAvailability, ProcessExitWait, ProcessIdentity,
};
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use windows::Win32::Foundation::{
  CloseHandle, ERROR_ACCESS_DENIED, ERROR_CANCELLED, ERROR_INVALID_PARAMETER,
  ERROR_NOT_ALL_ASSIGNED, FILETIME, GetLastError, HANDLE, LUID, WAIT_OBJECT_0,
};
use windows::Win32::Security::{
  AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_DEBUG_NAME,
  SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::Storage::FileSystem::{
  CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES,
  FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GETFINALPATHNAMEBYHANDLE_FLAGS,
  GetFinalPathNameByHandleW, OPEN_EXISTING, VOLUME_NAME_DOS,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Threading::{
  GetCurrentProcess, GetExitCodeProcess, GetProcessTimes, INFINITE, OpenProcess,
  OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
  TerminateProcess, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{
  FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, IsUserAnAdmin, KF_FLAG_DEFAULT,
  SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, SHGetKnownFolderPath,
  ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{GUID, PCWSTR};

/// Elevated launches run `current_exe()`. When its folder is writable without
/// elevation (the NSIS per-user install, or an MSI outside Program Files), a
/// same-user process can replace the executable before the UAC prompt and have
/// its own code run as administrator, so elevation is only offered for an
/// executable under Program Files (#2216). The check compares resolved final
/// paths, so `..` segments and junctions outside Program Files cannot pass it,
/// and it trusts Program Files' default ACL: only an administrator can make a
/// folder inside it writable. Any failure to resolve a path refuses elevation.
pub fn elevation_availability() -> ElevationAvailability {
  match protected_executable_path() {
    Ok(Some(_)) => ElevationAvailability::Available,
    Ok(None) => ElevationAvailability::UnprotectedLocation,
    Err(detail) => {
      log_warn!(
        format!("Refusing elevation: could not verify the install folder: {detail}"),
        "process_elevation::elevation_availability",
        None::<&str>
      );
      ElevationAvailability::UnprotectedLocation
    }
  }
}

/// The resolved path of the current executable when it sits under Program
/// Files, `None` when it does not. The executable file itself is resolved, not
/// only its folder, and callers launch this resolved path: a junction or
/// symbolic link in `current_exe()` that is retargeted after the check cannot
/// redirect the elevated launch to a replaceable file.
fn protected_executable_path() -> Result<Option<PathBuf>, String> {
  let exe_path = std::env::current_exe()
    .map_err(|e| format!("Failed to obtain executable file path: {e}"))?;
  let resolved = PathBuf::from(final_path(&exe_path)?);
  let resolved_dir = resolved
    .parent()
    .ok_or_else(|| "The executable path has no parent folder".to_string())?
    .to_string_lossy()
    .into_owned();

  let mut known_roots = Vec::new();
  for folder in [&FOLDERID_ProgramFiles, &FOLDERID_ProgramFilesX86] {
    known_roots.push(known_folder_path(folder)?);
  }
  let roots = resolved_roots(&known_roots, final_path);

  Ok(is_within_any_root(&resolved_dir, &roots).then_some(resolved))
}

/// The final paths of `roots`. A root that cannot be resolved is left out
/// rather than compared unresolved, so it can never make a folder protected;
/// the other root still counts, so one missing folder does not refuse a valid
/// install under the other.
fn resolved_roots(
  roots: &[String],
  resolve: impl Fn(&Path) -> Result<String, String>,
) -> Vec<String> {
  roots
    .iter()
    .filter_map(|root| match resolve(Path::new(root)) {
      Ok(resolved) => Some(resolved),
      Err(detail) => {
        log_warn!(
          format!("Ignoring a Program Files folder that could not be resolved: {detail}"),
          "process_elevation::resolved_roots",
          None::<&str>
        );
        None
      }
    })
    .collect()
}

/// Case-insensitive check that `path` is one of `roots` or inside one of them,
/// on a path-component boundary.
fn is_within_any_root(path: &str, roots: &[String]) -> bool {
  let path = normalize_for_comparison(path);
  roots.iter().any(|root| {
    let root = normalize_for_comparison(root);
    !root.is_empty() && (path == root || path.starts_with(&format!("{root}\\")))
  })
}

fn normalize_for_comparison(path: &str) -> String {
  path.trim_end_matches(['\\', '/']).to_lowercase()
}

/// The path of `path` after resolving `..`, junctions, and symbolic links.
fn final_path(path: &Path) -> Result<String, String> {
  let wide_path = os_wide_null(path.as_os_str());
  let handle = unsafe {
    CreateFileW(
      PCWSTR(wide_path.as_ptr()),
      FILE_READ_ATTRIBUTES.0,
      FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
      None,
      OPEN_EXISTING,
      FILE_FLAG_BACKUP_SEMANTICS,
      None,
    )
  }
  .map_err(|e| format!("Failed to open {}: {e}", path.display()))?;

  let mut buffer = vec![0u16; 512];
  let resolved = loop {
    let length = unsafe {
      GetFinalPathNameByHandleW(
        handle,
        &mut buffer,
        GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
      )
    } as usize;
    if length == 0 {
      break Err(format!(
        "Failed to resolve {}: {}",
        path.display(),
        windows::core::Error::from_thread()
      ));
    }
    if length < buffer.len() {
      break Ok(String::from_utf16_lossy(&buffer[..length]));
    }
    buffer.resize(length + 1, 0);
  };
  let _ = unsafe { CloseHandle(handle) };

  resolved.map(|resolved| strip_verbatim_prefix(&resolved))
}

fn strip_verbatim_prefix(path: &str) -> String {
  if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
    format!(r"\\{rest}")
  } else if let Some(rest) = path.strip_prefix(r"\\?\") {
    rest.to_string()
  } else {
    path.to_string()
  }
}

fn known_folder_path(folder: &GUID) -> Result<String, String> {
  let path = unsafe { SHGetKnownFolderPath(folder, KF_FLAG_DEFAULT, None) }
    .map_err(|e| format!("Failed to resolve a Program Files folder: {e}"))?;
  let result = unsafe { path.to_string() }
    .map_err(|e| format!("Program Files folder path is not valid UTF-16: {e}"));
  unsafe { CoTaskMemFree(Some(path.0 as *const _)) };
  result
}

pub fn is_process_elevated() -> Result<bool, PlatformError> {
  Ok(unsafe { IsUserAnAdmin().as_bool() })
}

pub fn relaunch_current_process_elevated(args: &[String]) -> Result<(), PlatformError> {
  let args = args.iter().map(OsString::from).collect::<Vec<_>>();
  let launched = launch_current_executable_elevated(&args, "restart as administrator")?;

  match launched {
    Some(process) => {
      let _ = unsafe { CloseHandle(process) };
      Ok(())
    }
    // The existing restart contract reports a declined UAC prompt as a failure
    // so the caller can roll back Elevated Startup Mode.
    None => Err(PlatformError::fault(
      "Failed to restart as administrator: the elevation prompt was declined",
    )),
  }
}

/// How long [`run_current_executable_elevated`] waits for the elevated child.
/// Its only caller is External Component Setup, whose child bounds each of
/// its own slow steps (two downloads and the runtime installer, five minutes
/// each), so a run that is still going after twenty minutes is stuck outside
/// those bounds. Without a limit the Settings action's blocking task never
/// returns and its per-component in-flight guard refuses every retry until
/// the app restarts.
const ELEVATED_RUN_TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// Launch the current executable elevated with `args`, wait for it to exit,
/// and return its exit code. A declined UAC prompt is reported as
/// [`ElevatedProcessRun::Declined`], not as an error, and a child that
/// outlives [`ELEVATED_RUN_TIMEOUT`] as [`ElevatedProcessRun::TimedOut`].
pub fn run_current_executable_elevated(
  args: &[String],
) -> Result<ElevatedProcessRun, PlatformError> {
  let args = args.iter().map(OsString::from).collect::<Vec<_>>();
  let Some(process) = launch_current_executable_elevated(&args, "run as administrator")?
  else {
    return Ok(ElevatedProcessRun::Declined);
  };

  let run = wait_for_exit_code(process, ELEVATED_RUN_TIMEOUT);
  let _ = unsafe { CloseHandle(process) };
  Ok(run)
}

/// Returns the process handle, or `None` when the user declined the prompt.
fn launch_current_executable_elevated(
  args: &[OsString],
  action: &str,
) -> Result<Option<HANDLE>, PlatformError> {
  // Launch exactly the resolved file that passed the check.
  let exe_path = match protected_executable_path() {
    Ok(Some(path)) => path,
    Ok(None) => {
      return Err(PlatformError::unavailable(format!(
        "Refusing to {action}: HardwareVisualizer is not installed under Program Files, \
         so its executable could have been replaced."
      )));
    }
    Err(detail) => {
      return Err(PlatformError::unavailable(format!(
        "Refusing to {action}: could not verify the install folder: {detail}"
      )));
    }
  };
  let params = args
    .iter()
    .map(|arg| quote_windows_arg(arg))
    .collect::<Vec<_>>()
    .join(" ");

  let verb = wide_null("runas");
  let file = os_wide_null(exe_path.as_os_str());
  let parameters = os_wide_null(OsString::from(params).as_os_str());

  let mut execute_info = SHELLEXECUTEINFOW {
    cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
    fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
    lpVerb: PCWSTR(verb.as_ptr()),
    lpFile: PCWSTR(file.as_ptr()),
    lpParameters: PCWSTR(parameters.as_ptr()),
    nShow: SW_SHOWNORMAL.0,
    ..Default::default()
  };

  if let Err(e) = unsafe { ShellExecuteExW(&mut execute_info) } {
    if e.code() == ERROR_CANCELLED.to_hresult() {
      return Ok(None);
    }
    return Err(PlatformError::fault(format!("Failed to {action}: {e}")));
  }

  if execute_info.hProcess.is_invalid() {
    return Err(PlatformError::fault(format!(
      "Failed to {action}: no process handle was returned"
    )));
  }

  Ok(Some(execute_info.hProcess))
}

pub fn current_process_identity() -> Result<ProcessIdentity, PlatformError> {
  let creation_time = process_creation_time(unsafe { GetCurrentProcess() })?;
  Ok(ProcessIdentity {
    pid: std::process::id(),
    creation_time,
  })
}

/// Block until the process `identity` names has exited. The wait is
/// unbounded once the identity is verified: the handle keeps the id from
/// being reused, so only the named process can end the wait. The access
/// requested (`SYNCHRONIZE` and limited query) is granted to an elevated
/// process on the unelevated process that launched it when both run as the
/// same account; see [`open_process_to_wait`] for the other case.
pub fn wait_for_process_exit(
  identity: &ProcessIdentity,
) -> Result<ProcessExitWait, PlatformError> {
  let pid = identity.pid;
  let Some(process) = open_process_to_wait(pid)? else {
    return Ok(ProcessExitWait::AlreadyExited);
  };

  let waited = match process_creation_time(process) {
    // The id now belongs to a process started later: the named one has
    // exited and its id was reused.
    Ok(creation_time) if creation_time != identity.creation_time => {
      Ok(ProcessExitWait::AlreadyExited)
    }
    Ok(_) => match unsafe { WaitForSingleObject(process, INFINITE) } {
      WAIT_OBJECT_0 => Ok(ProcessExitWait::Exited),
      _ => Err(PlatformError::fault(format!(
        "Failed to wait for process {pid}: {}",
        windows::core::Error::from_thread()
      ))),
    },
    Err(e) => Err(e),
  };
  let _ = unsafe { CloseHandle(process) };
  waited
}

/// Open `pid` for waiting and reading its creation time. `None` when no
/// process holds the id any more.
///
/// Over-the-shoulder elevation: when a standard user answers the UAC prompt
/// with another administrator's credentials, the elevated child runs as that
/// account, and the parent's default DACL (its creator and SYSTEM) refuses it
/// with `ERROR_ACCESS_DENIED`. A full administrator token holds
/// `SeDebugPrivilege`, which lets it open any process regardless of its DACL,
/// so the open is retried once with that privilege enabled. It is enabled for
/// the retry only: the handle keeps the access it was opened with, and the
/// creation-time check still decides whether it is the parent.
fn open_process_to_wait(pid: u32) -> Result<Option<HANDLE>, PlatformError> {
  let open_error = |e: windows::core::Error| {
    PlatformError::fault(format!("Failed to open process {pid} to wait for it: {e}"))
  };
  match open_process(pid) {
    Ok(process) => Ok(process),
    Err(e) if e.code() == ERROR_ACCESS_DENIED.to_hresult() => {
      let Some(_debug_privilege) = DebugPrivilege::enable()? else {
        return Err(PlatformError::fault(format!(
          "Failed to open process {pid} to wait for it: access was denied and this \
           process does not hold SeDebugPrivilege"
        )));
      };
      open_process(pid).map_err(open_error)
    }
    Err(e) => Err(open_error(e)),
  }
}

/// `Ok(None)` when no process holds `pid` any more.
fn open_process(pid: u32) -> Result<Option<HANDLE>, windows::core::Error> {
  match unsafe {
    OpenProcess(
      PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
      false,
      pid,
    )
  } {
    Ok(process) => Ok(Some(process)),
    Err(e) if e.code() == ERROR_INVALID_PARAMETER.to_hresult() => Ok(None),
    Err(e) => Err(e),
  }
}

/// `SeDebugPrivilege` enabled on this process's token for as long as the
/// value lives; dropping it restores the privilege's previous state.
struct DebugPrivilege {
  previous: TOKEN_PRIVILEGES,
}

impl DebugPrivilege {
  /// `Ok(None)` when the token does not hold the privilege at all, which is
  /// the case for every token that is not a full administrator token.
  fn enable() -> Result<Option<Self>, PlatformError> {
    let mut previous = TOKEN_PRIVILEGES::default();
    let held = with_process_token(|token| {
      let mut luid = LUID::default();
      unsafe { LookupPrivilegeValueW(PCWSTR::null(), SE_DEBUG_NAME, &mut luid) }
        .map_err(|e| {
          PlatformError::fault(format!("Failed to look up SeDebugPrivilege: {e}"))
        })?;
      let enabled = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
          Luid: luid,
          Attributes: SE_PRIVILEGE_ENABLED,
        }],
      };
      let mut returned_length = 0u32;
      unsafe {
        AdjustTokenPrivileges(
          token,
          false,
          Some(&enabled),
          std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
          Some(&mut previous),
          Some(&mut returned_length),
        )
      }
      .map_err(|e| {
        PlatformError::fault(format!("Failed to enable SeDebugPrivilege: {e}"))
      })?;
      // The call succeeds even when the token does not hold the privilege;
      // only the last error tells.
      Ok(unsafe { GetLastError() } != ERROR_NOT_ALL_ASSIGNED)
    })?;
    Ok(held.then_some(Self { previous }))
  }
}

impl Drop for DebugPrivilege {
  fn drop(&mut self) {
    let previous = self.previous;
    let _ = with_process_token(|token| {
      unsafe { AdjustTokenPrivileges(token, false, Some(&previous), 0, None, None) }
        .map_err(|e| {
          PlatformError::fault(format!("Failed to restore SeDebugPrivilege: {e}"))
        })
    });
  }
}

fn with_process_token<T>(
  f: impl FnOnce(HANDLE) -> Result<T, PlatformError>,
) -> Result<T, PlatformError> {
  let mut token = HANDLE::default();
  unsafe {
    OpenProcessToken(
      GetCurrentProcess(),
      TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
      &mut token,
    )
  }
  .map_err(|e| PlatformError::fault(format!("Failed to open the process token: {e}")))?;
  let result = f(token);
  let _ = unsafe { CloseHandle(token) };
  result
}

/// The creation time of `process` as one integer, so it can travel on a
/// command line and be compared exactly.
fn process_creation_time(process: HANDLE) -> Result<u64, PlatformError> {
  let mut creation = FILETIME::default();
  let mut exit = FILETIME::default();
  let mut kernel = FILETIME::default();
  let mut user = FILETIME::default();
  unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) }
    .map_err(|e| {
    PlatformError::fault(format!("Failed to read the process creation time: {e}"))
  })?;
  Ok((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}

/// Wait up to `limit` for `process` to exit and read its exit code. On
/// timeout the process is terminated so the caller does not leave a stray
/// child behind; the handle `ShellExecuteExW` returns for an elevated child
/// does not always carry `PROCESS_TERMINATE` for a medium-integrity parent,
/// so a refused termination is logged and the result is still `TimedOut`.
/// This wait is deliberately separate from [`wait_for_process_exit`], whose
/// unbounded wait on a verified parent is part of the relaunch handoff.
fn wait_for_exit_code(process: HANDLE, limit: Duration) -> ElevatedProcessRun {
  let limit_ms = u32::try_from(limit.as_millis()).unwrap_or(INFINITE - 1);
  if unsafe { WaitForSingleObject(process, limit_ms) } == WAIT_OBJECT_0 {
    let mut exit_code: u32 = 0;
    let exit_code = unsafe { GetExitCodeProcess(process, &mut exit_code) }
      .ok()
      .map(|()| exit_code as i32);
    return ElevatedProcessRun::Exited { exit_code };
  }

  log_warn!(
    format!(
      "The elevated process did not exit within {} s; terminating it",
      limit.as_secs()
    ),
    "process_elevation::wait_for_exit_code",
    None::<&str>
  );
  if let Err(e) = unsafe { TerminateProcess(process, 1) } {
    log_warn!(
      format!("Failed to terminate the elevated process: {e}"),
      "process_elevation::wait_for_exit_code",
      None::<&str>
    );
  }
  ElevatedProcessRun::TimedOut
}

fn os_wide_null(value: &OsStr) -> Vec<u16> {
  value.encode_wide().chain(std::iter::once(0)).collect()
}

fn wide_null(value: &str) -> Vec<u16> {
  value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn quote_windows_arg(arg: &OsStr) -> String {
  let value = arg.to_string_lossy();

  if value.is_empty() {
    return "\"\"".to_string();
  }

  if !value
    .chars()
    .any(|ch| matches!(ch, ' ' | '\t' | '\n' | '\u{000b}' | '"'))
  {
    return value.into_owned();
  }

  let mut quoted = String::from("\"");
  let mut backslashes = 0;

  for ch in value.chars() {
    if ch == '\\' {
      backslashes += 1;
      continue;
    }

    if ch == '"' {
      quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
      quoted.push('"');
      backslashes = 0;
      continue;
    }

    if backslashes > 0 {
      quoted.push_str(&"\\".repeat(backslashes));
      backslashes = 0;
    }
    quoted.push(ch);
  }

  if backslashes > 0 {
    quoted.push_str(&"\\".repeat(backslashes * 2));
  }

  quoted.push('"');
  quoted
}

#[cfg(test)]
mod tests {
  use super::{
    DebugPrivilege, current_process_identity, is_process_elevated, is_within_any_root,
    process_creation_time, quote_windows_arg, resolved_roots, strip_verbatim_prefix,
    wait_for_exit_code, wait_for_process_exit,
  };
  use crate::platform::traits::ElevatedProcessRun;
  use std::time::Duration;

  #[test]
  fn a_bounded_wait_reports_the_exit_code_of_a_child_that_finishes_in_time() {
    let mut child = std::process::Command::new("cmd")
      .args(["/C", "exit", "5"])
      .spawn()
      .expect("cmd runs");

    let run = wait_for_exit_code(HANDLE(child.as_raw_handle()), Duration::from_secs(30));

    assert_eq!(run, ElevatedProcessRun::Exited { exit_code: Some(5) });
    let _ = child.wait();
  }

  #[test]
  fn a_bounded_wait_terminates_a_child_that_outlives_the_limit() {
    // `ping -n 30` keeps the child alive for about 30 s; the limit is far
    // shorter, so the wait must give up, terminate the child, and report it.
    let mut child = std::process::Command::new("ping")
      .args(["-n", "30", "127.0.0.1"])
      .stdout(std::process::Stdio::null())
      .spawn()
      .expect("ping runs");
    let started = std::time::Instant::now();

    let run =
      wait_for_exit_code(HANDLE(child.as_raw_handle()), Duration::from_millis(500));

    assert_eq!(run, ElevatedProcessRun::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(10));
    // The child was terminated: reaping it returns promptly and not success.
    let status = child.wait().expect("the terminated child is reaped");
    assert!(!status.success());
    assert!(started.elapsed() < Duration::from_secs(10));
  }

  #[test]
  fn the_debug_privilege_is_held_exactly_by_a_full_administrator_token() {
    // A full administrator token carries SeDebugPrivilege (disabled); a
    // filtered administrator token and a standard user token do not. The
    // test process may run either way, so both outcomes are checked against
    // the elevation state rather than assumed.
    let elevated = is_process_elevated().expect("elevation state is readable");
    match DebugPrivilege::enable().expect("adjusting the token does not fail") {
      Some(enabled) => {
        assert!(elevated, "the privilege was enabled without elevation");
        drop(enabled);
      }
      None => assert!(!elevated, "an elevated token holds SeDebugPrivilege"),
    }
  }
  use crate::platform::traits::{ProcessExitWait, ProcessIdentity};
  use std::ffi::OsStr;
  use std::os::windows::io::AsRawHandle;
  use windows::Win32::Foundation::HANDLE;

  fn identity_of(child: &std::process::Child) -> ProcessIdentity {
    let creation_time = process_creation_time(HANDLE(child.as_raw_handle()))
      .expect("the child's creation time is readable");
    ProcessIdentity {
      pid: child.id(),
      creation_time,
    }
  }

  fn spawn_short_lived_child() -> std::process::Child {
    std::process::Command::new("cmd")
      .args(["/C", "exit", "0"])
      .spawn()
      .expect("cmd runs")
  }

  #[test]
  fn the_current_identity_names_this_process() {
    let identity = current_process_identity().expect("own identity is readable");
    assert_eq!(identity.pid, std::process::id());
    assert_ne!(identity.creation_time, 0);
  }

  #[test]
  fn waiting_returns_once_the_verified_process_has_exited() {
    let mut child = spawn_short_lived_child();

    // The `Child` keeps a handle open, so the id cannot be reused before
    // the wait below has observed the exit.
    assert_eq!(
      wait_for_process_exit(&identity_of(&child)),
      Ok(ProcessExitWait::Exited)
    );
    assert!(child.wait().expect("child is reaped").success());
  }

  #[test]
  fn a_different_creation_time_counts_as_already_exited() {
    // The running test process would block an unbounded wait forever, so
    // the mismatch must be decided before waiting.
    let mut identity = current_process_identity().expect("own identity is readable");
    identity.creation_time += 1;

    assert_eq!(
      wait_for_process_exit(&identity),
      Ok(ProcessExitWait::AlreadyExited)
    );
  }

  #[test]
  fn an_id_no_process_holds_counts_as_already_exited() {
    // Waiting on a child whose id was just released would be racy here: a
    // process another test spawns in the same timer tick can take that id
    // with the same creation time. The System Idle Process id cannot be
    // opened at all and is documented to fail with `ERROR_INVALID_PARAMETER`,
    // which is the same error a released id produces.
    assert_eq!(
      wait_for_process_exit(&ProcessIdentity {
        pid: 0,
        creation_time: 0,
      }),
      Ok(ProcessExitWait::AlreadyExited)
    );
  }

  fn roots() -> Vec<String> {
    vec![
      r"C:\Program Files".to_string(),
      r"C:\Program Files (x86)\".to_string(),
    ]
  }

  #[test]
  fn program_files_folders_are_protected() {
    assert!(is_within_any_root(
      r"C:\Program Files\HardwareVisualizer",
      &roots()
    ));
    assert!(is_within_any_root(
      r"c:\program files\HardwareVisualizer\",
      &roots()
    ));
    assert!(is_within_any_root(
      r"C:\Program Files (x86)\HardwareVisualizer",
      &roots()
    ));
  }

  #[test]
  fn other_folders_are_unprotected() {
    assert!(!is_within_any_root(
      r"C:\Users\alice\AppData\Local\HardwareVisualizer",
      &roots()
    ));
    // A sibling whose name only starts with the root is not inside it.
    assert!(!is_within_any_root(
      r"C:\Program Files Evil\HardwareVisualizer",
      &roots()
    ));
    assert!(!is_within_any_root(
      r"D:\Program Files\HardwareVisualizer",
      &roots()
    ));
    assert!(!is_within_any_root(r"C:\Program Files\x", &[String::new()]));
  }

  #[test]
  fn program_files_resolves_to_a_plain_drive_path() {
    let root =
      super::known_folder_path(&windows::Win32::UI::Shell::FOLDERID_ProgramFiles)
        .expect("Program Files resolves");
    let resolved = super::final_path(std::path::Path::new(&root))
      .expect("Program Files final path resolves");
    assert!(!resolved.starts_with(r"\\?\"), "{resolved}");
    assert!(is_within_any_root(&resolved, &[root]));
  }

  #[test]
  fn a_test_binary_outside_program_files_cannot_elevate() {
    // Test binaries run from the cargo target folder, never Program Files.
    assert_eq!(
      super::elevation_availability(),
      crate::platform::traits::ElevationAvailability::UnprotectedLocation
    );
    assert!(super::relaunch_current_process_elevated(&[]).is_err());
  }

  #[test]
  fn final_path_follows_a_junction_to_the_real_file() {
    let base =
      std::env::temp_dir().join(format!("hardviz-final-path-{}", std::process::id()));
    let target = base.join("target");
    let link = base.join("link");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("app.exe"), b"").unwrap();
    let status = std::process::Command::new("cmd")
      .args(["/C", "mklink", "/J"])
      .arg(&link)
      .arg(&target)
      .output()
      .expect("mklink runs");
    assert!(status.status.success(), "{status:?}");

    let resolved = super::final_path(&link.join("app.exe")).expect("resolves");
    let expected =
      super::final_path(&target.join("app.exe")).expect("resolves the target");
    let _ = std::fs::remove_dir(&link);
    let _ = std::fs::remove_dir_all(&base);

    assert_eq!(resolved.to_lowercase(), expected.to_lowercase());
    assert!(!resolved.to_lowercase().contains(r"\link\"), "{resolved}");
  }

  #[test]
  fn verbatim_prefixes_are_removed() {
    assert_eq!(
      strip_verbatim_prefix(r"\\?\C:\Program Files\HardwareVisualizer"),
      r"C:\Program Files\HardwareVisualizer"
    );
    assert_eq!(
      strip_verbatim_prefix(r"\\?\UNC\server\share\app"),
      r"\\server\share\app"
    );
  }

  #[test]
  fn an_unresolvable_root_never_makes_a_folder_protected() {
    let roots = resolved_roots(&roots(), |root| {
      if root.to_string_lossy().contains("(x86)") {
        Err("not found".to_string())
      } else {
        Ok(root.to_string_lossy().into_owned())
      }
    });

    assert_eq!(roots, vec![r"C:\Program Files".to_string()]);
    assert!(!is_within_any_root(
      r"C:\Program Files (x86)\HardwareVisualizer",
      &roots
    ));
    assert!(is_within_any_root(
      r"C:\Program Files\HardwareVisualizer",
      &roots
    ));
  }

  #[test]
  fn quote_windows_arg_preserves_simple_arguments() {
    assert_eq!(quote_windows_arg(OsStr::new("--flag")), "--flag");
  }

  #[test]
  fn quote_windows_arg_quotes_spaces_and_quotes() {
    assert_eq!(
      quote_windows_arg(OsStr::new(r#"--path=C:\Program Files\"quoted""#)),
      r#""--path=C:\Program Files\\\"quoted\"""#
    );
  }
}
