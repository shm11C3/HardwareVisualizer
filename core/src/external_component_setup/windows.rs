//! Windows execution of an External Component Setup plan.
//!
//! This module runs inside an already elevated process: the App launches the
//! executable elevated in its setup command-line mode, or the installer's
//! custom action calls it from its elevated context. It downloads the pinned
//! artifacts, verifies them, runs the runtime installer unattended, and places
//! only the module files that are missing. It never overwrites or removes
//! anything, and it reports back through the process exit code only.
//!
//! Elevation-boundary rules this file keeps:
//!
//! - The verified installer is staged in an administrator-only directory
//!   under `%SystemRoot%\Temp`, not in the user's temp directory, and is held
//!   open with an exclusive share mode while it runs, so a same-user
//!   medium-integrity process cannot swap it between verification and start.
//! - Module files are published atomically with no-clobber semantics, so a
//!   partial file never counts as present and a concurrent writer is never
//!   truncated.
//! - Enumeration failures are reported as unknown state, never as absence.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use windows::Win32::Foundation::{
  CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HANDLE, HLOCAL, LocalFree,
  WAIT_OBJECT_0,
};
use windows::Win32::Security::Authorization::{
  ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::{CreateDirectoryW, FILE_SHARE_READ};
use windows::Win32::System::Diagnostics::ToolHelp::{
  CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
  TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Registry::{
  HKEY, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, REG_SZ, REG_VALUE_TYPE,
  RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
};
use windows::Win32::System::Threading::{
  OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
  QueryFullProcessImageNameW, WaitForSingleObject,
};
use windows::core::{PCWSTR, PWSTR};

use super::{
  ExternalComponentSetupOutcome, ExternalComponentSetupPlan,
  ExternalComponentSetupResult, ExternalComponentSetupStatus,
  ExternalComponentSetupSupport, FileBundleStep, InstallerExitOutcome, ModuleFileState,
  PinnedArtifact, RuntimeInstallState, SetupFailureStage, interpret_installer_exit_code,
  select_bundle_entries, verify_artifact,
};
use crate::models::ExternalComponent;
use crate::{log_info, log_warn};

/// Uninstall registry key the PawnIO installer registers; `InstallLocation`
/// is the documented discovery path (`docs/specs/sensors/pawnio-interface.md`).
const PAWNIO_UNINSTALL_SUBKEY: &str =
  r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO";
const PAWNIO_DIRECTORY_NAME: &str = "PawnIO";
/// Matches the provider's recursive module search depth so "present" here
/// agrees with what collection would find.
const MODULE_SEARCH_DEPTH: usize = 4;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
/// How long the runtime installer may run before it is terminated. A normal
/// unattended run finishes in seconds (driver registration and a few file
/// copies), so five minutes is headroom for a slow disk or a busy Windows
/// Installer service, not a working budget. A run that lasts longer is a hung
/// installer, typically a dialog raised despite `-silent` that nobody can
/// answer in session 0 when the MSI custom action runs it, and waiting on it
/// would otherwise block the product install or the Settings action forever.
const INSTALLER_TIMEOUT: Duration = Duration::from_secs(300);
/// How long [`run_bounded`] waits for a terminated installer to actually end.
/// `TerminateProcess` only requests termination; the process object is
/// signaled once the kernel has torn the process down, normally within
/// milliseconds, but pending I/O can delay it. A child still alive after this
/// is reported as unconfirmed rather than waited on without a limit, so the
/// installer bound keeps bounding the MSI custom action and the Settings task.
const TERMINATION_CONFIRM_TIMEOUT: Duration = Duration::from_secs(30);
/// Protected DACL: full control for Administrators and SYSTEM, nothing for
/// anyone else, no inheritance from `%SystemRoot%\Temp`.
const STAGING_DIRECTORY_SDDL: &str = "D:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)";
/// Name prefix of every staging directory this module creates under
/// `%SystemRoot%\Temp`; a random suffix follows. A process whose executable
/// lives under such a directory is an installer a previous run left behind.
const STAGING_DIRECTORY_PREFIX: &str = "hardviz-external-component-setup-";
const PARTIAL_SUFFIX: &str = ".hardviz-partial";

pub fn status(plan: &ExternalComponentSetupPlan) -> ExternalComponentSetupStatus {
  let runtime = runtime_install_state(plan.component);
  let roots = install_roots(&runtime);
  let mut enumeration_error = None;
  let module_files = plan
    .file_bundle
    .file_names
    .iter()
    .map(|file_name| {
      let mut present = false;
      for root in &roots {
        match find_named_file(root, file_name, MODULE_SEARCH_DEPTH) {
          Ok(Some(_)) => {
            present = true;
            break;
          }
          Ok(None) => {}
          Err(e) => {
            enumeration_error.get_or_insert_with(|| format!("{}: {e}", root.display()));
          }
        }
      }
      ModuleFileState {
        file_name: (*file_name).to_string(),
        present,
      }
    })
    .collect();

  ExternalComponentSetupStatus {
    component: plan.component,
    support: ExternalComponentSetupSupport::Supported,
    runtime,
    module_files,
    enumeration_error,
    pinned_runtime_version: plan.installer.artifact.version.to_string(),
    pinned_modules_version: plan.file_bundle.artifact.version.to_string(),
  }
}

pub fn run(plan: &ExternalComponentSetupPlan) -> ExternalComponentSetupResult {
  let mut result = ExternalComponentSetupResult {
    component: plan.component,
    outcome: ExternalComponentSetupOutcome::AlreadyInstalled,
    runtime_installed: false,
    module_files_placed: Vec::new(),
  };

  let before = status(plan);
  if let Some(blocker) = before.setup_blocker() {
    result.outcome =
      ExternalComponentSetupOutcome::failed(SetupFailureStage::StateUnknown, blocker);
    return result;
  }
  if before.is_complete() {
    return result;
  }

  let mut reboot_required = false;
  if matches!(before.runtime, RuntimeInstallState::NotInstalled) {
    // A previous run may have abandoned its staging directory with the
    // installer still executing from it (exit code 24); an app restart
    // clears the app's in-flight mark but not that process, so it is looked
    // for here, before anything is created or started.
    if let Some(detail) = staged_installer_still_running() {
      result.outcome = ExternalComponentSetupOutcome::failed(
        SetupFailureStage::InstallerStillRunning,
        detail,
      );
      return result;
    }
    let mut staging = match StagingDirectory::create() {
      Ok(staging) => staging,
      Err(detail) => {
        result.outcome = ExternalComponentSetupOutcome::failed(
          SetupFailureStage::StagingDirectory,
          detail,
        );
        return result;
      }
    };
    match install_runtime(plan, &staging.path) {
      Ok(InstallerExitOutcome::Installed) => result.runtime_installed = true,
      Ok(InstallerExitOutcome::RebootRequired) => {
        result.runtime_installed = true;
        reboot_required = true;
      }
      Ok(InstallerExitOutcome::Failed(code)) => {
        result.outcome = ExternalComponentSetupOutcome::failed(
          SetupFailureStage::InstallerExit,
          format!(
            "{} exited with {}",
            plan.installer.artifact.file_name,
            code.map_or_else(|| "no exit code".to_string(), |code| code.to_string())
          ),
        );
        return result;
      }
      Err((stage, detail)) => {
        if stage == SetupFailureStage::InstallerStillRunning {
          // The installer may still be executing from the staged file;
          // removing the directory under it is not safe, so leave it.
          staging.abandon();
        }
        result.outcome = ExternalComponentSetupOutcome::failed(stage, detail);
        return result;
      }
    }
  }

  let after = status(plan);
  if let Some(blocker) = after.setup_blocker() {
    result.outcome =
      ExternalComponentSetupOutcome::failed(SetupFailureStage::StateUnknown, blocker);
    return result;
  }
  let missing = after.missing_module_files();
  if !missing.is_empty() {
    let destination = module_destination(&after.runtime);
    match place_module_files(&plan.file_bundle, &missing, &destination) {
      Ok(placed) => result.module_files_placed = placed,
      Err((stage, detail)) => {
        result.outcome = ExternalComponentSetupOutcome::failed(stage, detail);
        return result;
      }
    }
  }

  // Do not report success on the strength of the steps alone; the state after
  // the run is the only evidence the caller can act on.
  let final_state = status(plan);
  if !final_state.is_complete() && !reboot_required {
    let detail = final_state.setup_blocker().unwrap_or_else(|| {
      format!(
        "runtime {} and {} module file(s) still missing",
        match final_state.runtime {
          RuntimeInstallState::Installed { .. } => "installed",
          _ => "not registered",
        },
        final_state.missing_module_files().len()
      )
    });
    result.outcome =
      ExternalComponentSetupOutcome::failed(SetupFailureStage::Incomplete, detail);
    return result;
  }

  result.outcome = if reboot_required {
    ExternalComponentSetupOutcome::RebootRequired
  } else {
    ExternalComponentSetupOutcome::Installed
  };
  result
}

fn runtime_install_state(component: ExternalComponent) -> RuntimeInstallState {
  let subkey = match component {
    ExternalComponent::Pawnio => PAWNIO_UNINSTALL_SUBKEY,
    ExternalComponent::Smartctl => return RuntimeInstallState::NotInstalled,
  };

  let subkey_w = wide_null(subkey);
  let mut key = HKEY::default();
  let opened = unsafe {
    RegOpenKeyExW(
      HKEY_LOCAL_MACHINE,
      PCWSTR(subkey_w.as_ptr()),
      Some(0),
      KEY_READ | KEY_WOW64_64KEY,
      &mut key,
    )
  };
  if opened == ERROR_FILE_NOT_FOUND {
    return RuntimeInstallState::NotInstalled;
  }
  if opened != ERROR_SUCCESS {
    return RuntimeInstallState::Unknown {
      detail: format!("RegOpenKeyExW({subkey}) failed with {}", opened.0),
    };
  }

  let version = read_registry_string(key, "DisplayVersion");
  let install_location = read_registry_string(key, "InstallLocation").map(PathBuf::from);
  let _ = unsafe { RegCloseKey(key) };

  RuntimeInstallState::Installed {
    version,
    install_location,
  }
}

fn read_registry_string(key: HKEY, value_name: &str) -> Option<String> {
  let name_w = wide_null(value_name);
  let mut kind = REG_VALUE_TYPE::default();
  let mut size: u32 = 0;
  let probed = unsafe {
    RegQueryValueExW(
      key,
      PCWSTR(name_w.as_ptr()),
      None,
      Some(&mut kind),
      None,
      Some(&mut size),
    )
  };
  if probed != ERROR_SUCCESS || kind != REG_SZ || size == 0 {
    return None;
  }

  let mut buffer = vec![0u8; size as usize];
  let read = unsafe {
    RegQueryValueExW(
      key,
      PCWSTR(name_w.as_ptr()),
      None,
      Some(&mut kind),
      Some(buffer.as_mut_ptr()),
      Some(&mut size),
    )
  };
  if read != ERROR_SUCCESS {
    return None;
  }

  let (pairs, _) = buffer[..size as usize].as_chunks::<2>();
  let units = pairs
    .iter()
    .map(|pair| u16::from_le_bytes(*pair))
    .collect::<Vec<_>>();
  let end = units
    .iter()
    .position(|&unit| unit == 0)
    .unwrap_or(units.len());
  let value = String::from_utf16_lossy(&units[..end]);
  let value = value.trim();
  (!value.is_empty()).then(|| value.to_string())
}

/// Every directory where module files count as present, mirroring the
/// provider's candidate order.
fn install_roots(runtime: &RuntimeInstallState) -> Vec<PathBuf> {
  let mut roots = Vec::new();
  if let RuntimeInstallState::Installed {
    install_location: Some(location),
    ..
  } = runtime
  {
    roots.push(location.clone());
  }
  for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
    if let Some(value) = std::env::var_os(var) {
      let path = PathBuf::from(value).join(PAWNIO_DIRECTORY_NAME);
      if !roots.iter().any(|root| root == &path) {
        roots.push(path);
      }
    }
  }
  roots
}

/// Where new module files go: the registered install location, otherwise the
/// documented `%ProgramFiles%\PawnIO` fallback.
fn module_destination(runtime: &RuntimeInstallState) -> PathBuf {
  if let RuntimeInstallState::Installed {
    install_location: Some(location),
    ..
  } = runtime
  {
    return location.clone();
  }

  std::env::var_os("ProgramFiles")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
    .join(PAWNIO_DIRECTORY_NAME)
}

/// An administrator-only directory under `%SystemRoot%\Temp`, removed on
/// drop. `%SystemRoot%\Temp` lets standard users create entries but not list,
/// delete, or rename them, and the protected DACL keeps the contents
/// unreadable and unwritable for medium-integrity processes.
struct StagingDirectory {
  path: PathBuf,
  /// Set by [`Self::abandon`]: the directory is left in place on drop.
  abandoned: bool,
}

impl StagingDirectory {
  /// Leave the directory behind instead of removing it on drop, because a
  /// process may still be running from a file inside it. It stays under
  /// `%SystemRoot%\Temp` with the administrator-only DACL; the path is
  /// logged so an administrator can remove it once the process has ended.
  fn abandon(&mut self) {
    self.abandoned = true;
    log_warn!(
      format!(
        "leaving the staging directory {} in place; a process may still be running \
         from it",
        self.path.display()
      ),
      "external_component_setup::StagingDirectory::abandon",
      None::<&str>
    );
  }

  fn create() -> Result<Self, String> {
    let name = format!("{STAGING_DIRECTORY_PREFIX}{}", random_hex::<16>()?);
    let path = staging_root()?.join(name);

    let sddl = wide_null(STAGING_DIRECTORY_SDDL);
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
    let path_w = os_wide_null(path.as_os_str());
    let created = unsafe { CreateDirectoryW(PCWSTR(path_w.as_ptr()), Some(&attributes)) };
    let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    created.map_err(|e| format!("failed to create {}: {e}", path.display()))?;

    Ok(Self {
      path,
      abandoned: false,
    })
  }
}

/// `%SystemRoot%\Temp`, the parent of every staging directory.
fn staging_root() -> Result<PathBuf, String> {
  let system_root =
    std::env::var_os("SystemRoot").ok_or_else(|| "SystemRoot is not set".to_string())?;
  Ok(PathBuf::from(system_root).join("Temp"))
}

/// A detail naming the first process still executing from one of this
/// module's staging directories, or `None` when there is none. A staging
/// root that cannot be determined is treated as no process found: the
/// following `StagingDirectory::create` reports that failure precisely.
fn staged_installer_still_running() -> Option<String> {
  let root = staging_root().ok()?;
  let (pid, path) =
    first_process_under_staging(&root, STAGING_DIRECTORY_PREFIX, process_image_paths())?;
  Some(format!(
    "a runtime installer from a previous run is still running (pid {pid}, {}); wait \
     for it to end before trying again",
    path.display()
  ))
}

/// The first of `processes` whose image path lies under `root` in a directory
/// whose name starts with `prefix`. Components are compared
/// case-insensitively and on component boundaries, so `root` itself, a
/// sibling whose name merely starts with `root`'s, and a file directly in
/// `root` do not match.
fn first_process_under_staging(
  root: &Path,
  prefix: &str,
  processes: impl IntoIterator<Item = (u32, PathBuf)>,
) -> Option<(u32, PathBuf)> {
  let root: Vec<String> = root
    .components()
    .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
    .collect();
  let prefix = prefix.to_lowercase();
  processes.into_iter().find(|(_, path)| {
    let mut components = path
      .components()
      .map(|component| component.as_os_str().to_string_lossy().to_lowercase());
    let under_root = root
      .iter()
      .all(|expected| components.next().as_ref() == Some(expected));
    let Some(directory) = components.next() else {
      return false;
    };
    // The staged file sits directly in the staging directory, so exactly one
    // component follows it.
    under_root
      && directory.starts_with(&prefix)
      && components.next().is_some()
      && components.next().is_none()
  })
}

/// `(pid, full image path)` of every process this process may query. Ones
/// that cannot be opened or whose path cannot be read are skipped: they are
/// system or other-user processes, never our staged installer, which runs
/// as an administrator like the caller.
fn process_image_paths() -> Vec<(u32, PathBuf)> {
  let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
    return Vec::new();
  };
  let mut entries = Vec::new();
  let mut entry = PROCESSENTRY32W {
    dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
    ..Default::default()
  };
  if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
    loop {
      if let Some(path) = process_image_path(entry.th32ProcessID) {
        entries.push((entry.th32ProcessID, path));
      }
      if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
        break;
      }
    }
  }
  let _ = unsafe { CloseHandle(snapshot) };
  entries
}

