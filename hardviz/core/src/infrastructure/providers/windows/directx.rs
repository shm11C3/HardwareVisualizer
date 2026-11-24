use crate::models::hardware::GraphicInfo;
use crate::{log_error, log_warn};

use super::setupdi_provider;
use std::collections::{HashMap, HashSet};
use tokio::task::spawn_blocking;
use windows::Wdk::Graphics::Direct3D::{
  D3DKMT_ADAPTERTYPE, D3DKMT_CLOSEADAPTER, D3DKMT_OPENADAPTERFROMLUID,
  D3DKMT_PHYSICAL_ADAPTER_COUNT, D3DKMT_PNP_KEY_HARDWARE,
  D3DKMT_QUERY_PHYSICAL_ADAPTER_PNP_KEY, D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter,
  D3DKMTOpenAdapterFromLuid, D3DKMTQueryAdapterInfo, KMTQAITYPE_ADAPTERTYPE,
  KMTQAITYPE_PHYSICALADAPTERCOUNT, KMTQAITYPE_PHYSICALADAPTERPNPKEY,
  KMTQUERYADAPTERINFOTYPE,
};
use windows::Win32::Foundation::{LUID, STATUS_SUCCESS};
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

/// LUID information for hardware GPU adapters, excluding proven display aliases.
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
    Ok(live_adapter_info(
      adapters,
      &pnp_by_luid,
      physical_adapter_identity,
    ))
  });

  handle.await.map_err(|e| {
    log_error!("join_error", "get_gpu_luid_info", Some(e.to_string()));
    "GPU LUID info retrieval failed".to_string()
  })?
}

