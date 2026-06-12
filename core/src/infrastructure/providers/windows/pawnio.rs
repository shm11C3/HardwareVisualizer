//! Minimal user-mode client for the PawnIO driver via PawnIOLib (Windows).
//!
//! Implemented clean-room from `docs/specs/sensors/pawnio-interface.md`
//! (rev 2). Facts used here:
//!
//! - PawnIOLib's install location is discovered through the registry
//!   value `InstallLocation` under
//!   `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO`,
//!   with `%ProgramFiles%\PawnIO` as the documented fallback. The
//!   library is loaded dynamically and its functions are resolved by
//!   name; this repository ships none of its (LGPL) code.
//! - The HRESULT-form API: `pawnio_version`, `pawnio_open`,
//!   `pawnio_load`, `pawnio_execute`, `pawnio_close`. Buffer sizes
//!   passed to `pawnio_execute` are counts of 64-bit `ULONG64` cells,
//!   not bytes, and `return_size` is the number of cells written.
//! - One executor handle holds one loaded module; module functions are
//!   addressed by string name (`ioctl_*`).
//! - Absence of PawnIO is a supported state: every failure here surfaces
//!   as an error the caller maps to "unavailable" (→ ACPI fallback).
//!
//! Compiled module blobs (`<Module>.amx`) are not bundled with this
//! repository (LGPL artifacts; packaging happens later). They are read
//! at runtime from a module directory: a process-wide override set via
//! [`set_module_dir`], defaulting to `<exe dir>/pawnio`.

use std::ffi::{CStr, c_char, c_void};
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows::core::PCWSTR;

use crate::log_debug;

/// Registry key (under HKLM) holding the `InstallLocation` value.
const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO";
const INSTALL_LOCATION_VALUE: &str = "InstallLocation";

/// File name of the user-mode library inside the install location.
const PAWNIO_LIB_FILE: &str = "PawnIOLib.dll";

/// Process-wide override for the module-blob directory. Read at session
/// establishment, so setting it before sampling starts (or before the
/// next establish attempt) takes effect.
static MODULE_DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Override the directory the PawnIO module blobs (`*.amx`) are read
/// from. Intended for the app layer to call before sampling starts;
/// without an override the default is `<directory of current exe>/pawnio`.
pub fn set_module_dir(dir: PathBuf) {
  if let Ok(mut guard) = MODULE_DIR_OVERRIDE.lock() {
    *guard = Some(dir);
  }
}

/// Resolve the module-blob directory: the override when set, otherwise
/// `<exe dir>/pawnio`. `None` when neither can be resolved.
pub fn module_dir() -> Option<PathBuf> {
  if let Ok(guard) = MODULE_DIR_OVERRIDE.lock()
    && let Some(dir) = guard.as_ref()
  {
    return Some(dir.clone());
  }
  std::env::current_exe()
    .ok()
    .and_then(|exe| exe.parent().map(|dir| dir.join("pawnio")))
}

/// Read a compiled module blob (`<module>.amx`) from the module
/// directory.
pub fn read_module_blob(module: &str) -> Result<Vec<u8>, PawnIoError> {
  let Some(dir) = module_dir() else {
    return Err(PawnIoError::BlobUnavailable(
      "module directory unresolvable (no override, no executable directory)".to_string(),
    ));
  };
  let path = dir.join(format!("{module}.amx"));
  std::fs::read(&path)
    .map_err(|e| PawnIoError::BlobUnavailable(format!("{}: {e}", path.display())))
}

/// Errors from PawnIO detection, loading, and execution. All of them
/// mean "no native reading this round"; the provider decides between
/// retrying later and tearing the session down.
#[derive(Debug)]
pub enum PawnIoError {
  /// PawnIOLib could not be located or loaded — PawnIO is not installed
  /// (or not where the documented discovery points).
  LibraryUnavailable(String),
  /// The installed PawnIOLib lacks a required export.
  MissingExport(&'static str),
  /// A PawnIOLib call returned a failure HRESULT.
  Call { call: String, hresult: i32 },
  /// `pawnio_execute` succeeded but wrote fewer output cells than the
  /// ioctl contract promises.
  ShortOutput {
    call: String,
    expected: usize,
    written: usize,
  },
  /// The module blob could not be read.
  BlobUnavailable(String),
}

impl fmt::Display for PawnIoError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::LibraryUnavailable(detail) => write!(f, "PawnIOLib unavailable: {detail}"),
      Self::MissingExport(symbol) => write!(f, "PawnIOLib export missing: {symbol}"),
      Self::Call { call, hresult } => {
        write!(f, "{call} failed with HRESULT {hresult:#010X}")
      }
      Self::ShortOutput {
        call,
        expected,
        written,
      } => write!(f, "{call} wrote {written} of {expected} output cells"),
      Self::BlobUnavailable(detail) => {
        write!(f, "PawnIO module blob unavailable: {detail}")
      }
    }
  }
}