fn process_image_path(pid: u32) -> Option<PathBuf> {
  let process =
    unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
  let mut buffer = vec![0u16; 32_768];
  let mut length = buffer.len() as u32;
  let queried = unsafe {
    QueryFullProcessImageNameW(
      process,
      PROCESS_NAME_WIN32,
      PWSTR(buffer.as_mut_ptr()),
      &mut length,
    )
  };
  let _ = unsafe { CloseHandle(process) };
  queried.ok()?;
  Some(PathBuf::from(OsString::from_wide(
    &buffer[..length as usize],
  )))
}

impl Drop for StagingDirectory {
  fn drop(&mut self) {
    if self.abandoned {
      return;
    }
    // Errors are ignored: a leftover directory is not a setup failure, and a
    // drop must never panic.
    let _ = fs::remove_dir_all(&self.path);
  }
}

fn install_runtime(
  plan: &ExternalComponentSetupPlan,
  staging: &Path,
) -> Result<InstallerExitOutcome, (SetupFailureStage, String)> {
  let artifact = &plan.installer.artifact;
  let bytes = download_verified(artifact).map_err(|(stage, detail)| {
    (
      match stage {
        DownloadFailure::Transfer => SetupFailureStage::DownloadRuntime,
        DownloadFailure::Verification => SetupFailureStage::VerifyRuntime,
      },
      detail,
    )
  })?;
  let installer_path = staging.join(artifact.file_name);

  // Create exclusively, write, then hold a read handle that denies write and
  // delete sharing for as long as the installer runs. The loader opens the
  // image for read/execute, which this share mode allows.
  {
    let mut file = fs::OpenOptions::new()
      .write(true)
      .create_new(true)
      .share_mode(0)
      .open(&installer_path)
      .map_err(|e| {
        (
          SetupFailureStage::StagingDirectory,
          format!("failed to create {}: {e}", installer_path.display()),
        )
      })?;
    file
      .write_all(&bytes)
      .and_then(|()| file.sync_all())
      .map_err(|e| {
        (
          SetupFailureStage::StagingDirectory,
          format!("failed to write {}: {e}", installer_path.display()),
        )
      })?;
  }
  let mut guard = fs::OpenOptions::new()
    .read(true)
    .share_mode(FILE_SHARE_READ.0)
    .open(&installer_path)
    .map_err(|e| {
      (
        SetupFailureStage::StagingDirectory,
        format!("failed to reopen {}: {e}", installer_path.display()),
      )
    })?;
  let mut staged = Vec::with_capacity(bytes.len());
  guard.read_to_end(&mut staged).map_err(|e| {
    (
      SetupFailureStage::VerifyRuntime,
      format!("failed to read back {}: {e}", installer_path.display()),
    )
  })?;
  verify_artifact(&staged, artifact)
    .map_err(|detail| (SetupFailureStage::VerifyRuntime, detail))?;

  log_info!(
    format!(
      "running {} {}",
      artifact.file_name,
      plan.installer.unattended_args.join(" ")
    ),
    "external_component_setup::install_runtime",
    None::<&str>
  );
  let mut command = Command::new(&installer_path);
  command.args(plan.installer.unattended_args);
  let exit_code = run_bounded(&mut command, artifact.file_name, INSTALLER_TIMEOUT)?;
  drop(guard);

  Ok(interpret_installer_exit_code(&plan.installer, exit_code))
}

