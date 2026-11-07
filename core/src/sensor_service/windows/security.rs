use std::ffi::c_void;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};

use tokio::net::windows::named_pipe::{NamedPipeClient, NamedPipeServer, ServerOptions};
use windows::Win32::Foundation::{GENERIC_ALL, GENERIC_WRITE, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
  ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
  SDDL_REVISION_1,
};
use windows::Win32::Security::{ACL, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::{
  CreateFileW, DELETE, FILE_APPEND_DATA, FILE_ATTRIBUTE_REPARSE_POINT, FILE_DELETE_CHILD,
  FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_SHARE_DELETE,
  FILE_SHARE_NONE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES,
  FILE_WRITE_DATA, FILE_WRITE_EA, OPEN_EXISTING, SECURITY_IDENTIFICATION,
  SECURITY_SQOS_PRESENT, WRITE_DAC, WRITE_OWNER,
};
use windows::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows::Win32::System::Services::*;
use windows::core::{PCWSTR, PWSTR};

use super::{MAX_CLIENTS, PIPE_NAME, SERVICE_NAME};

const PIPE_CLIENT_ACCESS: u32 =
  FILE_GENERIC_READ.0 | FILE_WRITE_DATA.0 | FILE_WRITE_EA.0 | FILE_WRITE_ATTRIBUTES.0;

const FILE_MUTATION_ACCESS: u32 = GENERIC_ALL.0
  | GENERIC_WRITE.0
  | DELETE.0
  | WRITE_DAC.0
  | WRITE_OWNER.0
  | FILE_WRITE_ATTRIBUTES.0
  | FILE_DELETE_CHILD.0
  | FILE_WRITE_EA.0
  | FILE_APPEND_DATA.0
  | FILE_WRITE_DATA.0;

struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
  fn drop(&mut self) {
    unsafe {
      let _ = LocalFree(Some(HLOCAL(self.0)));
    }
  }
}

pub(super) fn server_pipe(first: bool) -> Result<NamedPipeServer, String> {
  // Interactive clients get data rights, never FILE_APPEND_DATA (which is
  // FILE_CREATE_PIPE_INSTANCE on pipes). Generic-write would grant that bit.
  let descriptor_text: Vec<_> =
    format!("D:P(D;;GA;;;NU)(D;;GA;;;AN)(A;;GA;;;SY)(A;;{PIPE_CLIENT_ACCESS:#x};;;IU)S:(ML;;NW;;;ME)")
      .encode_utf16()
      .chain([0])
      .collect();
  let mut descriptor = PSECURITY_DESCRIPTOR::default();

  unsafe {
    ConvertStringSecurityDescriptorToSecurityDescriptorW(
      PCWSTR(descriptor_text.as_ptr()),
      SDDL_REVISION_1,
      &mut descriptor,
      None,
    )
    .map_err(|e| e.to_string())?;

    let _allocation = LocalAllocation(descriptor.0);
    let mut attrs = SECURITY_ATTRIBUTES {
      nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
      lpSecurityDescriptor: descriptor.0,
      bInheritHandle: false.into(),
    };
    ServerOptions::new()
      .first_pipe_instance(first)
      .reject_remote_clients(true)
      .max_instances(MAX_CLIENTS + 1)
      .in_buffer_size(65_536)
      .out_buffer_size(65_536)
      .create_with_security_attributes_raw(
        PIPE_NAME,
        (&mut attrs as *mut SECURITY_ATTRIBUTES).cast(),
      )
      .map_err(|e| e.to_string())
  }
}

struct ServiceHandle(SC_HANDLE);

impl Drop for ServiceHandle {
  fn drop(&mut self) {
    unsafe {
      let _ = CloseServiceHandle(self.0);
    }
  }
}

pub(super) fn connect() -> Result<NamedPipeClient, String> {
  let name: Vec<_> = PIPE_NAME.encode_utf16().chain([0]).collect();

  unsafe {
    let pipe = CreateFileW(
      PCWSTR(name.as_ptr()),
      PIPE_CLIENT_ACCESS,
      FILE_SHARE_NONE,
      None,
      OPEN_EXISTING,
      FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
      None,
    )
    .map_err(|e| format!("Sensor service connection unavailable: {e}"))?;

    let pipe = OwnedHandle::from_raw_handle(pipe.0);
    authenticate_server(HANDLE(pipe.as_raw_handle()))?;
    use std::os::windows::io::IntoRawHandle;
    NamedPipeClient::from_raw_handle(pipe.into_raw_handle()).map_err(|e| e.to_string())
  }
}

