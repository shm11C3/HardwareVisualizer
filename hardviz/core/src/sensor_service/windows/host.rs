use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use windows::Win32::System::Services::*;
use windows::core::{PCWSTR, PWSTR};

use super::SERVICE_NAME;

static STOP: AtomicBool = AtomicBool::new(false);
static STATUS: AtomicUsize = AtomicUsize::new(0);
static BUILD_VERSION: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

pub(super) fn build_version() -> &'static str {
  BUILD_VERSION.get().copied().unwrap_or("unknown")
}

pub fn run(version: &'static str) -> Result<(), String> {
  let _ = BUILD_VERSION.set(version);
  let _executable = super::security::protected_file(
    &std::env::current_exe().map_err(|e| e.to_string())?,
  )?;

  let name: Vec<_> = SERVICE_NAME.encode_utf16().chain([0]).collect();
  let entries = [
    SERVICE_TABLE_ENTRYW {
      lpServiceName: PWSTR(name.as_ptr().cast_mut()),
      lpServiceProc: Some(service_main),
    },
    SERVICE_TABLE_ENTRYW::default(),
  ];

  unsafe { StartServiceCtrlDispatcherW(entries.as_ptr()) }.map_err(|e| e.to_string())
}

unsafe extern "system" fn service_main(_: u32, _: *mut PWSTR) {
  let name: Vec<_> = SERVICE_NAME.encode_utf16().chain([0]).collect();
  let result =
    unsafe { RegisterServiceCtrlHandlerExW(PCWSTR(name.as_ptr()), Some(control), None) };

  let Ok(handle) = result else {
    return;
  };
  STATUS.store(handle.0 as usize, Ordering::Release);
  status(SERVICE_START_PENDING, 0);

  let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(2)
    .max_blocking_threads(16)
    .enable_all()
    .build();

  let result = match runtime {
    Ok(runtime) => {
      status(SERVICE_RUNNING, 0);

      let result = runtime.block_on(super::server::serve(&STOP));
      runtime.shutdown_timeout(Duration::from_secs(3));
      result
    }
    Err(error) => Err(error.to_string()),
  };

  if let Err(error) = &result {
    crate::log_error!(
      "sensor_service_failed",
      "sensor_service::service_main",
      Some(error.clone())
    );
  }
  status(SERVICE_STOPPED, u32::from(result.is_err()));
}

unsafe extern "system" fn control(
  event: u32,
  _: u32,
  _: *mut std::ffi::c_void,
  _: *mut std::ffi::c_void,
) -> u32 {
  match event {
    SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
      STOP.store(true, Ordering::Release);
      status(SERVICE_STOP_PENDING, 0);
    }

    SERVICE_CONTROL_POWEREVENT => {
      super::server::invalidate();
    }

    _ => {}
  }
  0
}

fn status(state: SERVICE_STATUS_CURRENT_STATE, error: u32) {
  let status = SERVICE_STATUS {
    dwServiceType: SERVICE_WIN32_OWN_PROCESS,
    dwCurrentState: state,
    dwControlsAccepted: if state == SERVICE_RUNNING {
      SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN | SERVICE_ACCEPT_POWEREVENT
    } else {
      0
    },
    dwWin32ExitCode: error,
    dwServiceSpecificExitCode: 0,
    dwCheckPoint: 0,
    dwWaitHint: if state == SERVICE_START_PENDING || state == SERVICE_STOP_PENDING {
      10_000
    } else {
      0
    },
  };

  unsafe {
    let _ = SetServiceStatus(
      SERVICE_STATUS_HANDLE(STATUS.load(Ordering::Acquire) as *mut _),
      &status,
    );
  }
}