/// Run `command` and return its exit code, or terminate it once `limit` has
/// passed. A start failure is `StartInstaller`. A terminated run is
/// `InstallerTimedOut` once the child is confirmed gone and reaped, so the
/// staged executable is no longer in use when the staging directory is
/// removed; a child whose termination is refused or that has not ended
/// within [`TERMINATION_CONFIRM_TIMEOUT`] is `InstallerStillRunning`, and
/// the caller must not remove the staging directory under it.
fn run_bounded(
  command: &mut Command,
  file_name: &str,
  limit: Duration,
) -> Result<Option<i32>, (SetupFailureStage, String)> {
  run_bounded_confirming(command, file_name, limit, TERMINATION_CONFIRM_TIMEOUT)
}

fn run_bounded_confirming(
  command: &mut Command,
  file_name: &str,
  limit: Duration,
  confirm_limit: Duration,
) -> Result<Option<i32>, (SetupFailureStage, String)> {
  let mut child = command.spawn().map_err(|e| {
    (
      SetupFailureStage::StartInstaller,
      format!("failed to start {file_name}: {e}"),
    )
  })?;

  if wait_signaled(&child, limit) {
    let status = child.wait().map_err(|e| {
      (
        SetupFailureStage::InstallerExit,
        format!("failed to read the exit code of {file_name}: {e}"),
      )
    })?;
    return Ok(status.code());
  }

  log_warn!(
    format!(
      "{file_name} did not exit within {} s; terminating it",
      limit.as_secs()
    ),
    "external_component_setup::run_bounded",
    None::<&str>
  );
  if let Err(e) = child.kill() {
    return Err((
      SetupFailureStage::InstallerStillRunning,
      format!(
        "{file_name} did not exit within {} s and could not be terminated: {e}",
        limit.as_secs()
      ),
    ));
  }
  if !wait_signaled(&child, confirm_limit) {
    return Err((
      SetupFailureStage::InstallerStillRunning,
      format!(
        "{file_name} did not exit within {} s and had not ended {} s after termination \
         was requested",
        limit.as_secs(),
        confirm_limit.as_secs()
      ),
    ));
  }
  // The process object is signaled, so reaping returns at once.
  let _ = child.wait();
  Err((
    SetupFailureStage::InstallerTimedOut,
    format!(
      "{file_name} did not exit within {} s and was terminated",
      limit.as_secs()
    ),
  ))
}