fn protected_path(path: &Path) -> Result<PathBuf, String> {
  use std::os::windows::fs::MetadataExt;

  let resolved = crate::platform::windows::process_elevation::protected_file_path(path)?
    .ok_or("Privileged sensor input must be under Program Files")?;

  for ancestor in path.ancestors() {
    if std::fs::symlink_metadata(ancestor)
      .map_err(|e| e.to_string())?
      .file_attributes()
      & FILE_ATTRIBUTE_REPARSE_POINT.0
      != 0
    {
      return Err("Reparse points are not allowed for privileged sensor inputs".into());
    }
  }

  Ok(resolved)
}

/// The install directory must be safe both now and for newly installed files.
pub(crate) fn validate_install_directory(path: &Path) -> Result<(), String> {
  let resolved = protected_path(path)?;
  validate_directory_acl(&resolved, AclCheck::NewFileParent)
}

fn validate_directory_acl(path: &Path, first_check: AclCheck) -> Result<(), String> {
  use std::os::windows::fs::OpenOptionsExt;

  let mut parent = Some(path);

  while let Some(directory) = parent {
    let last =
      crate::platform::windows::process_elevation::protected_file_path(directory)?
        .is_none();

    let handle = std::fs::OpenOptions::new()
      .read(true)
      .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
      .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
      .open(directory)
      .map_err(|e| e.to_string())?;
    let check = if directory == path {
      first_check
    } else {
      // Existing descendants already reflect inheritance or block it. Only
      // effective ancestor permissions can allow replacement of this path.
      AclCheck::Effective
    };
    protected_acl(HANDLE(handle.as_raw_handle()), check)?;

    if last {
      break;
    }

    parent = directory.parent();
  }

  Ok(())
}

/// Hold an existing runtime input against replacement through the caller's use.
pub(crate) fn protected_file(path: &Path) -> Result<std::fs::File, String> {
  use std::os::windows::fs::OpenOptionsExt;

  let resolved = protected_path(path)?;
  let file = std::fs::OpenOptions::new()
    .read(true)
    .share_mode(FILE_SHARE_READ.0)
    .open(&resolved)
    .map_err(|e| e.to_string())?;
  protected_acl(HANDLE(file.as_raw_handle()), AclCheck::Effective)?;
  validate_directory_acl(
    resolved.parent().ok_or("Sensor input has no parent")?,
    AclCheck::Effective,
  )?;

  Ok(file)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AclCheck {
  Effective,
  NewFileParent,
}

fn protected_acl(handle: HANDLE, check: AclCheck) -> Result<(), String> {
  use windows::Win32::Security::Authorization::{GetSecurityInfo, SE_FILE_OBJECT};
  use windows::Win32::Security::{DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION};

  unsafe {
    let mut owner = PSID::default();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    GetSecurityInfo(
      handle,
      SE_FILE_OBJECT,
      DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
      Some(&mut owner),
      None,
      Some(&mut dacl),
      None,
      Some(&mut descriptor),
    )
    .ok()
    .map_err(|e| e.to_string())?;

    let _allocation = LocalAllocation(descriptor.0);
    validate_acl(owner, dacl, check)
  }
}

/// The owner and ACL must remain valid in the same security descriptor for
/// the entire check; callers retain its Windows allocation until this returns.
unsafe fn validate_acl(
  owner: PSID,
  dacl: *mut ACL,
  check: AclCheck,
) -> Result<(), String> {
  use windows::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, GetAce, INHERIT_ONLY_ACE, IsWellKnownSid,
    OBJECT_INHERIT_ACE, WinCreatorOwnerSid,
  };

  unsafe {
    if !trusted_sid(owner)? || dacl.is_null() {
      return Err(
        "Privileged sensor input has an untrusted owner or unrestricted ACL".into(),
      );
    }

    for i in 0..(*dacl).AceCount {
      let mut ace = std::ptr::null_mut();
      GetAce(dacl, i as u32, &mut ace).map_err(|e| e.to_string())?;

      let ace = std::ptr::NonNull::new(ace).ok_or("Missing ACL entry")?;
      let header = ace.cast::<ACE_HEADER>().as_ref();

      let inherit_only = u32::from(header.AceFlags) & INHERIT_ONLY_ACE.0 != 0;
      if inherit_only
        && (check == AclCheck::Effective
          || u32::from(header.AceFlags) & OBJECT_INHERIT_ACE.0 == 0)
      {
        continue;
      }

      if header.AceType == 0 {
        let allowed = ace.cast::<ACCESS_ALLOWED_ACE>().as_ref();
        let sid = PSID((&allowed.SidStart as *const u32).cast_mut().cast());

        // A trusted installer creates the new file, so CREATOR OWNER resolves
        // to its trusted SID. Effective write grants are checked independently;
        // other users cannot create files here. Normal Program Files uses this
        // template and must not be rejected as an untrusted named principal.
        let trusted_creator =
          inherit_only && IsWellKnownSid(sid, WinCreatorOwnerSid).as_bool();
        if allowed.Mask & FILE_MUTATION_ACCESS != 0
          && !trusted_sid(sid)?
          && !trusted_creator
        {
          return Err(
            "Privileged sensor input is writable by an unelevated principal".into(),
          );
        }
      } else if header.AceType != 1 {
        return Err("Unsupported ACL entry on privileged sensor input".into());
      }
    }

    Ok(())
  }
}