// HRESULT-form prototypes from the interface spec, mapped to Rust:
// HRESULT → i32 (failure codes have the high bit set, i.e. negative),
// HANDLE → *mut c_void, PCSTR → *const c_char, SIZE_T/PSIZE_T → usize,
// PULONG → *mut u32, ULONG64 → u64.
type PawnioVersionFn = unsafe extern "system" fn(version: *mut u32) -> i32;
type PawnioOpenFn = unsafe extern "system" fn(handle: *mut *mut c_void) -> i32;
type PawnioLoadFn =
  unsafe extern "system" fn(handle: *mut c_void, blob: *const u8, size: usize) -> i32;
type PawnioExecuteFn = unsafe extern "system" fn(
  handle: *mut c_void,
  name: *const c_char,
  input: *const u64,
  in_size: usize,
  output: *mut u64,
  out_size: usize,
  return_size: *mut usize,
) -> i32;
type PawnioCloseFn = unsafe extern "system" fn(handle: *mut c_void) -> i32;

/// The dynamically loaded PawnIOLib with its exports resolved.
pub struct PawnIo {
  /// Keeps the DLL mapped so the resolved function pointers stay valid.
  _lib: libloading::Library,
  version: Option<PawnioVersionFn>,
  open: PawnioOpenFn,
  load: PawnioLoadFn,
  execute: PawnioExecuteFn,
  close: PawnioCloseFn,
}

impl PawnIo {
  /// Locate and load the installed PawnIOLib (registry `InstallLocation`,
  /// then `%ProgramFiles%\PawnIO`) and resolve the exports this project
  /// uses. Failure means PawnIO is unavailable on this host.
  pub fn load_installed() -> Result<Self, PawnIoError> {
    let dir = installed_location().ok_or_else(|| {
      PawnIoError::LibraryUnavailable(
        "no InstallLocation registry value and no %ProgramFiles%\\PawnIO".to_string(),
      )
    })?;
    let path = dir.join(PAWNIO_LIB_FILE);
    let lib = unsafe { libloading::Library::new(&path) }
      .map_err(|e| PawnIoError::LibraryUnavailable(format!("{}: {e}", path.display())))?;

    // Mandatory exports; `pawnio_version` stays optional (diagnostics only).
    let open = unsafe { lib.get::<PawnioOpenFn>(b"pawnio_open\0") }
      .map(|symbol| *symbol)
      .map_err(|_| PawnIoError::MissingExport("pawnio_open"))?;
    let load = unsafe { lib.get::<PawnioLoadFn>(b"pawnio_load\0") }
      .map(|symbol| *symbol)
      .map_err(|_| PawnIoError::MissingExport("pawnio_load"))?;
    let execute = unsafe { lib.get::<PawnioExecuteFn>(b"pawnio_execute\0") }
      .map(|symbol| *symbol)
      .map_err(|_| PawnIoError::MissingExport("pawnio_execute"))?;
    let close = unsafe { lib.get::<PawnioCloseFn>(b"pawnio_close\0") }
      .map(|symbol| *symbol)
      .map_err(|_| PawnIoError::MissingExport("pawnio_close"))?;
    let version = unsafe { lib.get::<PawnioVersionFn>(b"pawnio_version\0") }
      .ok()
      .map(|symbol| *symbol);

    log_debug!(
      "PawnIOLib loaded",
      "pawnio::PawnIo::load_installed",
      Some(path.display().to_string())
    );

    Ok(Self {
      _lib: lib,
      version,
      open,
      load,
      execute,
      close,
    })
  }

  /// Driver/library version as `major.minor.patch`, decoded from
  /// `version = (major << 16) | (minor << 8) | patch`.
  pub fn version_string(&self) -> Option<String> {
    let version_fn = self.version?;
    let mut raw: u32 = 0;
    let hresult = unsafe { version_fn(&mut raw) };
    (hresult >= 0).then(|| format!("{}.{}.{}", raw >> 16, (raw >> 8) & 0xFF, raw & 0xFF))
  }

