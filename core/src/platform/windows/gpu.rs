use crate::enums::error::PlatformError;
use crate::infrastructure;
use crate::infrastructure::providers::adl_provider::AdlAdapterMetric;
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
///
/// Usage and temperature are queried independently and joined by PCI BDF, so
/// an adapter appears once with whichever readings its queries produced. An
/// adapter whose usage query fails but whose temperature query succeeds keeps
/// an ADL-keyed sample with `usage: None`; `sample_pdh_gpus` may later fill
/// that missing usage in place.
async fn sample_amd_gpus(gpu_metrics: &mut Vec<GpuSample>) {
  use crate::infrastructure::providers::adl_provider;

  let usages = adl_provider::get_amd_gpu_usage_per_adapter()
    .await
    .unwrap_or_default();
  let temps = adl_provider::get_amd_gpu_temperatures_per_adapter()
    .await
    .unwrap_or_default();

  let adapters = join_adl_metrics_by_bdf(&usages, &temps);
  if adapters.is_empty() {
    return;
  }

  let bdf_map = bdf_to_dxgi_name().await;

  for adapter in adapters {
    let name = resolve_gpu_name_from_bdf_map(
      bdf_map,
      &adapter.adapter_name,
      adapter.bus,
      adapter.device,
      adapter.function,
    );
    gpu_metrics.push(GpuSample {
      gpu_id: adl_gpu_id(adapter.bus, adapter.device, adapter.function),
      name,
      usage: adapter.usage,
      temperature: adapter.temperature,
      dedicated_memory_kb: None,
      cooler_level: None,
      source: "ADL".to_string(),
    });
  }
}

/// One AMD adapter's readings after joining ADL's per-metric answers by BDF.
struct AdlAdapterReadings {
  adapter_name: String,
  bus: i32,
  device: i32,
  function: i32,
  usage: Option<f32>,
  temperature: Option<f32>,
}

/// Join ADL's per-adapter usage and temperature answers into one entry per
/// PCI address.
///
/// The two queries answer independently — an APU can refuse the usage query
/// while answering the temperature query — so the set of adapters ADL
/// measured is the union of both result sets, not the usage list alone.
/// Keying the emitted samples on the usage list dropped a temperature ADL
/// actually produced (#1991). Usage entries keep their query order; adapters
/// only the temperature query answered follow in theirs.
fn join_adl_metrics_by_bdf(
  usages: &[AdlAdapterMetric],
  temps: &[AdlAdapterMetric],
) -> Vec<AdlAdapterReadings> {
  let temp_map: std::collections::HashMap<(i32, i32, i32), f32> = temps
    .iter()
    .map(|m| ((m.bus, m.device, m.function), m.value))
    .collect();

  let mut joined: Vec<AdlAdapterReadings> = usages
    .iter()
    .map(|m| AdlAdapterReadings {
      adapter_name: m.adapter_name.clone(),
      bus: m.bus,
      device: m.device,
      function: m.function,
      usage: Some(m.value),
      temperature: temp_map.get(&(m.bus, m.device, m.function)).copied(),
    })
    .collect();

  let usage_bdfs: std::collections::HashSet<(i32, i32, i32)> = usages
    .iter()
    .map(|m| (m.bus, m.device, m.function))
    .collect();

  joined.extend(
    temps
      .iter()
      .filter(|m| !usage_bdfs.contains(&(m.bus, m.device, m.function)))
      .map(|m| AdlAdapterReadings {
        adapter_name: m.adapter_name.clone(),
        bus: m.bus,
        device: m.device,
        function: m.function,
        usage: None,
        temperature: Some(m.value),
      }),
  );

  joined
}

