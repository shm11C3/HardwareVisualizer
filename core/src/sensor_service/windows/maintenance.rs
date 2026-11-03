use std::path::PathBuf;
use std::sync::Mutex;
use windows::Win32::System::ApplicationInstallationAndServicing::*;
use windows::core::{PCWSTR, PWSTR, w};

use crate::sensor_service::protocol::{ALL, Operation, Payload, ServiceStatus};

static INSTALLING: Mutex<()> = Mutex::new(());

pub fn status() -> ServiceStatus {
  let can_configure = product_for_current_install().is_ok();
  let result = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .map_err(|e| e.to_string())
    .and_then(|runtime| {
      runtime.block_on(async {
        let (mut pipe, generation) = super::client::handshake().await?;
        let reply =
          super::client::exchange(&mut pipe, Operation::Sample { families: ALL }).await?;

        if reply.generation != generation {
          return Err("Sensor service restarted".to_string());
        }

        let Payload::Snapshot(snapshot) = reply.payload else {
          return Err("Unexpected sensor response".into());
        };

        Ok(
          snapshot
            .temperature
            .as_ref()
            .is_some_and(|v| v.value.cpu_temperature.is_some())
            || snapshot
              .power
              .as_ref()
              .is_some_and(|v| v.value.watts.is_some())
            || snapshot.motherboard.as_ref().is_some_and(|v| {
              !v.value.sample.temperatures.is_empty()
                || !v.value.sample.fan_speeds.is_empty()
            }),
        )
      })
    });

  match result {
    Ok(has_readings) => ServiceStatus {
      connected: true,
      has_readings,
      can_configure,
      detail: None,
    },
    Err(detail) => ServiceStatus {
      connected: false,
      has_readings: false,
      can_configure,
      detail: Some(detail),
    },
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
    let result = if MsiQueryFeatureStateW(product, w!("WindowsSensorService"))
      == INSTALLSTATE_LOCAL
    {
      MsiReinstallFeatureW(
        product,
        w!("WindowsSensorService"),
        REINSTALLMODE(REINSTALLMODE_FILEOLDERVERSION.0 | REINSTALLMODE_MACHINEDATA.0),
      )
    } else {
      MsiConfigureFeatureW(product, w!("WindowsSensorService"), INSTALLSTATE_LOCAL)
    };

    match result {
      0 | 3010 => Ok(()),
      1602 => Err("Sensor service setup was cancelled".into()),
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

    if result == 259 {
      break;
    }

    if result != 0 {
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

  if result != 0 {
    return Err(format!(
      "Cannot inspect Windows Installer product ({result})"
    ));
  }
  String::from_utf16(&value[..length as usize]).map_err(|e| e.to_string())
}