/// `true` once `child`'s process object is signaled (it has ended) within
/// `limit`.
fn wait_signaled(child: &std::process::Child, limit: Duration) -> bool {
  let limit_ms = u32::try_from(limit.as_millis()).unwrap_or(u32::MAX);
  let waited = unsafe { WaitForSingleObject(HANDLE(child.as_raw_handle()), limit_ms) };
  waited == WAIT_OBJECT_0
}

fn place_module_files(
  bundle: &FileBundleStep,
  missing: &[&str],
  destination: &Path,
) -> Result<Vec<String>, (SetupFailureStage, String)> {
  let bytes = download_verified(&bundle.artifact).map_err(|(stage, detail)| {
    (
      match stage {
        DownloadFailure::Transfer => SetupFailureStage::DownloadModules,
        DownloadFailure::Verification => SetupFailureStage::VerifyModules,
      },
      detail,
    )
  })?;
  let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| {
    (
      SetupFailureStage::ArchiveContents,
      format!("failed to open {}: {e}", bundle.artifact.file_name),
    )
  })?;
  let entry_names = archive.file_names().map(str::to_string).collect::<Vec<_>>();
  let selected = select_bundle_entries(&entry_names, missing);
  if selected.len() != missing.len() {
    let found = selected
      .iter()
      .map(|(_, file_name)| *file_name)
      .collect::<Vec<_>>();
    let absent = missing
      .iter()
      .filter(|file_name| !found.contains(file_name))
      .copied()
      .collect::<Vec<_>>();
    return Err((
      SetupFailureStage::ArchiveContents,
      format!(
        "{} does not contain {}",
        bundle.artifact.file_name,
        absent.join(", ")
      ),
    ));
  }

  fs::create_dir_all(destination).map_err(|e| {
    (
      SetupFailureStage::PlaceModules,
      format!("failed to create {}: {e}", destination.display()),
    )
  })?;

  let mut placed = Vec::new();
  for (entry_name, file_name) in selected {
    let mut entry = archive.by_name(entry_name).map_err(|e| {
      (
        SetupFailureStage::ArchiveContents,
        format!("failed to read {entry_name}: {e}"),
      )
    })?;
    let mut contents = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut contents).map_err(|e| {
      (
        SetupFailureStage::ArchiveContents,
        format!("failed to read {entry_name}: {e}"),
      )
    })?;
    if publish_file_no_clobber(destination, file_name, &contents)
      .map_err(|detail| (SetupFailureStage::PlaceModules, detail))?
    {
      placed.push(file_name.to_string());
    }
  }

  Ok(placed)
}