fn live_adapter_info(
  adapters: Vec<DXGI_ADAPTER_DESC1>,
  pnp_by_luid: &HashMap<(i32, u32), setupdi_provider::DisplayAdapterBdf>,
  identify: impl FnMut(LUID) -> Option<PhysicalAdapterIdentity>,
) -> Vec<GpuLuidInfo> {
  let indirect_aliases = identify_indirect_aliases(&adapters, identify);
  adapters
    .into_iter()
    .zip(indirect_aliases)
    .filter(|(_, is_indirect_alias)| !is_indirect_alias)
    .map(|(desc, _)| {
      let luid = desc.AdapterLuid;
      let pnp = pnp_by_luid.get(&(luid.HighPart, luid.LowPart));
      GpuLuidInfo {
        name: adapter_name(&desc),
        luid_high: luid.HighPart,
        luid_low: luid.LowPart,
        device_instance_id: pnp.and_then(|a| a.device_instance_id.clone()),
        vendor_id: desc.VendorId,
        bdf: pnp.map(|a| (a.bus, a.device, a.function)),
      }
    })
    .collect()
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
  pub(crate) is_indirect_alias: bool,
  // Older AMD/Intel inventory rows used only the device/model number.
  pub(crate) legacy_device_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PhysicalAdapterRole {
  Renderer,
  IndirectDisplayOnly,
}

struct PhysicalAdapterIdentity {
  role: PhysicalAdapterRole,
  hardware_key: Vec<u16>,
}

fn identify_indirect_aliases(
  adapters: &[DXGI_ADAPTER_DESC1],
  mut identify: impl FnMut(LUID) -> Option<PhysicalAdapterIdentity>,
) -> Vec<bool> {
  let identities: Vec<_> = adapters
    .iter()
    .map(|desc| identify(desc.AdapterLuid))
    .collect();
  let renderers: HashSet<_> = identities
    .iter()
    .flatten()
    .filter(|identity| identity.role == PhysicalAdapterRole::Renderer)
    .map(|identity| identity.hardware_key.as_slice())
    .collect();
  identities
    .iter()
    .map(|identity| {
      // A different LUID can be an indirect display alias of a renderer.
      // Only the OS-reported physical key can establish this relationship.
      identity.as_ref().is_some_and(|identity| {
        identity.role == PhysicalAdapterRole::IndirectDisplayOnly
          && renderers.contains(identity.hardware_key.as_slice())
      })
    })
    .collect()
}

fn physical_adapter_identity(luid: LUID) -> Option<PhysicalAdapterIdentity> {
  let adapter = KmtAdapter::open(luid)?;
  let mut adapter_type = D3DKMT_ADAPTERTYPE::default();
  // SAFETY: The query kinds match the initialized SDK output structures.
  unsafe { adapter.query(KMTQAITYPE_ADAPTERTYPE, &mut adapter_type) }?;
  let role = physical_adapter_role(unsafe { adapter_type.Anonymous.Value })?;
  let mut physical_count = D3DKMT_PHYSICAL_ADAPTER_COUNT::default();
  unsafe { adapter.query(KMTQAITYPE_PHYSICALADAPTERCOUNT, &mut physical_count) }?;
  if physical_count.Count != 1 {
    return None;
  }

  let mut buffer = [0u16; 512];
  let mut length = buffer.len() as u32;
  let mut key = D3DKMT_QUERY_PHYSICAL_ADAPTER_PNP_KEY {
    PhysicalAdapterIndex: 0,
    PnPKeyType: D3DKMT_PNP_KEY_HARDWARE,
    pDest: windows::core::PWSTR(buffer.as_mut_ptr()),
    pCchDest: &mut length,
  };
  // SAFETY: The key request points to a writable buffer and its length.
  unsafe { adapter.query(KMTQAITYPE_PHYSICALADAPTERPNPKEY, &mut key) }?;
  let [hardware_key @ .., 0] = buffer.get(..length as usize)? else {
    return None;
  };
  if hardware_key.is_empty() || hardware_key.contains(&0) {
    return None;
  }
  Some(PhysicalAdapterIdentity {
    role,
    hardware_key: hardware_key.to_vec(),
  })
}

fn physical_adapter_role(flags: u32) -> Option<PhysicalAdapterRole> {
  // D3DKMT_ADAPTERTYPE exposes these SDK bitfields through its Value member.
  const RENDER_SUPPORTED: u32 = 1 << 0;
  const DISPLAY_SUPPORTED: u32 = 1 << 1;
  const INDIRECT_DISPLAY_DEVICE: u32 = 1 << 6;
  const COMPUTE_ONLY: u32 = 1 << 11;
  const INDIRECT_DISPLAY: u32 = DISPLAY_SUPPORTED | INDIRECT_DISPLAY_DEVICE;

  if flags & RENDER_SUPPORTED != 0 {
    Some(PhysicalAdapterRole::Renderer)
  } else if flags & INDIRECT_DISPLAY == INDIRECT_DISPLAY && flags & COMPUTE_ONLY == 0 {
    Some(PhysicalAdapterRole::IndirectDisplayOnly)
  } else {
    None
  }
}

struct KmtAdapter(u32);

impl KmtAdapter {
  fn open(luid: LUID) -> Option<Self> {
    let mut open = D3DKMT_OPENADAPTERFROMLUID {
      AdapterLuid: luid,
      ..Default::default()
    };
    (unsafe { D3DKMTOpenAdapterFromLuid(&mut open) } == STATUS_SUCCESS)
      .then(|| Self(open.hAdapter))
  }

  /// The caller pairs the query type with its SDK payload and valid pointers.
  unsafe fn query<T>(&self, kind: KMTQUERYADAPTERINFOTYPE, data: &mut T) -> Option<()> {
    let mut query = D3DKMT_QUERYADAPTERINFO {
      hAdapter: self.0,
      Type: kind,
      pPrivateDriverData: std::ptr::from_mut(data).cast(),
      PrivateDriverDataSize: size_of::<T>() as u32,
    };
    (unsafe { D3DKMTQueryAdapterInfo(&mut query) } == STATUS_SUCCESS).then_some(())
  }
}

impl Drop for KmtAdapter {
  fn drop(&mut self) {
    let _ = unsafe { D3DKMTCloseAdapter(&D3DKMT_CLOSEADAPTER { hAdapter: self.0 }) };
  }
}

pub(crate) async fn get_gpu_inventory()
-> Result<AdapterDiscovery<InventoryAdapter>, String> {
  spawn_blocking(|| {
    let AdapterDiscovery { adapters, error } = hardware_adapter_descriptions()?;
    let indirect_aliases =
      identify_indirect_aliases(&adapters, physical_adapter_identity);
    Ok(AdapterDiscovery {
      adapters: adapters
        .into_iter()
        .zip(indirect_aliases)
        .map(|(desc, is_indirect_alias)| {
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
            is_indirect_alias,
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
  use windows::Win32::Foundation::E_FAIL;

  #[test]
  fn identifies_only_indirect_adapters_without_render_or_compute_support() {
    assert_eq!(
      physical_adapter_role(0x31b),
      Some(PhysicalAdapterRole::Renderer)
    );
    assert_eq!(
      physical_adapter_role(0x342),
      Some(PhysicalAdapterRole::IndirectDisplayOnly)
    );
    assert_eq!(
      physical_adapter_role(0x343),
      Some(PhysicalAdapterRole::Renderer)
    );
    assert!(physical_adapter_role(0x342 | (1 << 11)).is_none());
    assert!(physical_adapter_role(0x340).is_none());
    assert!(physical_adapter_role(0x02).is_none());
  }

  #[test]
  fn marks_only_indirect_display_aliases_with_a_matching_physical_renderer() {
    let adapters: Vec<_> = (1..=5)
      .map(|low| DXGI_ADAPTER_DESC1 {
        AdapterLuid: LUID {
          HighPart: 0,
          LowPart: low,
        },
        VendorId: if low == 5 { 0x17cb } else { 0x10de },
        DeviceId: 0x2c02,
        ..Default::default()
      })
      .collect();
    let identify = |luid: LUID| {
      use PhysicalAdapterRole::*;
      let (role, hardware_key) = match luid.LowPart {
        1 => (IndirectDisplayOnly, r"\Registry\Machine\Fixture\GpuA"),
        2 => (Renderer, r"\Registry\Machine\Fixture\GpuA"),
        3 => (Renderer, r"\Registry\Machine\Fixture\GpuB"),
        4 => (IndirectDisplayOnly, r"\Registry\Machine\Fixture\GpuC"),
        _ => return None,
      };
      Some(PhysicalAdapterIdentity {
        role,
        hardware_key: hardware_key.encode_utf16().collect(),
      })
    };

    let aliases = identify_indirect_aliases(&adapters, identify);
    assert_eq!(aliases, [true, false, false, false, false]);

    let instance_id = r"PCI\VEN_10DE&DEV_2C02\4&12345678&0&0008";
    let pnp = HashMap::from([(
      (0, 2),
      setupdi_provider::DisplayAdapterBdf {
        description: "Renderer".into(),
        bus: 1,
        device: 0,
        function: 0,
        adapter_luid: Some((0, 2)),
        device_instance_id: Some(instance_id.into()),
      },
    )]);
    let live = live_adapter_info(adapters, &pnp, identify);
    let luids: Vec<_> = live
      .iter()
      .map(|gpu| (gpu.luid_high, gpu.luid_low))
      .collect();
    assert_eq!(luids, [(0, 2), (0, 3), (0, 4), (0, 5)]);
    assert_eq!(live[0].device_instance_id.as_deref(), Some(instance_id));
    assert_eq!(live[0].bdf, Some((1, 0, 0)));
    assert_eq!(live[1].device_instance_id, None);
  }

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
