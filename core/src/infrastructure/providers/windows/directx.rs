use crate::models::hardware::GraphicInfo;
use crate::{log_error, log_warn};

use super::setupdi_provider;
use dxgi::Factory;
use dxgi::adapter::AdapterDesc;
use std::collections::HashMap;
use tokio::task::spawn_blocking;

/// `DXGI_ADAPTER_FLAG_SOFTWARE`: a software rasterizer such as WARP or the
/// "Microsoft Basic Render Driver". It has no device behind it to report on,
/// so offering it as a GPU to attribute readings to would be a fiction.
const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;

fn hardware_adapter_descriptions() -> Result<Vec<AdapterDesc>, String> {
  let factory =
    Factory::new().map_err(|e| format!("Failed to create DXGI Factory: {e:?}"))?;
  Ok(
    factory
      .adapters()
      .map(|adapter| adapter.get_desc())
      .filter(|desc| is_hardware_adapter(desc.flags()))
      .collect(),
  )
}

fn is_hardware_adapter(flags: u32) -> bool {
  flags & DXGI_ADAPTER_FLAG_SOFTWARE == 0
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
    let adapters = hardware_adapter_descriptions()?;
    let pnp_by_luid: HashMap<(i32, u32), setupdi_provider::DisplayAdapterBdf> =
      setupdi_provider::enumerate_display_adapters()
        .into_iter()
        .filter_map(|adapter| Some((adapter.adapter_luid?, adapter)))
        .collect();
    let mut result = Vec::new();

    for desc in adapters {
      let luid = desc.adapter_luid();
      let pnp = pnp_by_luid.get(&(luid.HighPart, luid.LowPart));
      result.push(GpuLuidInfo {
        name: desc.description().trim_end_matches('\0').to_string(),
        luid_high: luid.HighPart,
        luid_low: luid.LowPart,
        device_instance_id: pnp.and_then(|a| a.device_instance_id.clone()),
        vendor_id: desc.vendor_id(),
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
/// this call and leaves the cell uninitialized so the next sample retries:
/// caching a transient DXGI or SetupDi failure would declare every
/// vendor-uncovered adapter absent for the rest of the process, and absence
/// of discovery is uncertainty, not evidence of absence.
pub async fn get_gpu_luid_info_cached() -> &'static [GpuLuidInfo] {
  static INFO: tokio::sync::OnceCell<Vec<GpuLuidInfo>> =
    tokio::sync::OnceCell::const_new();
  static FAILURE_LOGGED: std::sync::Once = std::sync::Once::new();

  match INFO.get_or_try_init(get_gpu_luid_info).await {
    Ok(info) => info,
    Err(e) => {
      // Once, not per sample: the retry runs at the monitor cadence, and a
      // machine where discovery never succeeds would flood the log.
      FAILURE_LOGGED.call_once(|| {
        log_warn!("discovery_failed", "get_gpu_luid_info_cached", Some(e));
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

pub(crate) async fn get_gpu_inventory() -> Result<Vec<InventoryAdapter>, String> {
  spawn_blocking(|| {
    Ok(
      hardware_adapter_descriptions()?
        .into_iter()
        .map(|desc| {
          let vendor_id = desc.vendor_id();
          let vendor_name = match vendor_id {
            0x1002 => "AMD".into(),
            0x10de => "NVIDIA".into(),
            0x8086 => "Intel".into(),
            0 => "Unknown".into(),
            id => format!("0x{id:04X}"),
          };
          let luid = desc.adapter_luid();
          InventoryAdapter {
            info: GraphicInfo {
              id: format!("dxgi:{}:{}", luid.HighPart, luid.LowPart),
              name: desc.description(),
              vendor_name,
              clock: 0,
              memory_size: format!("{} MB", desc.shared_system_memory() / (1024 * 1024)),
              memory_size_dedicated: format!(
                "{} MB",
                desc.dedicated_video_memory() / (1024 * 1024)
              ),
              core_count: None,
            },
            vendor_id,
            legacy_device_id: (matches!(vendor_id, 0x1002 | 0x8086)
              && desc.device_id() != 0)
              .then(|| desc.device_id().to_string()),
          }
        })
        .collect(),
    )
  })
  .await
  .map_err(|e| format!("DXGI GPU inventory task failed: {e}"))?
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn excludes_the_software_bit_without_filtering_other_flags() {
    assert!(is_hardware_adapter(0));
    assert!(is_hardware_adapter(1));
    assert!(!is_hardware_adapter(2));
    assert!(!is_hardware_adapter(3));
  }
}
