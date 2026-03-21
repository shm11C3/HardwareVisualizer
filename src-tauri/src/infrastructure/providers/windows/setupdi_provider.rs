///
/// Windows SetupDi provider — enumerates display adapters with PCI bus location.
///
/// Uses `SetupDiGetClassDevs` / `SetupDiEnumDeviceInfo` /
/// `SetupDiGetDeviceRegistryProperty` to retrieve each display adapter's
/// human-readable description together with its PCI Bus/Device/Function
/// address.  The description returned by `SPDRP_DEVICEDESC` matches the
/// string reported by DXGI (`DXGI_ADAPTER_DESC.Description`), so the
/// result can be used to correlate DXGI names with ADL names via BDF.
///
use crate::{log_debug, log_internal};

/// PCI Bus/Device/Function together with the Windows device description
/// (which matches the DXGI adapter name).
#[derive(Debug, Clone)]
pub struct DisplayAdapterBdf {
  pub description: String,
  pub bus: i32,
  pub device: i32,
  pub function: i32,
}

/// Enumerate all *present* display adapters and return their device
/// description and PCI BDF address.
pub fn enumerate_display_adapters() -> Vec<DisplayAdapterBdf> {
  use winapi::shared::devguid::GUID_DEVCLASS_DISPLAY;
  use winapi::shared::minwindef::{BYTE, DWORD, FALSE};
  use winapi::um::setupapi::{
    DIGCF_PRESENT, SP_DEVINFO_DATA, SPDRP_ADDRESS, SPDRP_BUSNUMBER, SPDRP_DEVICEDESC,
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
    SetupDiGetDeviceRegistryPropertyW,
  };

  unsafe {
    let dev_info = SetupDiGetClassDevsW(
      &GUID_DEVCLASS_DISPLAY,
      std::ptr::null(),
      std::ptr::null_mut(),
      DIGCF_PRESENT,
    );

    if dev_info.is_null() || dev_info == winapi::um::handleapi::INVALID_HANDLE_VALUE {
      log_debug!(
        "SetupDiGetClassDevsW failed",
        "setupdi_provider",
        None::<&str>
      );
      return Vec::new();
    }

    let mut result = Vec::new();
    let mut index: DWORD = 0;

    loop {
      let mut dev_info_data: SP_DEVINFO_DATA = std::mem::zeroed();
      dev_info_data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as DWORD;

      if SetupDiEnumDeviceInfo(dev_info, index, &mut dev_info_data) == FALSE {
        break;
      }
      index += 1;

      // --- Device description (= DXGI adapter name) ---
      let mut desc_buf = [0u16; 256];
      let mut reg_type: DWORD = 0;
      let mut required_size: DWORD = 0;
      if SetupDiGetDeviceRegistryPropertyW(
        dev_info,
        &mut dev_info_data,
        SPDRP_DEVICEDESC,
        &mut reg_type,
        desc_buf.as_mut_ptr() as *mut BYTE,
        (desc_buf.len() * std::mem::size_of::<u16>()) as DWORD,
        &mut required_size,
      ) == FALSE
      {
        continue;
      }
      let description = String::from_utf16_lossy(&desc_buf)
        .trim_end_matches('\0')
        .to_string();

      // --- PCI bus number ---
      let mut bus: DWORD = 0;
      if SetupDiGetDeviceRegistryPropertyW(
        dev_info,
        &mut dev_info_data,
        SPDRP_BUSNUMBER,
        &mut reg_type,
        &mut bus as *mut DWORD as *mut BYTE,
        std::mem::size_of::<DWORD>() as DWORD,
        &mut required_size,
      ) == FALSE
      {
        continue;
      }

      // --- PCI address (device << 16 | function) ---
      let mut address: DWORD = 0;
      if SetupDiGetDeviceRegistryPropertyW(
        dev_info,
        &mut dev_info_data,
        SPDRP_ADDRESS,
        &mut reg_type,
        &mut address as *mut DWORD as *mut BYTE,
        std::mem::size_of::<DWORD>() as DWORD,
        &mut required_size,
      ) == FALSE
      {
        continue;
      }

      let device_num = (address >> 16) as i32;
      let function_num = (address & 0xFFFF) as i32;

      result.push(DisplayAdapterBdf {
        description,
        bus: bus as i32,
        device: device_num,
        function: function_num,
      });
    }

    SetupDiDestroyDeviceInfoList(dev_info);

    log_debug!(
      &format!("enumerated {} display adapters", result.len()),
      "setupdi_provider",
      None::<&str>
    );

    result
  }
}
