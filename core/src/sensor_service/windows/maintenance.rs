use std::path::PathBuf;
use std::sync::Mutex;
use windows::Win32::Foundation::{
  ERROR_INSTALL_USEREXIT, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS,
  ERROR_SUCCESS_REBOOT_REQUIRED,
};
use windows::Win32::System::ApplicationInstallationAndServicing::*;
use windows::core::{PCWSTR, PWSTR, w};

use crate::sensor_service::protocol::ServiceStatus;

static INSTALLING: Mutex<()> = Mutex::new(());
const SENSOR_FEATURE: PCWSTR = w!("WindowsSensorService");

pub async fn status() -> ServiceStatus {
  let result = super::client::handshake().await.map(|_| ());
  let can_configure =
    tokio::task::spawn_blocking(|| product_for_current_install().is_ok())
      .await
      .unwrap_or(false);

  ServiceStatus {
    connected: result.is_ok(),
    can_configure,
    detail: result.err(),
  }
}

/// Windows Installer owns the feature and its cached, registered product.
/// No client-supplied MSI path or command line crosses an elevated boundary.
pub fn configure() -> Result<(), String> {
  let _guard = INSTALLING
    .try_lock()
    .map_err(|_| "Sensor service setup is already running")?;

  let product = product_for_current_install()?;
  struct RestoreUi(INSTALLUILEVEL);

  impl Drop for RestoreUi {
    fn drop(&mut self) {
      unsafe {
        MsiSetInternalUI(self.0, None);
      }
    }
  }

  unsafe {
    let _ui = RestoreUi(MsiSetInternalUI(INSTALLUILEVEL_FULL, None));
    let product = PCWSTR(product.as_ptr());
    let result = if MsiQueryFeatureStateW(product, SENSOR_FEATURE) == INSTALLSTATE_LOCAL {
      MsiReinstallFeatureW(
        product,
        SENSOR_FEATURE,
        REINSTALLMODE(REINSTALLMODE_FILEOLDERVERSION.0 | REINSTALLMODE_MACHINEDATA.0),
      )
    } else {
      MsiConfigureFeatureW(product, SENSOR_FEATURE, INSTALLSTATE_LOCAL)
    };

    match result {
      code if code == ERROR_SUCCESS.0 || code == ERROR_SUCCESS_REBOOT_REQUIRED.0 => {
        Ok(())
      }
      code if code == ERROR_INSTALL_USEREXIT.0 => {
        Err("Sensor service setup was cancelled".into())
      }
      code => Err(format!(
        "Windows Installer sensor service setup failed ({code}); repair the protected MSI installation"
      )),
    }
  }
}

fn product_for_current_install() -> Result<Vec<u16>, String> {
  let exe = std::env::current_exe().map_err(|e| e.to_string())?;
  let _held_exe = super::security::protected_file(&exe)?;
  let parent = exe
    .parent()
    .ok_or("Application installation directory missing")?
    .canonicalize()
    .map_err(|e| e.to_string())?;

  let mut found = None;

  // MSI registrations are OS-owned. Match both product name and resolved
  // location; a different MSI or a per-user NSIS client cannot own this feature.
  for index in 0..4096 {
    let mut product = vec![0u16; 39];
    let result = unsafe { MsiEnumProductsW(index, PWSTR(product.as_mut_ptr())) };

    if result == ERROR_NO_MORE_ITEMS.0 {
      break;
    }

    if result != ERROR_SUCCESS.0 {
      return Err(format!(
        "Cannot inspect Windows Installer registration ({result})"
      ));
    }

    let name = property(&product, w!("ProductName"))?;

    if name != "HardwareVisualizer" {
      continue;
    }

    let location = property(&product, w!("InstallLocation"))?;

    if location.is_empty() {
      continue;
    }

    if PathBuf::from(location).canonicalize().ok().as_ref() != Some(&parent) {
      continue;
    }

    if found.is_some() {
      return Err("Ambiguous HardwareVisualizer MSI registration".into());
    }

    found = Some(product);
  }
  found.ok_or_else(|| {
    "Install the protected HardwareVisualizer MSI to set up the sensor service".into()
  })
}

fn property(product: &[u16], key: PCWSTR) -> Result<String, String> {
  let mut value = vec![0u16; 32768];
  let mut length = value.len() as u32;
  let result = unsafe {
    MsiGetProductInfoW(
      PCWSTR(product.as_ptr()),
      key,
      Some(PWSTR(value.as_mut_ptr())),
      Some(&mut length),
    )
  };

  if result != ERROR_SUCCESS.0 {
    return Err(format!(
      "Cannot inspect Windows Installer product ({result})"
    ));
  }
  String::from_utf16(&value[..length as usize]).map_err(|e| e.to_string())
}
