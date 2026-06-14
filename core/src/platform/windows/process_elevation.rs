use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::UI::Shell::{
  IsUserAnAdmin, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

pub fn is_process_elevated() -> Result<bool, String> {
  Ok(unsafe { IsUserAnAdmin().as_bool() })
}

pub fn relaunch_current_process_elevated() -> Result<(), String> {
  let exe_path = std::env::current_exe()
    .map_err(|e| format!("Failed to obtain executable file path: {e}"))?;
  let params = std::env::args_os()
    .skip(1)
    .map(|arg| quote_windows_arg(&arg))
    .collect::<Vec<_>>()
    .join(" ");

  let verb = wide_null("runas");
  let file = os_wide_null(exe_path.as_os_str());
  let parameters = os_wide_null(OsString::from(params).as_os_str());

  let mut execute_info = SHELLEXECUTEINFOW {
    cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
    fMask: SEE_MASK_NOCLOSEPROCESS,
    lpVerb: PCWSTR(verb.as_ptr()),
    lpFile: PCWSTR(file.as_ptr()),
    lpParameters: PCWSTR(parameters.as_ptr()),
    nShow: SW_SHOWNORMAL.0,
    ..Default::default()
  };

  unsafe { ShellExecuteExW(&mut execute_info) }
    .map_err(|e| format!("Failed to restart as administrator: {e}"))?;

  if !execute_info.hProcess.is_invalid() {
    let _ = unsafe { CloseHandle(execute_info.hProcess) };
  }

  Ok(())
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
  use super::quote_windows_arg;
  use std::ffi::OsStr;

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
