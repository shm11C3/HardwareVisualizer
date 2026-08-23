use crate::enums::error::PlatformError;
use crate::infrastructure;
use crate::models::GpuSample;
use crate::{log_error, log_warn};

pub async fn get_gpu_usage() -> Result<(f32, String), PlatformError> {
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
  let intel_gpu = infrastructure::providers::directx::get_gpu_luid_info_cached()
    .await
    .iter()
    .find(|gpu| gpu.name.contains("Intel"));
  if let Some(gpu) = intel_gpu
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
    Err(e) => Err(PlatformError::unavailable(format!(
      "Failed to get GPU usage from NVIDIA API, AMD ADL, and PDH: {e:?}"
    ))),
  }
}

/// Always returns raw degrees Celsius. Presentation conversion lives at
/// the App-side boundary.
pub async fn get_gpu_temperature()
-> Result<Vec<crate::models::hardware::NameValue>, PlatformError> {
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
    Err(PlatformError::unavailable(
      "Failed to get GPU temperature from any provider",
    ))
  } else {
    Ok(all_temps)
  }
}

pub async fn get_gpu_info()
-> Result<Vec<crate::models::hardware::GraphicInfo>, PlatformError> {
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

  sample_pdh_gpus(&mut gpu_metrics).await;

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

/// Collect usage via PDH performance counters for every adapter no vendor API
/// spoke for.
///
/// The vendor APIs come first because they carry temperature, VRAM, and fan
/// readings PDH does not expose; PDH is what keeps an adapter they cannot read
/// from disappearing entirely. An adapter absent from the sample stream has no
/// name, no readings, and no entry in the GPU switcher, which is how an AMD APU
/// that ADL enumerates but cannot measure became invisible next to a working
/// discrete card.
async fn sample_pdh_gpus(gpu_metrics: &mut Vec<GpuSample>) {
  use crate::infrastructure::providers::pdh_provider::{self, GpuEngineType};

  let adapters =
    crate::infrastructure::providers::directx::get_gpu_luid_info_cached().await;
  if adapters.is_empty() {
    return;
  }

  // Decided up front because the check borrows `gpu_metrics` while the loop
  // below writes to it. Nothing is lost by deciding early: a sample PDH adds
  // is this loop's own output, and can never be the vendor API's answer for a
  // later adapter.
  let uncovered: Vec<_> = {
    let sampled: Vec<(&str, &str)> = gpu_metrics
      .iter()
      .map(|sample| (sample.gpu_id.as_str(), sample.name.as_str()))
      .collect();
    adapters
      .iter()
      .filter(|gpu| {
        !is_covered_by_vendor_api(gpu.vendor_id, &gpu.name, gpu.bdf, &sampled)
      })
      .collect()
  };

  for gpu in uncovered {
    let usage = pdh_provider::query_gpu_usage_by_luid_and_engine(
      gpu.luid_high,
      gpu.luid_low,
      GpuEngineType::Graphics3D,
    )
    .await
    .ok()
    .map(|v| (v * 100.0).round());

    gpu_metrics.push(GpuSample {
      gpu_id: pdh_gpu_id(
        gpu.device_instance_id.as_deref(),
        gpu.luid_high,
        gpu.luid_low,
      ),
      name: gpu.name.clone(),
      usage,
      temperature: None,
      dedicated_memory_kb: None,
      cooler_level: None,
      source: "PDH".to_string(),
    });
  }
}

/// PCI vendor id NVIDIA adapters report through DXGI.
const NVIDIA_VENDOR_ID: u32 = 0x10DE;

/// Whether a vendor API already produced a sample for this DXGI adapter.
///
/// Sampling the same card twice is worse than sampling it through the weaker
/// source: the two ids live in the same namespace, so the switcher would offer
/// one physical GPU as two adapters, and the duplicated name would make the
/// inventory join ambiguous enough to drop the VRAM total.
///
/// ADL ids carry the adapter's PCI address, so an AMD adapter is matched
/// exactly — an APU that ADL enumerates but cannot read still falls through to
/// PDH. NVAPI ids carry neither address nor LUID, so any NVAPI sample claims
/// every NVIDIA adapter. The reported name is the last resort, and the only
/// join key the two sources share when SetupDi cannot supply a PCI address.
fn is_covered_by_vendor_api(
  vendor_id: u32,
  name: &str,
  bdf: Option<(i32, i32, i32)>,
  sampled: &[(&str, &str)],
) -> bool {
  if vendor_id == NVIDIA_VENDOR_ID
    && sampled.iter().any(|(id, _)| id.starts_with("nvapi:"))
  {
    return true;
  }

  if let Some((bus, device, function)) = bdf {
    let pci_id = format!("pci:{bus}:{device}:{function}");
    if sampled.iter().any(|(id, _)| *id == pci_id) {
      return true;
    }
  }

  sampled
    .iter()
    .any(|(_, sampled_name)| *sampled_name == name)
}

/// Build the live id for an adapter sampled through PDH.
///
/// The DXGI LUID identifies the physical adapter independently of its display
/// name, which is not unique when Windows exposes two same-model adapters.
fn pdh_gpu_id(device_instance_id: Option<&str>, luid_high: i32, luid_low: u32) -> String {
  device_instance_id
    .map(|id| format!("pdh:instance:{id}"))
    .unwrap_or_else(|| format!("pdh:{luid_high}:{luid_low}"))
}

#[cfg(test)]
mod tests {
  use super::*;

  const AMD_VENDOR_ID: u32 = 0x1002;

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

  #[test]
  fn nvidia_adapter_is_covered_once_nvapi_reported_anything() {
    let sampled = [("nvapi:4294967295", "NVIDIA GeForce RTX 5080")];
    assert!(is_covered_by_vendor_api(
      NVIDIA_VENDOR_ID,
      "NVIDIA GeForce RTX 5080",
      Some((1, 0, 0)),
      &sampled
    ));
  }

  #[test]
  fn nvidia_adapter_falls_through_to_pdh_when_nvapi_reported_nothing() {
    assert!(!is_covered_by_vendor_api(
      NVIDIA_VENDOR_ID,
      "NVIDIA GeForce RTX 5080",
      Some((1, 0, 0)),
      &[]
    ));
  }

  #[test]
  fn amd_adapter_is_covered_by_the_adl_sample_at_its_pci_address() {
    let sampled = [("pci:3:0:0", "AMD Radeon RX 7900 XTX")];
    assert!(is_covered_by_vendor_api(
      AMD_VENDOR_ID,
      "AMD Radeon RX 7900 XTX",
      Some((3, 0, 0)),
      &sampled
    ));
  }

  #[test]
  fn amd_adapter_adl_could_not_read_still_falls_through_to_pdh() {
    // ADL reported the discrete card only; the APU's integrated Radeon is the
    // adapter that used to vanish from the switcher entirely.
    let sampled = [
      ("nvapi:1", "NVIDIA GeForce RTX 5080"),
      ("pci:3:0:0", "AMD Radeon RX 7900 XTX"),
    ];
    assert!(!is_covered_by_vendor_api(
      AMD_VENDOR_ID,
      "AMD Radeon(TM) Graphics",
      Some((101, 0, 0)),
      &sampled
    ));
  }

  #[test]
  fn adapter_without_a_pci_address_is_covered_by_a_matching_name() {
    let sampled = [("pci:3:0:0", "AMD Radeon RX 7900 XTX")];
    assert!(is_covered_by_vendor_api(
      AMD_VENDOR_ID,
      "AMD Radeon RX 7900 XTX",
      None,
      &sampled
    ));
  }

  #[test]
  fn pdh_gpu_id_prefers_reboot_stable_device_identity() {
    assert_eq!(
      pdh_gpu_id(Some(r"PCI\VEN_8086&DEV_1234&1"), 1, 2),
      r"pdh:instance:PCI\VEN_8086&DEV_1234&1"
    );
  }

  #[test]
  fn pdh_gpu_id_falls_back_to_luid_when_device_identity_is_unavailable() {
    assert_eq!(pdh_gpu_id(None, 1, 2), "pdh:1:2");
    assert_ne!(pdh_gpu_id(None, 1, 2), pdh_gpu_id(None, 3, 4));
  }
}
