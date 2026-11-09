use std::ffi::{CString, c_char};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use libloading::Library;
use windows::Win32::Foundation::{
  CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_EVENT, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::LibraryLoader::{
  LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
};
use windows::Win32::System::Threading::{
  CreateMutexW, MUTEX_MODIFY_STATE, OpenMutexW, ReleaseMutex,
  SYNCHRONIZATION_SYNCHRONIZE, WaitForSingleObject,
};
use windows::core::PCWSTR;

const PAWNIO_DLL_NAME: &str = "PawnIOLib.dll";
const INTEL_MSR_MODULE_FILE: &str = "IntelMSR.bin";
const INTEL_MSR_LEGACY_MODULE_FILE: &str = "IntelMSR.amx";
const RYZEN_SMU_MODULE_FILE: &str = "RyzenSMU.bin";
const RYZEN_SMU_LEGACY_MODULE_FILE: &str = "RyzenSMU.amx";
const AMD_FAMILY17_MODULE_FILE: &str = "AMDFamily17.bin";
const AMD_FAMILY17_LEGACY_MODULE_FILE: &str = "AMDFamily17.amx";
const LPC_IO_MODULE_FILE: &str = "LpcIO.bin";
const LPC_IO_LEGACY_MODULE_FILE: &str = "LpcIO.amx";

pub(crate) const ACCESS_PCI_MUTEX: &str = "Global\\Access_PCI";
pub(crate) const ACCESS_ISABUS_MUTEX: &str = "Global\\Access_ISABUS.HTP.Method";

/// A failed request to a loaded PawnIO module.
///
/// The `Display` text is the surfaced reason: guidance classification and
/// the service log read it, so keep it stable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PawnIoError {
  #[error("invalid PawnIO function name: {0}")]
  InvalidFunctionName(#[from] std::ffi::NulError),
  #[error("pawnio_execute {function} failed: {}", format_hresult(*.hresult))]
  Execute {
    function: &'static str,
    hresult: i32,
  },
  #[error("pawnio_execute {function} returned no output cells")]
  NoOutput { function: &'static str },
  #[error(transparent)]
  Mutex(#[from] MutexError),
}

/// A failure to take an ecosystem mutex shared with other monitoring tools.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum MutexError {
  #[error("failed to open/create mutex {name}: open={open:?}; create={create:?}")]
  OpenOrCreate {
    name: &'static str,
    open: windows::core::Error,
    create: windows::core::Error,
  },
  #[error("timed out waiting for mutex {name}")]
  Timeout { name: &'static str },
  #[error("failed waiting for mutex {name}: {wait:?}")]
  Wait {
    name: &'static str,
    wait: WAIT_EVENT,
  },
}

// Transitional bridges for callers that still carry `String` reasons. They
// produce the same text as before; #2639 removes them once every caller
// keeps the typed error.
impl From<PawnIoError> for String {
  fn from(error: PawnIoError) -> Self {
    error.to_string()
  }
}

impl From<MutexError> for String {
  fn from(error: MutexError) -> Self {
    error.to_string()
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PawnIoModule {
  IntelMsr,
  RyzenSmu,
  AmdFamily17,
  LpcIo,
}

impl PawnIoModule {
  fn file_names(&self) -> &'static [&'static str] {
    match self {
      Self::IntelMsr => &[INTEL_MSR_MODULE_FILE, INTEL_MSR_LEGACY_MODULE_FILE],
      Self::RyzenSmu => &[RYZEN_SMU_MODULE_FILE, RYZEN_SMU_LEGACY_MODULE_FILE],
      Self::AmdFamily17 => &[AMD_FAMILY17_MODULE_FILE, AMD_FAMILY17_LEGACY_MODULE_FILE],
      Self::LpcIo => &[LPC_IO_MODULE_FILE, LPC_IO_LEGACY_MODULE_FILE],
    }
  }

  fn not_found_reason(&self) -> String {
    format!("{} not found", self.file_names().join(" or "))
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PawnIoDiscovery {
  pub install_location: Option<PathBuf>,
  pub dll_path: Option<PathBuf>,
  pub module_path: Option<PathBuf>,
  pub pawnio_available: bool,
  pub library_loadable: bool,
  pub driver_openable: bool,
  pub module_loadable: bool,
  pub version: Option<u32>,
  pub fallback_reason: Option<String>,
}

impl PawnIoDiscovery {
  pub(crate) fn unavailable(reason: impl Into<String>) -> Self {
    Self {
      install_location: None,
      dll_path: None,
      module_path: None,
      pawnio_available: false,
      library_loadable: false,
      driver_openable: false,
      module_loadable: false,
      version: None,
      fallback_reason: Some(reason.into()),
    }
  }

  pub(crate) fn probe_install() -> Self {
    let candidates = pawnio_install_candidates();
    let install_location = candidates.iter().find(|path| path.exists()).cloned();
    let dll_path = candidates
      .iter()
      .find_map(|path| find_named_file(path, PAWNIO_DLL_NAME, 4));
    let pawnio_available = dll_path.is_some();
    let fallback_reason =
      (!pawnio_available).then(|| format!("{PAWNIO_DLL_NAME} not found"));

    Self {
      install_location,
      dll_path,
      module_path: None,
      pawnio_available,
      library_loadable: false,
      driver_openable: false,
      module_loadable: false,
      version: None,
      fallback_reason,
    }
  }
}

pub(crate) struct PawnIoClient {
  library: PawnIoLibrary,
  handle: HANDLE,
}

#[derive(Debug, Clone)]
pub(crate) struct PawnIoInitError {
  pub(crate) discovery: Box<PawnIoDiscovery>,
  pub(crate) reason: String,
}

impl PawnIoClient {
  pub(crate) fn open(
    module: PawnIoModule,
  ) -> Result<(Self, PawnIoDiscovery), PawnIoInitError> {
    let mut discovery = discover_pawnio_files(&module);

    let dll_path = match discovery.dll_path.clone() {
      Some(path) => path,
      None => {
        let reason = format!("{PAWNIO_DLL_NAME} not found");
        discovery.fallback_reason = Some(reason.clone());
        return Err(PawnIoInitError {
          discovery: Box::new(discovery),
          reason,
        });
      }
    };

    let module_path = match discovery.module_path.clone() {
      Some(path) => path,
      None => {
        let reason = module.not_found_reason();
        discovery.fallback_reason = Some(reason.clone());
        return Err(PawnIoInitError {
          discovery: Box::new(discovery),
          reason,
        });
      }
    };

    let library = match PawnIoLibrary::load(&dll_path) {
      Ok(library) => {
        discovery.library_loadable = true;
        library
      }
      Err(reason) => {
        discovery.fallback_reason = Some(reason.clone());
        return Err(PawnIoInitError {
          discovery: Box::new(discovery),
          reason,
        });
      }
    };

    let mut version = 0_u32;
    let hr = unsafe { (library.version)(&mut version) };
    if hresult_succeeded(hr) {
      discovery.version = Some(version);
    }

    let mut handle = HANDLE::default();
    let hr = unsafe { (library.open)(&mut handle) };
    if hresult_failed(hr) || handle.is_invalid() {
      let reason = format!("pawnio_open failed: {}", format_hresult(hr));
      discovery.fallback_reason = Some(reason.clone());
      return Err(PawnIoInitError {
        discovery: Box::new(discovery),
        reason,
      });
    }
    discovery.driver_openable = true;

    let blob = match read_protected_module(&module_path) {
      Ok(blob) => blob,
      Err(e) => {
        let _ = unsafe { (library.close)(handle) };
        let reason = format!("failed to read {}: {e}", module_path.display());
        discovery.fallback_reason = Some(reason.clone());
        return Err(PawnIoInitError {
          discovery: Box::new(discovery),
          reason,
        });
      }
    };

    let hr = unsafe { (library.load)(handle, blob.as_ptr(), blob.len()) };
    if hresult_failed(hr) {
      let _ = unsafe { (library.close)(handle) };
      let reason = format!("pawnio_load failed: {}", format_hresult(hr));
      discovery.fallback_reason = Some(reason.clone());
      return Err(PawnIoInitError {
        discovery: Box::new(discovery),
        reason,
      });
    }
    discovery.module_loadable = true;

    Ok((Self { library, handle }, discovery))
  }

  pub(crate) fn read_msr(&self, msr: u64) -> Result<u64, PawnIoError> {
    self.execute("ioctl_read_msr", &[msr]).map(|out| out[0])
  }

  pub(crate) fn read_smu_register(&self, address: u64) -> Result<u64, PawnIoError> {
    // RyzenSMU does not serialize the host bridge index/data pair for callers.
    let _mutex = NamedMutex::acquire(ACCESS_PCI_MUTEX, Duration::from_millis(50))?;
    self
      .execute("ioctl_read_smu_register", &[address])
      .map(|out| out[0])
  }

  pub(crate) fn select_lpc_slot(&self, slot: u64) -> Result<(), PawnIoError> {
    self.execute_no_output("ioctl_select_slot", &[slot])
  }

  pub(crate) fn find_lpc_bars(&self) -> Result<(), PawnIoError> {
    self.execute_no_output("ioctl_find_bars", &[])
  }

  pub(crate) fn pio_inb(&self, port: u16) -> Result<u8, PawnIoError> {
    self
      .execute("ioctl_pio_inb", &[port as u64])
      .map(|out| out[0] as u8)
  }

  pub(crate) fn pio_outb(&self, port: u16, value: u8) -> Result<(), PawnIoError> {
    self.execute_no_output("ioctl_pio_outb", &[port as u64, value as u64])
  }

  pub(crate) fn superio_inb(&self, register: u8) -> Result<u8, PawnIoError> {
    self
      .execute("ioctl_superio_inb", &[register as u64])
      .map(|out| out[0] as u8)
  }

  pub(crate) fn superio_outb(&self, register: u8, value: u8) -> Result<(), PawnIoError> {
    self.execute_no_output("ioctl_superio_outb", &[register as u64, value as u64])
  }

  fn execute(
    &self,
    function: &'static str,
    input: &[u64],
  ) -> Result<Vec<u64>, PawnIoError> {
    let output = self.execute_raw(function, input, 1)?;
    if output.is_empty() {
      return Err(PawnIoError::NoOutput { function });
    }

    Ok(output)
  }

  fn execute_no_output(
    &self,
    function: &'static str,
    input: &[u64],
  ) -> Result<(), PawnIoError> {
    self.execute_raw(function, input, 0).map(|_| ())
  }

  fn execute_raw(
    &self,
    function: &'static str,
    input: &[u64],
    output_cells: usize,
  ) -> Result<Vec<u64>, PawnIoError> {
    let name = CString::new(function)?;
    // Always back the output pointer with at least one cell so PawnIOLib
    // receives a valid, aligned, non-null pointer even for no-output IOCTLs,
    // but tell it the true cell count (`output_cells`) so a 0-output IOCTL is
    // never told it has space to write.
    let mut output = vec![0_u64; output_cells.max(1)];
    let mut return_size = 0_usize;
    // SAFETY: `self.handle` is returned by `pawnio_open` and kept alive by
    // `PawnIoClient`; `name` is a NUL-terminated `CString` that outlives the
    // call; `input` and `output` pointers reference valid contiguous `u64`
    // buffers for the exact element counts passed to PawnIOLib. For no-output
    // IOCTLs, `output_cells` is 0 but the backing vector still provides a
    // non-null pointer and PawnIOLib receives an output count of 0.
    let hr = unsafe {
      (self.library.execute)(
        self.handle,
        name.as_ptr(),
        input.as_ptr(),
        input.len(),
        output.as_mut_ptr(),
        output_cells,
        &mut return_size,
      )
    };
    if hresult_failed(hr) {
      return Err(PawnIoError::Execute {
        function,
        hresult: hr,
      });
    }

    Ok(output[..return_size.min(output_cells)].to_vec())
  }
}

type SharedIntelMsr =
  Result<(Arc<Mutex<PawnIoClient>>, PawnIoDiscovery), PawnIoInitError>;

static SHARED_INTEL_MSR: Mutex<Option<(Instant, SharedIntelMsr)>> = Mutex::new(None);

/// Open the IntelMSR module once and share its executor handle across CPU
/// temperature and power providers.
pub(crate) fn open_shared_intel_msr() -> SharedIntelMsr {
  let mut shared = SHARED_INTEL_MSR.lock().unwrap_or_else(|p| p.into_inner());

  if shared
    .as_ref()
    .is_some_and(|(at, value)| value.is_err() && at.elapsed() >= Duration::from_secs(30))
  {
    *shared = None;
  }

  shared
    .get_or_insert_with(|| {
      (
        Instant::now(),
        PawnIoClient::open(PawnIoModule::IntelMsr)
          .map(|(client, discovery)| (Arc::new(Mutex::new(client)), discovery)),
      )
    })
    .1
    .clone()
}

/// Called only after the service's temperature/power owners have been dropped.
/// A later demand must reprobe repaired components rather than cache a failure
/// or retain an executor forever while no application needs readings.
pub(crate) fn release_shared_intel_msr() {
  *SHARED_INTEL_MSR.lock().unwrap_or_else(|p| p.into_inner()) = None;
}

impl Drop for PawnIoClient {
  fn drop(&mut self) {
    let _ = unsafe { (self.library.close)(self.handle) };
  }
}

// SAFETY: the PawnIO handle is owned by `PawnIoClient` and closed exactly once
// in `Drop`. Callers serialize stateful module transactions with the
// appropriate ecosystem mutex (`Access_PCI` for SMN, `Access_ISABUS` for
// Super I/O). The DLL remains loaded for the full handle lifetime through
// `PawnIoLibrary`.
unsafe impl Send for PawnIoClient {}

type PawnIoVersion = unsafe extern "system" fn(*mut u32) -> i32;
type PawnIoOpen = unsafe extern "system" fn(*mut HANDLE) -> i32;
type PawnIoLoad = unsafe extern "system" fn(HANDLE, *const u8, usize) -> i32;
type PawnIoExecute = unsafe extern "system" fn(
  HANDLE,
  *const c_char,
  *const u64,
  usize,
  *mut u64,
  usize,
  *mut usize,
) -> i32;
type PawnIoClose = unsafe extern "system" fn(HANDLE) -> i32;

struct PawnIoLibrary {
  _library: Library,
  _file: std::fs::File,
  version: PawnIoVersion,
  open: PawnIoOpen,
  load: PawnIoLoad,
  execute: PawnIoExecute,
  close: PawnIoClose,
}

impl PawnIoLibrary {
  fn load(path: &Path) -> Result<Self, String> {
    let (resolved, file) =
      crate::sensor_service::windows::security::protected_file(path)?;

    let library: Library = unsafe {
      libloading::os::windows::Library::load_with_flags(
        &resolved,
        (LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32).0,
      )
    }
    .map(Into::into)
    .map_err(|e| format!("failed to load {}: {e}", resolved.display()))?;
    let version = *unsafe { library.get::<PawnIoVersion>(b"pawnio_version") }
      .map_err(|e| format!("missing pawnio_version: {e}"))?;
    let open = *unsafe { library.get::<PawnIoOpen>(b"pawnio_open") }
      .map_err(|e| format!("missing pawnio_open: {e}"))?;
    let load = *unsafe { library.get::<PawnIoLoad>(b"pawnio_load") }
      .map_err(|e| format!("missing pawnio_load: {e}"))?;
    let execute = *unsafe { library.get::<PawnIoExecute>(b"pawnio_execute") }
      .map_err(|e| format!("missing pawnio_execute: {e}"))?;
    let close = *unsafe { library.get::<PawnIoClose>(b"pawnio_close") }
      .map_err(|e| format!("missing pawnio_close: {e}"))?;

    Ok(Self {
      _library: library,
      _file: file,
      version,
      open,
      load,
      execute,
      close,
    })
  }
}

fn read_protected_module(path: &Path) -> Result<Vec<u8>, String> {
  if path.extension().and_then(|v| v.to_str()) != Some("bin") {
    return Err("The sensor service requires signed .bin modules".into());
  }

  let (_, mut file) = crate::sensor_service::windows::security::protected_file(path)?;

  if file.metadata().map_err(|e| e.to_string())?.len() > 1_048_576 {
    return Err("PawnIO module exceeds size limit".into());
  }

  let mut blob = Vec::new();
  file.read_to_end(&mut blob).map_err(|e| e.to_string())?;

  // An unrestricted driver would accept an unsigned blob renamed to .bin.
  // Only known signed release bytes may enter this SYSTEM process, even when
  // another application selected that unrestricted driver. Unknown user files
  // are retained, never replaced by a collection or repair attempt.
  let plan =
    crate::external_component_setup::setup_plan(crate::models::ExternalComponent::Pawnio)
      .ok_or("PawnIO release catalog unavailable")?;

  let name = path
    .file_name()
    .and_then(|v| v.to_str())
    .ok_or("Invalid PawnIO module name")?;

  let known = plan
    .file_bundle
    .files
    .iter()
    .find(|file| file.file_name.eq_ignore_ascii_case(name))
    .ok_or("Unknown PawnIO module")?;

  if known.classify(&blob)
    == crate::external_component_setup::ModuleFileCondition::Unrecognized
  {
    return Err(
      "Unrecognized PawnIO module: use a compatible signed release; \
       the existing file was kept"
        .into(),
    );
  }

  Ok(blob)
}

pub(crate) struct NamedMutex {
  handle: HANDLE,
  acquired: bool,
}

impl NamedMutex {
  pub(crate) fn acquire(
    name: &'static str,
    timeout: Duration,
  ) -> Result<Self, MutexError> {
    let wide_name = wide_null(name);
    let handle = open_or_create_mutex(name, PCWSTR(wide_name.as_ptr()))?;
    let wait = unsafe { WaitForSingleObject(handle, timeout.as_millis() as u32) };
    if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
      Ok(Self {
        handle,
        acquired: true,
      })
    } else {
      let _ = unsafe { CloseHandle(handle) };
      if wait == WAIT_TIMEOUT {
        Err(MutexError::Timeout { name })
      } else {
        Err(MutexError::Wait { name, wait })
      }
    }
  }
}

impl Drop for NamedMutex {
  fn drop(&mut self) {
    if self.acquired {
      let _ = unsafe { ReleaseMutex(self.handle) };
    }
    let _ = unsafe { CloseHandle(self.handle) };
  }
}

fn open_or_create_mutex(
  name: &'static str,
  wide_name: PCWSTR,
) -> Result<HANDLE, MutexError> {
  let access = MUTEX_MODIFY_STATE | SYNCHRONIZATION_SYNCHRONIZE;
  match unsafe { OpenMutexW(access, false, wide_name) } {
    Ok(handle) => Ok(handle),
    Err(open) => unsafe { CreateMutexW(None, false, wide_name) }
      .map_err(|create| MutexError::OpenOrCreate { name, open, create }),
  }
}

fn discover_pawnio_files(module: &PawnIoModule) -> PawnIoDiscovery {
  let mut discovery = PawnIoDiscovery::probe_install();
  let candidates = pawnio_install_candidates();
  let module_path = find_first_module_file(&candidates, module.file_names(), 4);

  discovery.module_path = module_path;
  if discovery.pawnio_available && discovery.module_path.is_none() {
    discovery.fallback_reason = Some(module.not_found_reason());
  }
  discovery
}

fn pawnio_install_candidates() -> Vec<PathBuf> {
  let mut candidates = Vec::new();
  if let Some(registry_path) = pawnio_install_location_from_registry() {
    candidates.push(registry_path);
  }
  for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
    if let Some(value) = std::env::var_os(var) {
      let path = PathBuf::from(value).join("PawnIO");
      if !candidates.iter().any(|candidate| candidate == &path) {
        candidates.push(path);
      }
    }
  }
  candidates
}

fn pawnio_install_location_from_registry() -> Option<PathBuf> {
  let output = Command::new("reg.exe")
    .args([
      "query",
      r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO",
      "/v",
      "InstallLocation",
    ])
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }

  String::from_utf8_lossy(&output.stdout)
    .lines()
    .find_map(|line| {
      let index = line.find("REG_SZ")?;
      let value = line[index + "REG_SZ".len()..].trim();
      (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

fn find_first_module_file(
  roots: &[PathBuf],
  file_names: &[&str],
  max_depth: usize,
) -> Option<PathBuf> {
  file_names.iter().find_map(|file_name| {
    roots
      .iter()
      .find_map(|root| find_named_file(root, file_name, max_depth))
  })
}

fn find_named_file(root: &Path, file_name: &str, max_depth: usize) -> Option<PathBuf> {
  if max_depth == 0 || !root.exists() {
    return None;
  }

  let direct = root.join(file_name);
  if direct.is_file() {
    return Some(direct);
  }

  let entries = fs::read_dir(root).ok()?;
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_file()
      && path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case(file_name))
        .unwrap_or(false)
    {
      return Some(path);
    }
    if path.is_dir()
      && let Some(found) = find_named_file(&path, file_name, max_depth - 1)
    {
      return Some(found);
    }
  }

  None
}

fn hresult_succeeded(hr: i32) -> bool {
  hr >= 0
}

fn hresult_failed(hr: i32) -> bool {
  !hresult_succeeded(hr)
}

fn format_hresult(hr: i32) -> String {
  format!("0x{:08X}", hr as u32)
}

fn wide_null(value: &str) -> Vec<u16> {
  value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn production_signed_modules_are_preferred() {
    assert_eq!(
      PawnIoModule::IntelMsr.file_names(),
      ["IntelMSR.bin", "IntelMSR.amx"]
    );
    assert_eq!(
      PawnIoModule::RyzenSmu.file_names(),
      ["RyzenSMU.bin", "RyzenSMU.amx"]
    );
    assert_eq!(
      PawnIoModule::AmdFamily17.file_names(),
      ["AMDFamily17.bin", "AMDFamily17.amx"]
    );
    assert_eq!(PawnIoModule::LpcIo.file_names(), ["LpcIO.bin", "LpcIO.amx"]);
  }

  #[test]
  fn execute_failure_keeps_the_surfaced_reason_and_its_guidance_kind() {
    let error = PawnIoError::Execute {
      function: "ioctl_pio_inb",
      hresult: 0x8007_0005_u32 as i32,
    };

    assert_eq!(
      error.to_string(),
      "pawnio_execute ioctl_pio_inb failed: 0x80070005"
    );
    assert_eq!(
      crate::models::external_component_guidance::classify_external_component_reason(
        &error.to_string()
      ),
      crate::models::ExternalComponentReasonKind::Permission
    );
  }

  #[test]
  fn missing_output_names_the_function() {
    assert_eq!(
      PawnIoError::NoOutput {
        function: "ioctl_read_msr"
      }
      .to_string(),
      "pawnio_execute ioctl_read_msr returned no output cells"
    );
  }

  #[test]
  fn mutex_failures_keep_their_text_through_the_client_error() {
    let timeout = MutexError::Timeout {
      name: ACCESS_ISABUS_MUTEX,
    };
    let wait = MutexError::Wait {
      name: ACCESS_PCI_MUTEX,
      wait: WAIT_EVENT(u32::MAX),
    };

    assert_eq!(
      PawnIoError::from(timeout).to_string(),
      "timed out waiting for mutex Global\\Access_ISABUS.HTP.Method"
    );
    assert_eq!(
      wait.to_string(),
      "failed waiting for mutex Global\\Access_PCI: WAIT_EVENT(4294967295)"
    );
  }
}
