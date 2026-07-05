use crate::infrastructure;
use crate::models::GpuSample;
use crate::{log_error, log_warn};

pub async fn get_gpu_usage() -> Result<(f32, String), String> {
  use infrastructure::providers::pdh_provider::GpuEngineType;

  // 1. NVAPI (NVIDIA)
  if let Ok(usage) =
    infrastructure::providers::nvapi_provider::get_nvidia_gpu_usage().await
  {
    return Ok(((usage * 100.0).round(), "NVAPI".to_string()));
  }

  // 2. ADL (AMD) – dedicated API for AMD GPUs
  if infrastructure::providers::adl_provider::is_available() {
    if let Ok(usage) = infrastructure::providers::adl_provider::get_amd_gpu_usage().await
    {
      return Ok((usage.round(), "ADL".to_string()));
    }
    log_warn!(
      "adl_fallback",
      "get_gpu_usage",
      Some("ADL usage query failed, falling back to PDH")
    );
  }

  // 3. Intel via PDH + DXGI LUID (cached)
  let intel_gpus =
    infrastructure::providers::directx::get_intel_gpu_luid_info_cached().await;
  if let Some(gpu) = intel_gpus.first()
    && let Ok(usage) =
      infrastructure::providers::pdh_provider::query_gpu_usage_by_luid_and_engine(
        gpu.luid_high,
        gpu.luid_low,
        GpuEngineType::Graphics3D,
      )
      .await
  {
    return Ok(((usage * 100.0).round(), "PDH (Intel)".to_string()));
  }

  // 4. PDH (generic fallback using Windows Performance Counters)
  match infrastructure::providers::pdh_provider::query_gpu_usage_by_device_and_engine(
    GpuEngineType::Graphics3D,
  )
  .await
  {
    Ok(usage) => Ok(((usage * 100.0).round(), "PDH".to_string())),
    Err(e) => Err(format!(
      "Failed to get GPU usage from NVIDIA API, AMD ADL, and PDH: {e:?}"
    )),
  }
}

/// Always returns raw degrees Celsius. Presentation conversion lives at
/// the App-side boundary.
pub async fn get_gpu_temperature()
-> Result<Vec<crate::models::hardware::NameValue>, String> {
  let mut all_temps: Vec<crate::models::hardware::NameValue> = Vec::new();

  // 1. NVAPI (NVIDIA)
  if let Ok(nvidia_temps) =
    infrastructure::providers::nvapi_provider::get_nvidia_gpu_temperature().await
  {
    for temp in &nvidia_temps {
      all_temps.push(crate::models::hardware::NameValue {
        name: temp.name.clone(),
        value: temp.value,
      });
    }
  }

  // 2. ADL (AMD)
  if let Ok(amd_temps) =
    infrastructure::providers::adl_provider::get_amd_gpu_temperatures().await
  {
    for temp in &amd_temps {
      all_temps.push(crate::models::hardware::NameValue {
        name: temp.name.clone(),
        value: temp.value,
      });
    }
  }

  if all_temps.is_empty() {
    Err("Failed to get GPU temperature from any provider".to_string())
  } else {
    Ok(all_temps)
  }
}

pub async fn get_gpu_info() -> Result<Vec<crate::models::hardware::GraphicInfo>, String> {
  let (nvidia_res, amd_res, intel_res) = tokio::join!(
    infrastructure::providers::nvapi_provider::get_nvidia_gpu_info(),
    infrastructure::providers::directx::get_amd_gpu_info(),
    infrastructure::providers::directx::get_intel_gpu_info(),
  );

  fn append(
    tag: &str,
    result: Result<Vec<crate::models::hardware::GraphicInfo>, String>,
    acc: &mut Vec<crate::models::hardware::GraphicInfo>,
  ) {
    match result {
      Ok(list) => acc.extend(list),
      Err(e) => log_error!(tag, "get_gpu_info", Some(e.clone())),
    }
  }

  let mut gpus = Vec::new();

  append("nvidia_error", nvidia_res, &mut gpus);
  append("amd_error", amd_res, &mut gpus);
  append("intel_error", intel_res, &mut gpus);

  Ok(gpus)
}

pub async fn sample_gpus() -> Vec<GpuSample> {
  use crate::infrastructure::providers::nvapi_provider;
  use nvapi::PhysicalGpu;

  let mut gpu_metrics: Vec<GpuSample> = Vec::new();

  if let Some(nvapi_metrics) = PhysicalGpu::enumerate().ok().map(|gpus| {
    gpus
      .iter()
      .map(|gpu| {
        let name = gpu.full_name().unwrap_or_else(|_| "Unknown".to_string());
        let usage = nvapi_provider::get_gpu_usage_from_physical_gpu(gpu);
        let temperature =
          nvapi_provider::get_gpu_temperature_from_physical_gpu(gpu) as f32;
        let memory_usage =
          nvapi_provider::get_gpu_dedicated_memory_usage_from_physical_gpu(gpu) as f32;
        let cooler_level = nvapi_provider::get_gpu_cooler_level_from_physical_gpu(gpu);
        GpuSample {
          gpu_id: format!("nvapi:{}", gpu.gpu_id().unwrap_or(0)),
          name,
          usage: Some(usage),
          temperature: Some(temperature),
          dedicated_memory_kb: Some(memory_usage),
          cooler_level,
          source: "NVAPI".to_string(),
        }
      })
      .collect::<Vec<_>>()
  }) {
    gpu_metrics.extend(nvapi_metrics);
  }

  if crate::infrastructure::providers::adl_provider::is_available() {
    sample_amd_gpus(&mut gpu_metrics).await;
  }

  sample_intel_gpus(&mut gpu_metrics).await;

  gpu_metrics
}