/// Collect usage via PDH performance counters, in two roles: add a sample for
/// every adapter no vendor API spoke for, and fill in the missing usage of a
/// vendor sample whose own usage query failed.
///
/// The vendor APIs come first because they carry temperature, VRAM, and fan
/// readings PDH does not expose; PDH is what keeps an adapter they cannot read
/// from disappearing entirely. An adapter absent from the sample stream has no
/// name, no readings, and no entry in the GPU switcher, which is how an AMD APU
/// that ADL enumerates but cannot measure became invisible next to a working
/// discrete card.
///
/// The fill role exists because vendor coverage is per-adapter, not
/// per-metric: an ADL sample carrying only a temperature still covers its
/// adapter, so no `pdh:` sample is added for it — yet PDH may well be able to
/// read the usage ADL could not. The reading is borrowed into the vendor
/// sample in place; identity stays vendor-keyed (the `pci:` id), because a
/// second id for the same adapter is the duplication
/// `is_covered_by_vendor_api` exists to prevent.
async fn sample_pdh_gpus(gpu_metrics: &mut Vec<GpuSample>) {
  use crate::infrastructure::providers::pdh_provider::{self, GpuEngineType};

  let adapters =
    crate::infrastructure::providers::directx::get_gpu_luid_info_cached().await;
  if adapters.is_empty() {
    return;
  }

  // Decided up front because the decision borrows `gpu_metrics` while the
  // loop below writes to it. Nothing is lost by deciding early: a sample PDH
  // adds is this loop's own output, and can never be the vendor API's answer
  // for a later adapter. Fill indices stay valid because the loop only
  // appends.
  let assignments: Vec<_> = {
    let sampled: Vec<(&str, &str, Option<f32>)> = gpu_metrics
      .iter()
      .map(|sample| (sample.gpu_id.as_str(), sample.name.as_str(), sample.usage))
      .collect();
    adapters
      .iter()
      .filter_map(|gpu| {
        pdh_assignment_for_adapter(gpu.vendor_id, &gpu.name, gpu.bdf, &sampled)
          .map(|assignment| (gpu, assignment))
      })
      .collect()
  };

  for (gpu, assignment) in assignments {
    let usage = pdh_provider::query_gpu_usage_by_luid_and_engine(
      gpu.luid_high,
      gpu.luid_low,
      GpuEngineType::Graphics3D,
    )
    .await
    .ok()
    .map(|v| (v * 100.0).round());

    match assignment {
      PdhAssignment::FillUsage(index) => {
        // Only a successful PDH read may overwrite: `None` is already the
        // vendor sample's honest answer.
        if let Some(value) = usage {
          gpu_metrics[index].usage = Some(value);
        }
      }
      PdhAssignment::AddAdapter => gpu_metrics.push(GpuSample {
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
      }),
    }
  }
}

/// What the PDH pass owes one DXGI adapter, decided against the samples the
/// vendor APIs already produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PdhAssignment {
  /// No vendor API spoke for the adapter: add a new `pdh:`-keyed sample.
  AddAdapter,
  /// The vendor sample at this index speaks for the adapter but is missing
  /// its usage: fill the reading in place, keeping the vendor id.
  FillUsage(usize),
}

/// Decide the PDH pass's role for one DXGI adapter, given the vendor samples
/// as `(id, name, usage)`. `None` means the adapter needs nothing from PDH:
/// its covering vendor sample already carries a usage reading, or the
/// coverage claim cannot be pinned to a single sample.
fn pdh_assignment_for_adapter(
  vendor_id: u32,
  name: &str,
  bdf: Option<(i32, i32, i32)>,
  sampled: &[(&str, &str, Option<f32>)],
) -> Option<PdhAssignment> {
  let ids_and_names: Vec<(&str, &str)> = sampled
    .iter()
    .map(|(id, sampled_name, _)| (*id, *sampled_name))
    .collect();
  if !is_covered_by_vendor_api(vendor_id, name, bdf, &ids_and_names) {
    return Some(PdhAssignment::AddAdapter);
  }

  covering_sample_index(name, bdf, sampled)
    .filter(|&index| sampled[index].2.is_none())
    .map(PdhAssignment::FillUsage)
}

/// The vendor sample that speaks for this adapter, joined the way coverage
/// was decided: by PCI address when SetupDi supplied one, by name otherwise.
///
/// An NVAPI claim resolves to no single sample — NVAPI ids carry neither a
/// PCI address nor a LUID, so a missing usage could not be attributed to this
/// adapter rather than a sibling. NVAPI samples always carry usage, so there
/// is nothing to fill there anyway.
fn covering_sample_index(
  name: &str,
  bdf: Option<(i32, i32, i32)>,
  sampled: &[(&str, &str, Option<f32>)],
) -> Option<usize> {
  if let Some((bus, device, function)) = bdf {
    let pci_id = adl_gpu_id(bus, device, function);
    return sampled.iter().position(|(id, _, _)| *id == pci_id);
  }
  sampled
    .iter()
    .position(|(_, sampled_name, _)| *sampled_name == name)
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
/// ADL ids carry the adapter's PCI address, so an AMD adapter with one is
/// decided by that comparison alone — an APU that ADL enumerates but cannot
/// read still falls through to PDH, and a same-name sibling the vendor API did
/// read must not claim coverage for it. NVAPI ids carry neither address nor
/// LUID, so any NVAPI sample claims every NVIDIA adapter. The reported name is
/// consulted only when SetupDi cannot supply a PCI address, because it is then
/// the only join key the two sources share.
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
    let pci_id = adl_gpu_id(bus, device, function);
    return sampled.iter().any(|(id, _)| *id == pci_id);
  }

  sampled
    .iter()
    .any(|(_, sampled_name)| *sampled_name == name)
}