/// Write `contents` to `<directory>\<file_name>` atomically without ever
/// replacing an existing file. The data is staged in a sibling partial file
/// and linked into place; a link fails when the target already exists, so a
/// concurrent placement or a user-provided file is preserved untouched and a
/// partial file never appears under the final name. Returns whether the file
/// was placed by this call.
fn publish_file_no_clobber(
  directory: &Path,
  file_name: &str,
  contents: &[u8],
) -> Result<bool, String> {
  let target = directory.join(file_name);
  // Unique per attempt: a partial left behind by an interrupted run must not
  // block later attempts, and a partial another process is writing must not
  // be touched.
  let partial = directory.join(format!(
    "{file_name}{PARTIAL_SUFFIX}.{}",
    random_hex::<8>()?
  ));

  let mut file = fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(&partial)
    .map_err(|e| format!("failed to create {}: {e}", partial.display()))?;
  let written = file.write_all(contents).and_then(|()| file.sync_all());
  drop(file);
  if let Err(e) = written {
    let _ = fs::remove_file(&partial);
    return Err(format!("failed to write {}: {e}", partial.display()));
  }

  let linked = fs::hard_link(&partial, &target);
  let _ = fs::remove_file(&partial);
  match linked {
    Ok(()) => Ok(true),
    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
    Err(e) => Err(format!("failed to place {}: {e}", target.display())),
  }
}

