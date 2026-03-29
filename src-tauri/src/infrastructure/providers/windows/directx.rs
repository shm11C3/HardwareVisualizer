use crate::models::hardware::GraphicInfo;
use crate::{log_debug, log_error, log_internal};

use dxgi::Factory;
use dxgi::adapter::AdapterDesc;
use tokio::task::spawn_blocking;

/// Lightweight LUID info for a GPU adapter, used to match PDH counters.
pub struct GpuLuidInfo {
  pub name: String,
  pub luid_high: i32,
  pub luid_low: u32,
}

/// Get Intel GPU LUID information for matching with PDH performance counters.
pub async fn get_intel_gpu_luid_info() -> Result<Vec<GpuLuidInfo>, String> {
  let handle = spawn_blocking(|| {
    let factory =
      Factory::new().map_err(|e| format!("Failed to create DXGI Factory: {e:?}"))?;
    let mut result = Vec::new();

    for adapter in factory.adapters() {
      let desc: AdapterDesc = adapter.get_desc();
      let gpu_name = desc.description();
      if gpu_name.contains("Intel") {
        let luid = desc.adapter_luid();
        result.push(GpuLuidInfo {
          name: gpu_name.trim_end_matches('\0').to_string(),
          luid_high: luid.HighPart,
          luid_low: luid.LowPart,
        });
      }
    }

    Ok(result)
  });

  handle.await.map_err(|e| {
    log_error!("join_error", "get_intel_gpu_luid_info", Some(e.to_string()));
    "Intel GPU LUID info retrieval failed".to_string()
  })?
}

/// Get Intel GPU LUID information, cached for the process lifetime.
/// Returns an empty slice if the initial query fails.
pub async fn get_intel_gpu_luid_info_cached() -> &'static [GpuLuidInfo] {
  static INFO: tokio::sync::OnceCell<Vec<GpuLuidInfo>> =
    tokio::sync::OnceCell::const_new();

  INFO
    .get_or_init(|| async { get_intel_gpu_luid_info().await.unwrap_or_default() })
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
