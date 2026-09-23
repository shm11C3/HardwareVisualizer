use crate::enums::error::PlatformError;
use crate::log_warn;
use crate::platform::traits::{ElevatedProcessRun, ElevationAvailability};
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE};
use windows::Win32::Storage::FileSystem::{
  CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES,
  FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GETFINALPATHNAMEBYHANDLE_FLAGS,
  GetFinalPathNameByHandleW, OPEN_EXISTING, VOLUME_NAME_DOS,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Threading::{
  GetExitCodeProcess, INFINITE, WaitForSingleObject,
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
  match current_executable_is_under_program_files() {
    Ok(true) => ElevationAvailability::Available,
    Ok(false) => ElevationAvailability::UnprotectedLocation,
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

fn current_executable_is_under_program_files() -> Result<bool, String> {
  let exe_path = std::env::current_exe()
    .map_err(|e| format!("Failed to obtain executable file path: {e}"))?;
  let exe_dir = exe_path
    .parent()
    .ok_or_else(|| "The executable path has no parent folder".to_string())?;
  let exe_dir = final_path(exe_dir)?;

  let mut roots = Vec::new();
  for folder in [&FOLDERID_ProgramFiles, &FOLDERID_ProgramFilesX86] {
    let root = known_folder_path(folder)?;
    roots.push(final_path(Path::new(&root)).unwrap_or(root));
  }

  Ok(is_within_any_root(&exe_dir, &roots))
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

pub fn relaunch_current_process_elevated() -> Result<(), PlatformError> {
  let args = std::env::args_os().skip(1).collect::<Vec<_>>();
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

/// Launch the current executable elevated with `args`, wait for it to exit,
/// and return its exit code. A declined UAC prompt is reported as
/// [`ElevatedProcessRun::Declined`], not as an error.
pub fn run_current_executable_elevated(
  args: &[String],
) -> Result<ElevatedProcessRun, PlatformError> {
  let args = args.iter().map(OsString::from).collect::<Vec<_>>();
  let Some(process) = launch_current_executable_elevated(&args, "run as administrator")?
  else {
    return Ok(ElevatedProcessRun::Declined);
  };

  let exit_code = wait_for_exit_code(process);
  let _ = unsafe { CloseHandle(process) };
  Ok(ElevatedProcessRun::Exited { exit_code })
}

/// Returns the process handle, or `None` when the user declined the prompt.
fn launch_current_executable_elevated(
  args: &[OsString],
  action: &str,
) -> Result<Option<HANDLE>, PlatformError> {
  if elevation_availability() != ElevationAvailability::Available {
    return Err(PlatformError::unavailable(format!(
      "Refusing to {action}: HardwareVisualizer is not installed under Program Files, \
       so its executable could have been replaced."
    )));
  }

  let exe_path = std::env::current_exe().map_err(|e| {
    PlatformError::fault(format!("Failed to obtain executable file path: {e}"))
  })?;
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

fn wait_for_exit_code(process: HANDLE) -> Option<i32> {
  let _ = unsafe { WaitForSingleObject(process, INFINITE) };
  let mut exit_code: u32 = 0;
  unsafe { GetExitCodeProcess(process, &mut exit_code) }
    .ok()
    .map(|()| exit_code as i32)
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
  use super::{is_within_any_root, quote_windows_arg, strip_verbatim_prefix};
  use std::ffi::OsStr;

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
    assert!(super::relaunch_current_process_elevated().is_err());
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
