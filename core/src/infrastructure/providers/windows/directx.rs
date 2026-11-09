use crate::models::hardware::GraphicInfo;
use crate::{log_error, log_warn};

use super::setupdi_provider;
use std::collections::HashMap;
use tokio::task::spawn_blocking;
use windows::Win32::Graphics::Dxgi::{
  CreateDXGIFactory1, DXGI_ADAPTER_DESC1, DXGI_ADAPTER_FLAG_SOFTWARE,
  DXGI_ERROR_NOT_FOUND, IDXGIFactory1,
};

pub(crate) struct AdapterDiscovery<T> {
  pub(crate) adapters: Vec<T>,
  pub(crate) error: Option<String>,
}

fn hardware_adapter_descriptions() -> Result<AdapterDiscovery<DXGI_ADAPTER_DESC1>, String>
{
  // The generated bindings own the COM interfaces and output buffers.
  let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }
    .map_err(|error| format!("CreateDXGIFactory1 failed: {error}"))?;
  Ok(discover_adapters(
    |index| match unsafe { factory.EnumAdapters1(index) } {
      Ok(adapter) => Ok(Some(adapter)),
      Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => Ok(None),
      Err(error) => Err(error),
    },
    |adapter| unsafe { adapter.GetDesc1() },
  ))
}

fn discover_adapters<A>(
  mut enumerate: impl FnMut(u32) -> windows::core::Result<Option<A>>,
  mut describe: impl FnMut(A) -> windows::core::Result<DXGI_ADAPTER_DESC1>,
) -> AdapterDiscovery<DXGI_ADAPTER_DESC1> {
  let mut adapters = Vec::new();
  let mut error = None;
  for index in 0.. {
    let adapter = match enumerate(index) {
      Ok(Some(adapter)) => adapter,
      Ok(None) => break,
      Err(cause) => {
        error = Some(format!("EnumAdapters1({index}) failed: {cause}"));
        break;
      }
    };
    match describe(adapter) {
      Ok(desc) if is_hardware_adapter(desc.Flags) => adapters.push(desc),
      Ok(_) => {}
      Err(cause) => {
        error.get_or_insert_with(|| format!("GetDesc1({index}) failed: {cause}"));
      }
    }
  }
  AdapterDiscovery { adapters, error }
}

fn is_hardware_adapter(flags: u32) -> bool {
  flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 == 0
}

fn adapter_name(desc: &DXGI_ADAPTER_DESC1) -> String {
  let end = desc
    .Description
    .iter()
    .position(|&unit| unit == 0)
    .unwrap_or(desc.Description.len());
  String::from_utf16_lossy(&desc.Description[..end])
}

/// Lightweight LUID info for a GPU adapter, used to match PDH counters.
pub struct GpuLuidInfo {
  pub name: String,
  pub luid_high: i32,
  pub luid_low: u32,
  /// PnP identity, when SetupDi can associate it with this DXGI LUID.
  pub device_instance_id: Option<String>,
  /// PCI vendor id as DXGI reports it, used to tell which vendor API — if
  /// any — already speaks for this adapter.
  pub vendor_id: u32,
  /// PCI address, when SetupDi can associate it with this DXGI LUID. It is
  /// the id ADL samples are keyed by, so it is how a PDH candidate
  /// recognises an adapter ADL already reported.
  pub bdf: Option<(i32, i32, i32)>,
}

/// LUID information for every hardware GPU adapter Windows exposes.
///
/// Deliberately vendor-agnostic: PDH is the only usage source left for an
/// adapter whose vendor API is missing or refuses to answer — an AMD APU
/// whose ADL usage query fails, for one — and an adapter absent from the
/// sample stream cannot be named, attributed, or selected at all.
pub async fn get_gpu_luid_info() -> Result<Vec<GpuLuidInfo>, String> {
  let handle = spawn_blocking(|| {
    let AdapterDiscovery { adapters, error } = hardware_adapter_descriptions()?;
    // Live discovery is cached for the process lifetime and must be complete.
    if let Some(error) = error {
      return Err(error);
    }
    let pnp_by_luid: HashMap<(i32, u32), setupdi_provider::DisplayAdapterBdf> =
      setupdi_provider::enumerate_display_adapters()
        .into_iter()
        .filter_map(|adapter| Some((adapter.adapter_luid?, adapter)))
        .collect();
    let mut result = Vec::with_capacity(adapters.len());

    for desc in adapters {
      let luid = desc.AdapterLuid;
      let pnp = pnp_by_luid.get(&(luid.HighPart, luid.LowPart));
      result.push(GpuLuidInfo {
        name: adapter_name(&desc),
        luid_high: luid.HighPart,
        luid_low: luid.LowPart,
        device_instance_id: pnp.and_then(|a| a.device_instance_id.clone()),
        vendor_id: desc.VendorId,
        bdf: pnp.map(|a| (a.bus, a.device, a.function)),
      });
    }

    Ok(result)
  });

  handle.await.map_err(|e| {
    log_error!("join_error", "get_gpu_luid_info", Some(e.to_string()));
    "GPU LUID info retrieval failed".to_string()
  })?
}