/// Build the live id for an adapter sampled through ADL.
///
/// ADL ids carry the adapter's PCI address (ADR 0016), which is also how a
/// PDH candidate recognises an adapter ADL already reported.
fn adl_gpu_id(bus: i32, device: i32, function: i32) -> String {
  format!("pci:{bus}:{device}:{function}")
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
  fn same_name_sibling_does_not_cover_an_adapter_with_a_different_pci_address() {
    // Two identical cards; ADL read only the one on bus 3. The one on bus 6
    // must stay uncovered, or the name match would hide it from PDH — the
    // exact disappearance this module exists to prevent.
    let sampled = [("pci:3:0:0", "AMD Radeon RX 7900 XTX")];
    assert!(!is_covered_by_vendor_api(
      AMD_VENDOR_ID,
      "AMD Radeon RX 7900 XTX",
      Some((6, 0, 0)),
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

  #[test]
  fn adl_gpu_id_is_the_pci_address() {
    assert_eq!(adl_gpu_id(3, 0, 0), "pci:3:0:0");
  }

  fn adl_metric(
    name: &str,
    bus: i32,
    device: i32,
    function: i32,
    value: f32,
  ) -> AdlAdapterMetric {
    AdlAdapterMetric {
      adapter_name: name.to_string(),
      bus,
      device,
      function,
      value,
    }
  }

  #[test]
  fn adl_join_keeps_a_temperature_only_adapter() {
    // The APU answered the temperature query and refused the usage query.
    // Its temperature used to be dropped with the whole adapter (#1991).
    let usages = [adl_metric("AMD Radeon RX 7900 XTX", 3, 0, 0, 42.0)];
    let temps = [adl_metric("AMD Radeon(TM) Graphics", 101, 0, 0, 55.0)];

    let joined = join_adl_metrics_by_bdf(&usages, &temps);

    assert_eq!(joined.len(), 2);
    let apu = &joined[1];
    assert_eq!(apu.adapter_name, "AMD Radeon(TM) Graphics");
    assert_eq!((apu.bus, apu.device, apu.function), (101, 0, 0));
    assert_eq!(apu.usage, None);
    assert_eq!(apu.temperature, Some(55.0));
  }

  #[test]
  fn adl_join_keeps_a_usage_only_adapter_unchanged() {
    let usages = [adl_metric("AMD Radeon RX 7900 XTX", 3, 0, 0, 42.0)];

    let joined = join_adl_metrics_by_bdf(&usages, &[]);

    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].adapter_name, "AMD Radeon RX 7900 XTX");
    assert_eq!(joined[0].usage, Some(42.0));
    assert_eq!(joined[0].temperature, None);
  }

  #[test]
  fn adl_join_pairs_usage_and_temperature_at_the_same_address() {
    let usages = [adl_metric("AMD Radeon RX 7900 XTX", 3, 0, 0, 42.0)];
    let temps = [adl_metric("AMD Radeon RX 7900 XTX", 3, 0, 0, 68.0)];

    let joined = join_adl_metrics_by_bdf(&usages, &temps);

    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].usage, Some(42.0));
    assert_eq!(joined[0].temperature, Some(68.0));
  }

  #[test]
  fn covered_sample_missing_usage_is_the_pdh_fill_target() {
    // The temperature-only ADL sample: still vendor covered, so no `pdh:`
    // sample may be added for it — its missing usage is filled in place.
    let sampled = [
      ("pci:3:0:0", "AMD Radeon RX 7900 XTX", Some(42.0)),
      ("pci:101:0:0", "AMD Radeon(TM) Graphics", None),
    ];
    assert_eq!(
      pdh_assignment_for_adapter(
        AMD_VENDOR_ID,
        "AMD Radeon(TM) Graphics",
        Some((101, 0, 0)),
        &sampled
      ),
      Some(PdhAssignment::FillUsage(1))
    );
  }

  #[test]
  fn covered_sample_with_usage_needs_nothing_from_pdh() {
    let sampled = [
      ("pci:3:0:0", "AMD Radeon RX 7900 XTX", Some(42.0)),
      ("pci:101:0:0", "AMD Radeon(TM) Graphics", None),
    ];
    assert_eq!(
      pdh_assignment_for_adapter(
        AMD_VENDOR_ID,
        "AMD Radeon RX 7900 XTX",
        Some((3, 0, 0)),
        &sampled
      ),
      None
    );
  }

  #[test]
  fn uncovered_adapter_still_gets_a_new_pdh_sample() {
    let sampled = [("pci:3:0:0", "AMD Radeon RX 7900 XTX", None)];
    assert_eq!(
      pdh_assignment_for_adapter(
        AMD_VENDOR_ID,
        "AMD Radeon(TM) Graphics",
        Some((101, 0, 0)),
        &sampled
      ),
      Some(PdhAssignment::AddAdapter)
    );
  }

  #[test]
  fn nvapi_coverage_never_selects_a_fill_target() {
    // NVAPI ids carry neither PCI address nor LUID, so a missing usage could
    // not be attributed to this adapter rather than a sibling.
    let sampled = [("nvapi:1", "NVIDIA GeForce RTX 5080", Some(10.0))];
    assert_eq!(
      pdh_assignment_for_adapter(
        NVIDIA_VENDOR_ID,
        "NVIDIA GeForce RTX 5080",
        Some((1, 0, 0)),
        &sampled
      ),
      None
    );
  }
}