  /// Open an executor handle and load one compiled module blob into it
  /// (one handle holds one loaded module).
  pub fn open_module(&self, blob: &[u8]) -> Result<PawnIoModule<'_>, PawnIoError> {
    let mut handle: *mut c_void = std::ptr::null_mut();
    let hresult = unsafe { (self.open)(&mut handle) };
    if hresult < 0 || handle.is_null() {
      return Err(PawnIoError::Call {
        call: "pawnio_open".to_string(),
        hresult,
      });
    }
    let hresult = unsafe { (self.load)(handle, blob.as_ptr(), blob.len()) };
    if hresult < 0 {
      unsafe {
        let _ = (self.close)(handle);
      }
      return Err(PawnIoError::Call {
        call: "pawnio_load".to_string(),
        hresult,
      });
    }
    Ok(PawnIoModule { lib: self, handle })
  }
}

/// One executor handle with one loaded module. Closed on drop.
pub struct PawnIoModule<'lib> {
  lib: &'lib PawnIo,
  handle: *mut c_void,
}

impl PawnIoModule<'_> {
  /// Execute a module function (`ioctl_*`) by name. `input` and the
  /// returned buffer are 64-bit cells; `output_cells` is the cell count
  /// the ioctl contract promises, and a successful call that writes
  /// fewer cells is reported as [`PawnIoError::ShortOutput`].
  pub fn execute(
    &self,
    name: &CStr,
    input: &[u64],
    output_cells: usize,
  ) -> Result<Vec<u64>, PawnIoError> {
    let mut output = vec![0u64; output_cells];
    let mut written: usize = 0;
    let hresult = unsafe {
      (self.lib.execute)(
        self.handle,
        name.as_ptr(),
        input.as_ptr(),
        input.len(),
        output.as_mut_ptr(),
        output.len(),
        &mut written,
      )
    };
    if hresult < 0 {
      return Err(PawnIoError::Call {
        call: format!("pawnio_execute({})", name.to_string_lossy()),
        hresult,
      });
    }
    if written < output_cells {
      return Err(PawnIoError::ShortOutput {
        call: name.to_string_lossy().into_owned(),
        expected: output_cells,
        written,
      });
    }
    Ok(output)
  }
}

impl Drop for PawnIoModule<'_> {
  fn drop(&mut self) {
    unsafe {
      let _ = (self.lib.close)(self.handle);
    }
  }
}

/// Discover the PawnIO install directory: registry first, then the
/// documented `%ProgramFiles%\PawnIO` fallback (only when the directory
/// actually exists, so a clean miss reports as unavailable).
fn installed_location() -> Option<PathBuf> {
  if let Some(dir) = registry_install_location() {
    return Some(dir);
  }
  let fallback = PathBuf::from(std::env::var_os("ProgramFiles")?).join("PawnIO");
  fallback.is_dir().then_some(fallback)
}

/// Read `InstallLocation` (REG_SZ) from the PawnIO uninstall key.
fn registry_install_location() -> Option<PathBuf> {
  let subkey: Vec<u16> = UNINSTALL_KEY
    .encode_utf16()
    .chain(std::iter::once(0))
    .collect();
  let value: Vec<u16> = INSTALL_LOCATION_VALUE
    .encode_utf16()
    .chain(std::iter::once(0))
    .collect();

  // First call sizes the buffer (byte count incl. NUL), second fills it.
  let mut byte_len: u32 = 0;
  let status = unsafe {
    RegGetValueW(
      HKEY_LOCAL_MACHINE,
      PCWSTR(subkey.as_ptr()),
      PCWSTR(value.as_ptr()),
      RRF_RT_REG_SZ,
      None,
      None,
      Some(&mut byte_len),
    )
  };
  if status != ERROR_SUCCESS || byte_len == 0 {
    return None;
  }

  let mut buffer: Vec<u16> = vec![0; (byte_len as usize).div_ceil(2)];
  let mut byte_size = byte_len;
  let status = unsafe {
    RegGetValueW(
      HKEY_LOCAL_MACHINE,
      PCWSTR(subkey.as_ptr()),
      PCWSTR(value.as_ptr()),
      RRF_RT_REG_SZ,
      None,
      Some(buffer.as_mut_ptr().cast::<c_void>()),
      Some(&mut byte_size),
    )
  };
  if status != ERROR_SUCCESS {
    return None;
  }

  let chars = (byte_size as usize) / 2;
  let location = String::from_utf16_lossy(&buffer[..chars.min(buffer.len())])
    .trim_end_matches('\0')
    .trim()
    .to_string();
  (!location.is_empty()).then(|| PathBuf::from(location))
}
