use crate::models::hardware::GraphicInfo;
use crate::{log_debug, log_error};

use super::setupdi_provider;
use dxgi::Factory;
use dxgi::adapter::AdapterDesc;
use std::collections::HashMap;
use tokio::task::spawn_blocking;

/// `DXGI_ADAPTER_FLAG_SOFTWARE`: a software rasterizer such as WARP or the
/// "Microsoft Basic Render Driver". It has no device behind it to report on,
/// so offering it as a GPU to attribute readings to would be a fiction.
const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;

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
    let factory =
      Factory::new().map_err(|e| format!("Failed to create DXGI Factory: {e:?}"))?;
    let pnp_by_luid: HashMap<(i32, u32), setupdi_provider::DisplayAdapterBdf> =
      setupdi_provider::enumerate_display_adapters()
        .into_iter()
        .filter_map(|adapter| Some((adapter.adapter_luid?, adapter)))
        .collect();
    let mut result = Vec::new();

    for adapter in factory.adapters() {
      let desc: AdapterDesc = adapter.get_desc();
      if desc.flags() & DXGI_ADAPTER_FLAG_SOFTWARE != 0 {
        continue;
      }

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
/// Returns an empty slice if the initial query fails.
pub async fn get_gpu_luid_info_cached() -> &'static [GpuLuidInfo] {
  static INFO: tokio::sync::OnceCell<Vec<GpuLuidInfo>> =
    tokio::sync::OnceCell::const_new();

  INFO
    .get_or_init(|| async { get_gpu_luid_info().await.unwrap_or_default() })
    .await
}

/// Get Intel GPU information
pub async fn get_intel_gpu_info() -> Result<Vec<GraphicInfo>, String> {
  let handle = spawn_blocking(|| {
    log_debug!("start", "get_intel_gpu_info", None::<&str>);

    // Create DXGI factory instance
    let factory = Factory::new().expect("Failed to create DXGI Factory");
    let mut gpu_info_list = Vec::new();

    // Enumerate adapters
    for adapter in factory.adapters() {
      // Unwrap get_desc() result and handle errors
      let desc: AdapterDesc = adapter.get_desc();

      // Add only Intel GPUs
      let gpu_name = desc.description();
      if gpu_name.contains("Intel") {
        let memory_size_dedicated = desc.dedicated_video_memory() / 1024 / 1024;
        let memory_size_shared = desc.shared_system_memory() / 1024 / 1024;

        let gpu_info = GraphicInfo {
          id: desc.device_id().to_string(),
          name: gpu_name.trim_end_matches('\0').to_string(),
          vendor_name: "Intel".to_string(),
          clock: 0, // Set to 0 because Intel clock frequency is difficult to obtain
          memory_size: format!("{memory_size_shared} MB"),
          memory_size_dedicated: format!("{memory_size_dedicated} MB"),
          core_count: None,
        };

        gpu_info_list.push(gpu_info);
      }
    }

    log_debug!("end", "get_intel_gpu_info", None::<&str>);
    Ok(gpu_info_list)
  });

  handle.await.map_err(|e| {
    log_error!("join_error", "get_intel_gpu_info", Some(e.to_string()));
    "Intel GPU info retrieval failed".to_string()
  })?
}

/// Get AMD GPU information
pub async fn get_amd_gpu_info() -> Result<Vec<GraphicInfo>, String> {
  let handle = spawn_blocking(|| {
    log_debug!("start", "get_amd_gpu_info", None::<&str>);

    // Create DXGI factory instance
    let factory = Factory::new().expect("Failed to create DXGI Factory");
    let mut gpu_info_list = Vec::new();

    // Enumerate adapters
    for adapter in factory.adapters() {
      // Unwrap get_desc() result and handle errors
      let desc: AdapterDesc = adapter.get_desc();

      // Get GPU name
      let gpu_name = desc.description();
      if gpu_name.contains("AMD") || gpu_name.contains("Radeon") {
        let memory_size_dedicated = desc.dedicated_video_memory() / 1024 / 1024;
        let memory_size_shared = desc.shared_system_memory() / 1024 / 1024;

        let gpu_info = GraphicInfo {
          id: desc.device_id().to_string(),
          name: gpu_name.trim_end_matches('\0').to_string(),
          vendor_name: "AMD".to_string(),
          clock: 0, // Set to 0 because clock frequency is difficult to obtain
          memory_size: format!("{memory_size_shared} MB"),
          memory_size_dedicated: format!("{memory_size_dedicated} MB"),
          core_count: None,
        };

        gpu_info_list.push(gpu_info);
      }
    }

    log_debug!("end", "get_amd_gpu_info", None::<&str>);
    Ok(gpu_info_list)
  });

  handle.await.map_err(|e| {
    log_error!("join_error", "get_amd_gpu_info", Some(e.to_string()));
    "AMD GPU info retrieval failed".to_string()
  })?
}