/// Get GPU LUID information, cached for the process lifetime.
///
/// Only a successful query is cached. A failure returns an empty slice for
/// this call and leaves the cell uninitialized so the next sample retries.
pub async fn get_gpu_luid_info_cached() -> &'static [GpuLuidInfo] {
  static INFO: tokio::sync::OnceCell<Vec<GpuLuidInfo>> =
    tokio::sync::OnceCell::const_new();
  static FAILURE_LOGGED: std::sync::Once = std::sync::Once::new();

  match INFO.get_or_try_init(get_gpu_luid_info).await {
    Ok(info) => info,
    Err(error) => {
      // Once, not per sample: the retry runs at the monitor cadence, and a
      // machine where discovery never succeeds would flood the log.
      FAILURE_LOGGED.call_once(|| {
        log_warn!("discovery_failed", "get_gpu_luid_info_cached", Some(error));
      });
      &[]
    }
  }
}

pub(crate) struct InventoryAdapter {
  pub(crate) info: GraphicInfo,
  pub(crate) vendor_id: u32,
  // Older AMD/Intel inventory rows used only the device/model number.
  pub(crate) legacy_device_id: Option<String>,
}

pub(crate) async fn get_gpu_inventory()
-> Result<AdapterDiscovery<InventoryAdapter>, String> {
  spawn_blocking(|| {
    let AdapterDiscovery { adapters, error } = hardware_adapter_descriptions()?;
    Ok(AdapterDiscovery {
      adapters: adapters
        .into_iter()
        .map(|desc| {
          let vendor_id = desc.VendorId;
          let vendor_name = match vendor_id {
            0x1002 => "AMD".into(),
            0x10de => "NVIDIA".into(),
            0x8086 => "Intel".into(),
            0 => "Unknown".into(),
            id => format!("0x{id:04X}"),
          };
          let luid = desc.AdapterLuid;
          InventoryAdapter {
            info: GraphicInfo {
              id: format!("dxgi:{}:{}", luid.HighPart, luid.LowPart),
              name: adapter_name(&desc),
              vendor_name,
              clock: 0,
              memory_size: format!("{} MB", desc.SharedSystemMemory / (1024 * 1024)),
              memory_size_dedicated: format!(
                "{} MB",
                desc.DedicatedVideoMemory / (1024 * 1024)
              ),
              core_count: None,
            },
            vendor_id,
            legacy_device_id: (matches!(vendor_id, 0x1002 | 0x8086)
              && desc.DeviceId != 0)
              .then(|| desc.DeviceId.to_string()),
          }
        })
        .collect(),
      error,
    })
  })
  .await
  .map_err(|e| format!("DXGI GPU inventory task failed: {e}"))?
}

#[cfg(test)]
mod tests {
  use super::*;
  use windows::Win32::Foundation::{E_FAIL, LUID};

  #[test]
  fn excludes_the_software_bit_without_filtering_other_flags() {
    assert!(is_hardware_adapter(0));
    assert!(is_hardware_adapter(1));
    assert!(!is_hardware_adapter(2));
    assert!(!is_hardware_adapter(3));
  }

  #[test]
  fn descriptor_failure_preserves_hardware_before_and_after_the_failure() {
    let discovery = discover_adapters(
      |index| Ok((index < 4).then_some(index)),
      |index| match index {
        1 => Err(E_FAIL.into()),
        _ => Ok(DXGI_ADAPTER_DESC1 {
          AdapterLuid: LUID {
            HighPart: -1,
            LowPart: index,
          },
          Flags: if index == 2 {
            DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32
          } else {
            0
          },
          ..Default::default()
        }),
      },
    );

    assert!(discovery.error.unwrap().starts_with("GetDesc1(1)"));
    let luids: Vec<_> = discovery
      .adapters
      .iter()
      .map(|desc| desc.AdapterLuid.LowPart)
      .collect();
    assert_eq!(luids, [0, 3]);
  }

  #[test]
  fn enumeration_failure_stops_and_preserves_the_discovered_prefix() {
    let discovery = discover_adapters(
      |index| match index {
        0 => Ok(Some(())),
        1 => Err(E_FAIL.into()),
        _ => panic!("Enumeration must stop on an unexpected DXGI error"),
      },
      |()| Ok(DXGI_ADAPTER_DESC1::default()),
    );

    assert!(discovery.error.unwrap().starts_with("EnumAdapters1(1)"));
    assert_eq!(discovery.adapters.len(), 1);
  }
}