/// Build the HTTP client for artifact downloads.
///
/// reqwest is compiled with `rustls-no-provider`, which does not fall back to
/// the rustls crate-feature provider: the process must have a default
/// `CryptoProvider` installed before the first client is built, otherwise the
/// client's event-loop thread panics and takes the setup process down with
/// exit status 101. The GUI app never installs one either (its updater
/// installs its own only when used), so this boundary installs the `ring`
/// provider itself. A second installation returns `Err`, which is the
/// already-installed case and is fine.
fn download_client() -> Result<reqwest::blocking::Client, reqwest::Error> {
  let _ = rustls::crypto::ring::default_provider().install_default();
  reqwest::blocking::Client::builder()
    .timeout(DOWNLOAD_TIMEOUT)
    .user_agent(concat!("HardwareVisualizer/", env!("CARGO_PKG_VERSION")))
    .build()
}

enum DownloadFailure {
  Transfer,
  Verification,
}

fn download_verified(
  artifact: &PinnedArtifact,
) -> Result<Vec<u8>, (DownloadFailure, String)> {
  log_info!(
    format!("downloading {}", artifact.url),
    "external_component_setup::download_verified",
    None::<&str>
  );
  let client = download_client().map_err(|e| {
    (
      DownloadFailure::Transfer,
      format!("failed to build the download client: {e}"),
    )
  })?;
  let response = client
    .get(artifact.url)
    .send()
    .and_then(|response| response.error_for_status())
    .map_err(|e| {
      (
        DownloadFailure::Transfer,
        format!("failed to download {}: {e}", artifact.file_name),
      )
    })?;
  let bytes = response.bytes().map_err(|e| {
    (
      DownloadFailure::Transfer,
      format!("failed to read {}: {e}", artifact.file_name),
    )
  })?;

  if let Err(detail) = verify_artifact(&bytes, artifact) {
    log_warn!(
      detail.clone(),
      "external_component_setup::download_verified",
      None::<&str>
    );
    return Err((DownloadFailure::Verification, detail));
  }

  Ok(bytes.to_vec())
}