fn trusted_sid(sid: windows::Win32::Security::PSID) -> Result<bool, String> {
  unsafe {
    let mut value = PWSTR::null();
    ConvertSidToStringSidW(sid, &mut value).map_err(|e| e.to_string())?;

    let _allocation = LocalAllocation(value.0.cast());
    let sid = value.to_string().map_err(|e| e.to_string())?;

    Ok(matches!(
      sid.as_str(),
      "S-1-5-18"
        | "S-1-5-32-544"
        | "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
    ))
  }
}

fn authenticate_server(pipe: HANDLE) -> Result<(), String> {
  unsafe {
    let manager = ServiceHandle(
      OpenSCManagerW(None, None, SC_MANAGER_CONNECT).map_err(|e| e.to_string())?,
    );

    let service_name = SERVICE_NAME
      .encode_utf16()
      .chain(Some(0))
      .collect::<Vec<_>>();

    let service = ServiceHandle(
      OpenServiceW(
        manager.0,
        PCWSTR(service_name.as_ptr()),
        SERVICE_QUERY_STATUS | SERVICE_QUERY_CONFIG,
      )
      .map_err(|e| e.to_string())?,
    );

    let status = || -> Result<SERVICE_STATUS_PROCESS, String> {
      let mut status = SERVICE_STATUS_PROCESS::default();
      let mut needed = 0;
      let bytes = std::slice::from_raw_parts_mut(
        (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
        size_of::<SERVICE_STATUS_PROCESS>(),
      );
      QueryServiceStatusEx(service.0, SC_STATUS_PROCESS_INFO, Some(bytes), &mut needed)
        .map_err(|e| e.to_string())?;

      Ok(status)
    };

    let mut pipe_pid = 0;
    GetNamedPipeServerProcessId(pipe, &mut pipe_pid).map_err(|e| e.to_string())?;

    let before = status()?;

    if before.dwCurrentState != SERVICE_RUNNING
      || pipe_pid == 0
      || before.dwProcessId != pipe_pid
    {
      return Err("Sensor pipe is not owned by the running Windows service".into());
    }

    // Standard users may query SCM, but cannot necessarily open a SYSTEM
    // process or its token. Bind the OS-reported pipe PID to the protected
    // service registration instead of requiring those extra process rights.
    let mut needed = 0;
    let _ = QueryServiceConfigW(service.0, None, 0, &mut needed);

    if needed < size_of::<QUERY_SERVICE_CONFIGW>() as u32 || needed > 64 * 1024 {
      return Err("Invalid sensor service configuration size".into());
    }

    let mut buffer = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
    QueryServiceConfigW(
      service.0,
      Some(buffer.as_mut_ptr().cast()),
      needed,
      &mut needed,
    )
    .map_err(|e| e.to_string())?;

    let config = &*buffer.as_ptr().cast::<QUERY_SERVICE_CONFIGW>();
    let account = config
      .lpServiceStartName
      .to_string()
      .map_err(|e| e.to_string())?;

    if !registered_system_service(config.dwServiceType, &account) {
      return Err("Sensor service must be a dedicated LocalSystem service".into());
    }

    let after = status()?;
    let mut after_pipe_pid = 0;
    GetNamedPipeServerProcessId(pipe, &mut after_pipe_pid).map_err(|e| e.to_string())?;

    if after.dwCurrentState != SERVICE_RUNNING
      || after.dwProcessId != pipe_pid
      || after_pipe_pid != pipe_pid
    {
      return Err("Sensor service changed during authentication".into());
    }

    Ok(())
  }
}

fn registered_system_service(kind: ENUM_SERVICE_TYPE, account: &str) -> bool {
  kind == SERVICE_WIN32_OWN_PROCESS && account.eq_ignore_ascii_case("LocalSystem")
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::net::windows::named_pipe::ClientOptions;

  fn check_sddl(sddl: &str, check: AclCheck) -> Result<(), String> {
    use windows::Win32::Security::{
      GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
    };
    use windows::core::BOOL;

    let sddl = sddl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();

    unsafe {
      let mut descriptor = PSECURITY_DESCRIPTOR::default();
      ConvertStringSecurityDescriptorToSecurityDescriptorW(
        PCWSTR(sddl.as_ptr()),
        SDDL_REVISION_1,
        &mut descriptor,
        None,
      )
      .unwrap();
      let _allocation = LocalAllocation(descriptor.0);
      let mut owner = PSID::default();
      let mut defaulted = BOOL::default();
      GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted).unwrap();
      let mut dacl = std::ptr::null_mut();
      let mut present = BOOL::default();
      GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
        .unwrap();
      assert!(present.as_bool());
      validate_acl(owner, dacl, check)
    }
  }

  #[test]
  fn future_file_writes_are_checked_before_installation() {
    let inherited_write = "O:SYG:SYD:P(A;;FA;;;SY)(A;OIIO;GW;;;BU)";
    assert!(check_sddl(inherited_write, AclCheck::Effective).is_ok());
    assert!(check_sddl(inherited_write, AclCheck::NewFileParent).is_err());

    let effective_write = "O:SYG:SYD:P(A;;FA;;;SY)(A;;GW;;;BU)";
    assert!(check_sddl(effective_write, AclCheck::Effective).is_err());
    assert!(check_sddl(effective_write, AclCheck::NewFileParent).is_err());
  }

  #[test]
  fn normal_creator_owner_templates_and_program_files_ancestry_remain_supported() {
    assert!(
      check_sddl(
        "O:SYG:SYD:P(A;;FA;;;SY)(A;OICIIO;GA;;;CO)",
        AclCheck::NewFileParent,
      )
      .is_ok()
    );
    let common_files = std::env::var_os("CommonProgramFiles").unwrap();
    validate_install_directory(std::path::Path::new(&common_files)).unwrap();
  }

  #[test]
  fn server_registration_requires_a_dedicated_system_service() {
    assert!(registered_system_service(
      SERVICE_WIN32_OWN_PROCESS,
      "LocalSystem"
    ));
    assert!(!registered_system_service(
      SERVICE_WIN32_SHARE_PROCESS,
      "LocalSystem"
    ));
    assert!(!registered_system_service(
      SERVICE_WIN32_OWN_PROCESS,
      "LocalService"
    ));
  }

  #[test]
  fn rejects_a_writable_runtime_input_before_loading_it() {
    let temp = tempfile::tempdir().unwrap();
    let dll = temp.path().join("PawnIOLib.dll");
    std::fs::write(&dll, b"not executable").unwrap();
    assert!(protected_file(&dll).is_err());
  }

  #[tokio::test]
  async fn an_interactive_pipe_server_cannot_impersonate_the_system_service() {
    let name = format!(r"\\.\pipe\Hardviz.SpoofTest.{}", std::process::id());
    let server = ServerOptions::new()
      .first_pipe_instance(true)
      .create(&name)
      .unwrap();

    let client = ClientOptions::new().open(&name).unwrap();
    server.connect().await.unwrap();
    assert!(authenticate_server(HANDLE(client.as_raw_handle())).is_err());
  }
}
