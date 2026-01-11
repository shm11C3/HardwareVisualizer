use std::ffi::CString;

/// Reads a `u64` value via `sysctlbyname`.
///
/// This is a low-level macOS OS API access helper.
pub fn sysctl_u64(name: &str) -> Result<u64, String> {
  let cname =
    CString::new(name).map_err(|e| format!("invalid sysctl name '{name}': {e}"))?;

  let mut value: u64 = 0;
  let mut len: libc::size_t = std::mem::size_of::<u64>() as libc::size_t;

  let ret = unsafe {
    libc::sysctlbyname(
      cname.as_ptr(),
      (&mut value as *mut u64).cast::<libc::c_void>(),
      &mut len as *mut libc::size_t,
      std::ptr::null_mut(),
      0,
    )
  };

  if ret != 0 {
    return Err(format!(
      "sysctlbyname({name}) failed: {}",
      std::io::Error::last_os_error()
    ));
  }

  if len as usize != std::mem::size_of::<u64>() {
    return Err(format!(
      "sysctlbyname({name}) returned unexpected size: {len}"
    ));
  }

  Ok(value)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  #[test]
  fn hw_memsize_is_positive() {
    let bytes = sysctl_u64("hw.memsize").expect("sysctl hw.memsize should succeed");
    assert!(bytes > 0);
  }
}