/// Search `root` for `file_name` up to `max_depth` levels. A missing root is
/// positive absence; a directory that cannot be read is an error, because a
/// permission or transient failure is not evidence that the file is absent.
fn find_named_file(
  root: &Path,
  file_name: &str,
  max_depth: usize,
) -> std::io::Result<Option<PathBuf>> {
  if max_depth == 0 || !root.exists() {
    return Ok(None);
  }

  let direct = root.join(file_name);
  if direct.is_file() {
    return Ok(Some(direct));
  }

  for entry in fs::read_dir(root)? {
    let path = entry?.path();
    if path.is_file()
      && path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(file_name))
    {
      return Ok(Some(path));
    }
    if path.is_dir()
      && let Some(found) = find_named_file(&path, file_name, max_depth - 1)?
    {
      return Ok(Some(found));
    }
  }

  Ok(None)
}

fn random_hex<const N: usize>() -> Result<String, String> {
  let mut random = [0u8; N];
  getrandom::fill(&mut random).map_err(|e| format!("random name failed: {e}"))?;
  Ok(random.iter().map(|b| format!("{b:02x}")).collect())
}

fn wide_null(value: &str) -> Vec<u16> {
  value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn os_wide_null(value: &OsStr) -> Vec<u16> {
  value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The client must be constructible in a process that never installed a
  /// rustls provider itself; without the installation in `download_client`
  /// this panics on the client's event-loop thread. No request is sent.
  #[test]
  fn download_client_builds_without_a_preinstalled_crypto_provider() {
    let client = download_client().expect("download client must build");
    drop(client);
    // Idempotent: a second call must not fail on the already-installed provider.
    download_client().expect("download client must build again");
  }

  #[test]
  fn run_bounded_returns_the_exit_code_of_a_child_that_finishes_in_time() {
    let mut command = Command::new("cmd");
    command.args(["/C", "exit", "7"]);

    assert_eq!(
      run_bounded(&mut command, "cmd.exe", Duration::from_secs(30)),
      Ok(Some(7))
    );
  }

  /// A command that keeps its child alive for about 30 s, far beyond any
  /// limit the bounded-run tests use.
  fn long_lived_command() -> Command {
    let mut command = Command::new("ping");
    command
      .args(["-n", "30", "127.0.0.1"])
      .stdout(std::process::Stdio::null());
    command
  }

  #[test]
  fn run_bounded_terminates_a_child_that_outlives_the_limit() {
    // The limit is far shorter than the child's life, so the child must be
    // terminated, confirmed gone, and reported as timed out.
    let mut command = long_lived_command();
    let started = std::time::Instant::now();

    let result = run_bounded(&mut command, "ping.exe", Duration::from_millis(500));

    let (stage, detail) = result.expect_err("a hung child is a failure");
    assert_eq!(stage, SetupFailureStage::InstallerTimedOut);
    assert!(detail.contains("was terminated"), "{detail}");
    assert!(
      started.elapsed() < Duration::from_secs(10),
      "the wait must end at the limit, not at the child's natural exit"
    );
  }

  #[test]
  fn run_bounded_never_reports_a_confirmed_stop_for_a_child_that_has_not_ended() {
    // With no time allowed for the termination to complete, whether the
    // kernel has already torn the child down is timing-dependent, so either
    // stage is acceptable; what must never happen is `InstallerTimedOut`
    // while the child is still alive, and the wait must not block on it.
    let mut command = long_lived_command();
    let started = std::time::Instant::now();

    let result = run_bounded_confirming(
      &mut command,
      "ping.exe",
      Duration::from_millis(500),
      Duration::ZERO,
    );

    let (stage, detail) = result.expect_err("a hung child is a failure");
    match stage {
      SetupFailureStage::InstallerTimedOut => {
        assert!(detail.contains("was terminated"), "{detail}");
      }
      SetupFailureStage::InstallerStillRunning => {
        assert!(detail.contains("had not ended"), "{detail}");
      }
      other => panic!("unexpected stage {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(10));
  }

  #[test]
  fn a_process_under_a_staging_directory_is_found_on_component_boundaries() {
    let root = Path::new(r"C:\Windows\Temp");
    let prefix = "hardviz-external-component-setup-";
    let staged = (
      7,
      PathBuf::from(
        r"c:\windows\temp\HARDVIZ-EXTERNAL-COMPONENT-SETUP-abc\PawnIO_setup.exe",
      ),
    );
    let processes = vec![
      (1, PathBuf::from(r"C:\Windows\Temp\PawnIO_setup.exe")),
      (
        2,
        PathBuf::from(r"C:\Windows\Temp2\hardviz-external-component-setup-x\a.exe"),
      ),
      (3, PathBuf::from(r"C:\Windows\Temp\hardviz-other-x\a.exe")),
      (
        4,
        PathBuf::from(r"C:\Windows\Temp\hardviz-external-component-setup-x\deep\a.exe"),
      ),
      (
        5,
        PathBuf::from(r"C:\Windows\Temp\hardviz-external-component-setup-x"),
      ),
      (6, PathBuf::from(r"C:\Windows\System32\svchost.exe")),
      staged.clone(),
    ];

    assert_eq!(
      first_process_under_staging(root, prefix, processes),
      Some(staged)
    );
    assert_eq!(first_process_under_staging(root, prefix, Vec::new()), None);
  }

  #[test]
  fn a_live_child_running_from_a_staging_directory_is_detected() {
    // A copy of cmd.exe runs from a directory named like a staging
    // directory under a temp root; the same copy running from a sibling
    // outside the prefix must not be reported.
    let root = std::env::temp_dir().join(format!(
      "hardviz-scan-root-{}",
      random_hex::<8>().expect("random name")
    ));
    let inside = root.join(format!("{STAGING_DIRECTORY_PREFIX}test"));
    let outside = root.join("unrelated-test");
    for directory in [&inside, &outside] {
      fs::create_dir_all(directory).expect("test directory is created");
    }
    let system_root = std::env::var_os("SystemRoot").expect("SystemRoot is set");
    let cmd = PathBuf::from(system_root).join("System32").join("cmd.exe");
    let inside_exe = inside.join("staged.exe");
    let outside_exe = outside.join("staged.exe");
    fs::copy(&cmd, &inside_exe).expect("cmd.exe is copied");
    fs::copy(&cmd, &outside_exe).expect("cmd.exe is copied");
    let spawn = |exe: &Path| {
      Command::new(exe)
        .args(["/C", "ping", "-n", "30", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("the copied cmd.exe runs")
    };
    let mut inside_child = spawn(&inside_exe);
    let mut outside_child = spawn(&outside_exe);

    let found =
      first_process_under_staging(&root, STAGING_DIRECTORY_PREFIX, process_image_paths());

    let _ = inside_child.kill();
    let _ = outside_child.kill();
    let _ = inside_child.wait();
    let _ = outside_child.wait();
    let _ = fs::remove_dir_all(&root);

    let (pid, path) = found.expect("the staged child is detected");
    assert_eq!(pid, inside_child.id());
    assert_eq!(
      path.to_string_lossy().to_lowercase(),
      inside_exe.to_string_lossy().to_lowercase()
    );
  }

  #[test]
  fn an_abandoned_staging_directory_is_not_removed_on_drop() {
    // Built directly so the test does not depend on the DACL-protected
    // `%SystemRoot%\Temp`, which a non-administrator test run cannot clean.
    let path = std::env::temp_dir().join(format!(
      "hardviz-staging-abandon-{}",
      random_hex::<8>().expect("random name")
    ));
    fs::create_dir(&path).expect("test directory is created");

    let mut staging = StagingDirectory {
      path: path.clone(),
      abandoned: false,
    };
    staging.abandon();
    drop(staging);

    assert!(path.is_dir(), "the abandoned directory stays in place");
    fs::remove_dir_all(&path).expect("test directory is removed");
  }

  #[test]
  fn a_staging_directory_is_removed_on_drop_unless_abandoned() {
    let path = std::env::temp_dir().join(format!(
      "hardviz-staging-drop-{}",
      random_hex::<8>().expect("random name")
    ));
    fs::create_dir(&path).expect("test directory is created");

    drop(StagingDirectory {
      path: path.clone(),
      abandoned: false,
    });

    assert!(!path.exists(), "the directory is removed on drop");
  }

  #[test]
  fn run_bounded_reports_a_start_failure_at_the_start_stage() {
    let mut command = Command::new(r"C:\hardviz-does-not-exist\installer.exe");

    let (stage, _) = run_bounded(&mut command, "installer.exe", Duration::from_secs(1))
      .expect_err("a missing executable cannot start");
    assert_eq!(stage, SetupFailureStage::StartInstaller);
  }
}