/// Build a lookup table that maps PCI BDF to DXGI device description
/// (= the canonical GPU name used by `get_gpu_info` / `GraphicInfo`).
///
/// The table is computed once via SetupDi and cached for the lifetime of
/// the process. SetupDi calls are blocking Win32 APIs, so the first
/// invocation offloads them to the Tokio blocking thread pool.
async fn bdf_to_dxgi_name() -> &'static std::collections::HashMap<(i32, i32, i32), String>
{
  use crate::infrastructure::providers::setupdi_provider;

  static MAP: tokio::sync::OnceCell<std::collections::HashMap<(i32, i32, i32), String>> =
    tokio::sync::OnceCell::const_new();

  MAP
    .get_or_init(|| async {
      match tokio::task::spawn_blocking(|| {
        let adapters = setupdi_provider::enumerate_display_adapters();
        adapters
          .into_iter()
          .map(|a| ((a.bus, a.device, a.function), a.description))
          .collect()
      })
      .await
      {
        Ok(map) => map,
        Err(e) => {
          log_error!(
            &format!("SetupDi enumeration task failed: {e}"),
            "platform::windows::gpu::bdf_to_dxgi_name",
            None::<&str>
          );
          std::collections::HashMap::new()
        }
      }
    })
    .await
}

/// Resolve a GPU's canonical name using the Windows BDF to DXGI name map.
///
/// Returns the mapped DXGI name on hit, or the original adapter name on miss.
fn resolve_gpu_name_from_bdf_map(
  bdf_map: &std::collections::HashMap<(i32, i32, i32), String>,
  adapter_name: &str,
  bus: i32,
  device: i32,
  function: i32,
) -> String {
  bdf_map
    .get(&(bus, device, function))
    .cloned()
    .unwrap_or_else(|| adapter_name.to_string())
}

/// Collect AMD GPU usage and temperature via ADL.
/// VRAM usage is not available via ADL.
async fn sample_amd_gpus(gpu_metrics: &mut Vec<GpuSample>) {
  use crate::infrastructure::providers::adl_provider;

  let usages = adl_provider::get_amd_gpu_usage_per_adapter()
    .await
    .unwrap_or_default();

  if usages.is_empty() {
    return;
  }

  let temps = adl_provider::get_amd_gpu_temperatures_per_adapter()
    .await
    .unwrap_or_default();

  let temp_map: std::collections::HashMap<(i32, i32, i32), f32> = temps
    .iter()
    .map(|m| ((m.bus, m.device, m.function), m.value))
    .collect();

  let bdf_map = bdf_to_dxgi_name().await;

  for metric in &usages {
    let bdf = (metric.bus, metric.device, metric.function);
    let temperature = temp_map.get(&bdf).copied();
    let name = resolve_gpu_name_from_bdf_map(
      bdf_map,
      &metric.adapter_name,
      metric.bus,
      metric.device,
      metric.function,
    );
    gpu_metrics.push(GpuSample {
      gpu_id: format!("pci:{}:{}:{}", metric.bus, metric.device, metric.function),
      name,
      usage: Some(metric.value),
      temperature,
      dedicated_memory_kb: None,
      cooler_level: None,
      source: "ADL".to_string(),
    });
  }
}

/// Collect Intel GPU usage via PDH performance counters, filtered by LUID.
async fn sample_intel_gpus(gpu_metrics: &mut Vec<GpuSample>) {
  use crate::infrastructure::providers::pdh_provider::{self, GpuEngineType};

  let intel_gpus =
    crate::infrastructure::providers::directx::get_intel_gpu_luid_info_cached().await;
  if intel_gpus.is_empty() {
    return;
  }

  for gpu in intel_gpus {
    let usage = pdh_provider::query_gpu_usage_by_luid_and_engine(
      gpu.luid_high,
      gpu.luid_low,
      GpuEngineType::Graphics3D,
    )
    .await
    .ok()
    .map(|v| (v * 100.0).round());

    gpu_metrics.push(GpuSample {
      gpu_id: format!("pdh:{}", gpu.name),
      name: gpu.name.clone(),
      usage,
      temperature: None,
      dedicated_memory_kb: None,
      cooler_level: None,
      source: "PDH".to_string(),
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resolve_gpu_name_returns_dxgi_name_on_bdf_hit() {
    let mut map = std::collections::HashMap::new();
    map.insert((3, 0, 0), "AMD Radeon RX 7900 XTX".to_string());
    let result = resolve_gpu_name_from_bdf_map(&map, "Radeon RX 7900 XTX", 3, 0, 0);
    assert_eq!(result, "AMD Radeon RX 7900 XTX");
  }

  #[test]
  fn resolve_gpu_name_falls_back_on_miss() {
    let map = std::collections::HashMap::new();
    let result = resolve_gpu_name_from_bdf_map(&map, "Radeon RX 7900 XTX", 3, 0, 0);
    assert_eq!(result, "Radeon RX 7900 XTX");
  }

  #[test]
  fn resolve_gpu_name_distinguishes_by_bdf() {
    let mut map = std::collections::HashMap::new();
    map.insert((3, 0, 0), "GPU on bus 3".to_string());
    map.insert((6, 0, 0), "GPU on bus 6".to_string());

    assert_eq!(
      resolve_gpu_name_from_bdf_map(&map, "fallback", 3, 0, 0),
      "GPU on bus 3"
    );
    assert_eq!(
      resolve_gpu_name_from_bdf_map(&map, "fallback", 6, 0, 0),
      "GPU on bus 6"
    );
    assert_eq!(
      resolve_gpu_name_from_bdf_map(&map, "fallback", 9, 0, 0),
      "fallback"
    );
  }
}
