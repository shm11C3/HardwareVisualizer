use std::ffi::c_void;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use tokio::net::windows::named_pipe::{NamedPipeClient, NamedPipeServer, ServerOptions};
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
  ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
  SDDL_REVISION_1,
};
use windows::Win32::Security::{
  GetTokenInformation, PSECURITY_DESCRIPTOR, RevertToSelf, SECURITY_ATTRIBUTES,
  TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::Storage::FileSystem::{
  CreateFileW, FILE_FLAG_OVERLAPPED, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_NONE,
  OPEN_EXISTING,
};
use windows::Win32::System::Pipes::{
  GetNamedPipeServerProcessId, ImpersonateNamedPipeClient,
};
use windows::Win32::System::Services::*;
use windows::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};
use windows::core::{PCWSTR, PWSTR, w};

use super::{PIPE_NAME, SERVICE_NAME};

struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
  fn drop(&mut self) {
    unsafe {
      let _ = LocalFree(Some(HLOCAL(self.0)));
    }
  }
}

fn user_sid(token: HANDLE) -> Result<String, String> {
  let mut needed = 0;

  unsafe {
    let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
  }

  if needed == 0 || needed > 64 * 1024 {
    return Err("Invalid Windows token size".into());
  }

  // usize storage keeps TOKEN_USER aligned, unlike an arbitrary byte slice.
  let mut buffer = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];

  unsafe {
    GetTokenInformation(
      token,
      TokenUser,
      Some(buffer.as_mut_ptr().cast()),
      needed,
      &mut needed,
    )
    .map_err(|e| e.to_string())?;

    let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
    let mut sid = PWSTR::null();
    ConvertSidToStringSidW(user.User.Sid, &mut sid).map_err(|e| e.to_string())?;

    let _allocation = LocalAllocation(sid.0.cast());
    sid.to_string().map_err(|e| e.to_string())
  }
}

pub(super) fn server_pipe(first: bool) -> Result<NamedPipeServer, String> {
  // Interactive clients get data rights, never FILE_APPEND_DATA (which is
  // FILE_CREATE_PIPE_INSTANCE on pipes). Generic-write would grant that bit.
  let mut descriptor = PSECURITY_DESCRIPTOR::default();

  unsafe {
    ConvertStringSecurityDescriptorToSecurityDescriptorW(
      w!("D:P(D;;GA;;;NU)(D;;GA;;;AN)(A;;GA;;;SY)(A;;0x12019b;;;IU)S:(ML;;NW;;;ME)"),
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
      .max_instances(17)
      .in_buffer_size(65_536)
      .out_buffer_size(65_536)
      .create_with_security_attributes_raw(
        PIPE_NAME,
        (&mut attrs as *mut SECURITY_ATTRIBUTES).cast(),
      )
      .map_err(|e| e.to_string())
  }
}

pub(super) fn client_sid(pipe: &NamedPipeServer) -> Result<String, String> {
  struct Revert;

  impl Drop for Revert {
    fn drop(&mut self) {
      unsafe {
        let _ = RevertToSelf();
      }
    }
  }

  unsafe {
    ImpersonateNamedPipeClient(HANDLE(pipe.as_raw_handle()))
      .map_err(|e| e.to_string())?;

    let _revert = Revert;
    let mut token = HANDLE::default();
    OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token)
      .map_err(|e| e.to_string())?;

    let token = OwnedHandle::from_raw_handle(token.0);
    user_sid(HANDLE(token.as_raw_handle()))
  }

  // No impersonation can survive this synchronous scope or cross an await.
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
  unsafe {
    let pipe = CreateFileW(
      w!("\\\\.\\pipe\\HardwareVisualizer.Sensors.v1"),
      0x12019b,
      FILE_SHARE_NONE,
      None,
      OPEN_EXISTING,
      FILE_FLAG_OVERLAPPED | FILE_FLAGS_AND_ATTRIBUTES(0x110000),
      None,
    )
    .map_err(|e| format!("Sensor service connection unavailable: {e}"))?;

    let pipe = OwnedHandle::from_raw_handle(pipe.0);
    authenticate_server(HANDLE(pipe.as_raw_handle()))?;
    use std::os::windows::io::IntoRawHandle;
    NamedPipeClient::from_raw_handle(pipe.into_raw_handle()).map_err(|e| e.to_string())
  }
}

/// Hold privileged code/data against replacement and reject writable owners,
/// writable ACLs and reparse redirection, including on ancestor directories.
pub(crate) fn validate_install_directory(path: &std::path::Path) -> Result<(), String> {
  use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

  let resolved = crate::platform::windows::process_elevation::protected_file_path(path)?
    .ok_or("The sensor service installation must be under Program Files")?;

  for ancestor in path.ancestors() {
    if std::fs::symlink_metadata(ancestor)
      .map_err(|e| e.to_string())?
      .file_attributes()
      & 0x400
      != 0
    {
      return Err(
        "Reparse points are not allowed in the sensor service installation".into(),
      );
    }
  }

  let mut parent = Some(resolved.as_path());

  while let Some(directory) = parent {
    let last =
      crate::platform::windows::process_elevation::protected_file_path(directory)?
        .is_none();

    let handle = std::fs::OpenOptions::new()
      .read(true)
      .share_mode(7)
      .custom_flags(0x02000000)
      .open(directory)
      .map_err(|e| e.to_string())?;
    protected_acl(HANDLE(handle.as_raw_handle()))?;

    if last {
      break;
    }

    parent = directory.parent();
  }

  Ok(())
}

pub(crate) fn protected_file(path: &std::path::Path) -> Result<std::fs::File, String> {
  use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

  let resolved = crate::platform::windows::process_elevation::protected_file_path(path)?
    .ok_or_else(|| {
      format!(
        "Privileged sensor input is outside Program Files: {}",
        path.display()
      )
    })?;

  for ancestor in path.ancestors() {
    if std::fs::symlink_metadata(ancestor)
      .map_err(|e| e.to_string())?
      .file_attributes()
      & 0x400
      != 0
    {
      return Err("Reparse points are not allowed for privileged sensor inputs".into());
    }
  }

  let file = std::fs::OpenOptions::new()
    .read(true)
    .share_mode(1)
    .open(&resolved)
    .map_err(|e| e.to_string())?;
  protected_acl(HANDLE(file.as_raw_handle()))?;
  validate_install_directory(resolved.parent().ok_or("Sensor input has no parent")?)?;

  Ok(file)
}

fn protected_acl(handle: HANDLE) -> Result<(), String> {
  use windows::Win32::Security::Authorization::{GetSecurityInfo, SE_FILE_OBJECT};
  use windows::Win32::Security::GetAce;
  use windows::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION,
    OWNER_SECURITY_INFORMATION, PSID,
  };

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

    if !trusted_sid(owner)? || dacl.is_null() {
      return Err(
        "Privileged sensor input has an untrusted owner or unrestricted ACL".into(),
      );
    }

    for i in 0..(*dacl).AceCount {
      let mut ace = std::ptr::null_mut();
      GetAce(dacl, i as u32, &mut ace).map_err(|e| e.to_string())?;

      let header = &*ace.cast::<ACE_HEADER>();

      if header.AceFlags & 0x08 != 0 {
        continue;
      }

      if header.AceType == 0 {
        let allowed = &*ace.cast::<ACCESS_ALLOWED_ACE>();
        let writes = 0x10000000 | 0x40000000 | 0x000d0156;
        let sid = PSID((&allowed.SidStart as *const u32).cast_mut().cast());

        if allowed.Mask & writes != 0 && !trusted_sid(sid)? {
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
