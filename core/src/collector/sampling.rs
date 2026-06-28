//! One sampling cycle of system + GPU metrics.
//!
//! Functions in this module read sysinfo / OS-specific provider state and
//! mutate the shared [`crate::collector::history::HistoryStore`].
//! Temperature unit conversion is **not** done here — collectors always
//! publish raw °C in [`crate::models::MetricsSnapshot`]. The App-side
//! `WindowAdapter` applies the user's preferred unit before emitting the
//! Tauri `HardwareMonitorUpdate` event.

use crate::collector::HistoryStore;
use crate::models::{
  ExternalComponentGuidanceCandidate, GpuMetric, MetricsSnapshot,
  MotherboardSensorSample, ProcessSample, SensorTemperature,
};

/// One GPU sample collected per physical GPU. `None` means the metric is
/// unavailable for this GPU vendor / platform.
pub struct GpuSample {
  pub gpu_id: String,
  pub name: String,
  pub usage: Option<f32>,
  pub temperature: Option<f32>,
  pub dedicated_memory_kb: Option<f32>,
  pub cooler_level: Option<u32>,
  pub source: String,
}

pub struct SystemSample {
  pub cpu_usage: f32,
  pub memory_usage: f32,
  pub processors_usage: Vec<f32>,
  pub processes: Vec<ProcessSample>,
}

/// One round of CPU / sensor temperature readings, always in raw °C.
/// Defaults to "nothing available" on platforms without a collector.
#[derive(Default)]
pub struct TemperatureSample {
  pub cpu_temperature: Option<f32>,
  pub sensor_temperatures: Vec<SensorTemperature>,
  pub unavailable_reason: Option<String>,
  pub guidance_candidates: Vec<ExternalComponentGuidanceCandidate>,
}

#[cfg(target_os = "windows")]
pub fn sample_motherboard_sensors() -> MotherboardSensorSample {
  crate::infrastructure::providers::windows::super_io_motherboard::sample_motherboard_sensors()
    .unwrap_or_default()
}

#[cfg(not(target_os = "windows"))]
pub fn sample_motherboard_sensors() -> MotherboardSensorSample {
  MotherboardSensorSample::default()
}

/// Read the latest CPU / sensor temperatures.
///
/// Windows: prefer CPU package temperature from a supported PawnIO source,
/// then fall back to ACPI thermal zones via the WMI sampler thread. When only
/// ACPI is available, the headline CPU value picks a CPU-named zone when one
/// exists, otherwise the hottest zone — see
/// [`crate::utils::thermal::select_cpu_temperature`].
#[cfg(target_os = "windows")]
pub fn sample_temperatures() -> TemperatureSample {
  use crate::infrastructure::providers::{cpu_temperature, thermal_zone};

  thermal_zone::init_thermal_zone_sampler();
  let sensor_temperatures = thermal_zone::read_thermal_zones_cached();
  build_temperature_sample(
    cpu_temperature::sample_cpu_package_temperature(),
    sensor_temperatures,
  )
}

#[cfg(target_os = "windows")]
fn build_temperature_sample(
  pawnio_cpu_temperature: Result<
    crate::infrastructure::providers::cpu_temperature::CpuPackageTemperature,
    String,
  >,
  mut sensor_temperatures: Vec<SensorTemperature>,
) -> TemperatureSample {
  match pawnio_cpu_temperature {
    Ok(sample) => {
      sensor_temperatures.insert(
        0,
        SensorTemperature {
          name: match sample.source {
            crate::infrastructure::providers::cpu_temperature::CpuTemperatureSource::IntelDtsPackageMsr => {
              "CPU Package (PawnIO Intel DTS)"
            }
            crate::infrastructure::providers::cpu_temperature::CpuTemperatureSource::AmdZenSmnTctl => {
              "CPU Package (PawnIO AMD SMN)"
            }
          }
          .to_string(),
          temperature: sample.temperature_celsius,
        },
      );
      TemperatureSample {
        cpu_temperature: Some(sample.temperature_celsius),
        sensor_temperatures,
        unavailable_reason: None,
        guidance_candidates: Vec::new(),
      }
    }
    Err(pawnio_reason) => {
      let cpu_temperature =
        crate::utils::thermal::select_cpu_temperature(&sensor_temperatures);
      let unavailable_reason = cpu_temperature.is_none().then(|| {
        format!("PawnIO unavailable ({pawnio_reason}); ACPI thermal zones unavailable")
      });
      let guidance_candidates = unavailable_reason
        .as_ref()
        .map(|reason| {
          vec![
            ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
              reason.clone(),
            ),
          ]
        })
        .unwrap_or_default();
      TemperatureSample {
        cpu_temperature,
        sensor_temperatures,
        unavailable_reason,
        guidance_candidates,
      }
    }
  }
}

/// CPU / sensor temperatures are currently collected on Windows only.
/// Linux (hwmon coretemp/k10temp) and macOS (SMC) can plug in here later.
#[cfg(not(target_os = "windows"))]
pub fn sample_temperatures() -> TemperatureSample {
  TemperatureSample::default()
}

/// Run one CPU / memory / process refresh and append samples to the
/// store's history rings.
pub fn sample_system(store: &HistoryStore) -> Option<SystemSample> {
  let result = store.system().lock().ok().map(|mut sys| {
    sys.refresh_all();

    let cpu_usage = calculate_average_cpu_usage(sys.cpus());
    let memory_usage =
      calculate_memory_usage_percentage(sys.used_memory(), sys.total_memory());
    let processors_usage: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();

    let processes: Vec<ProcessSample> = sys
      .processes()
      .iter()
      .map(|(pid, process)| ProcessSample {
        pid: pid.as_u32(),
        name: process.name().to_string_lossy().into_owned(),
        cpu_usage: process.cpu_usage(),
        memory_kb: process.memory() as f32 / 1024.0,
        run_time_secs: process.run_time(),
      })
      .collect();

    let process_history_input: Vec<_> = processes
      .iter()
      .map(|p| (sysinfo::Pid::from_u32(p.pid), p.cpu_usage, p.memory_kb))
      .collect();

    (
      cpu_usage,
      memory_usage,
      processors_usage,
      processes,
      process_history_input,
    )
  });

  let (cpu_usage, memory_usage, processors_usage, processes, process_history_input) =
    result?;

  store.push_cpu_history(cpu_usage);
  store.push_memory_history(memory_usage);
  store.update_process_histories(&process_history_input);

  Some(SystemSample {
    cpu_usage,
    memory_usage,
    processors_usage,
    processes,
  })
}

fn calculate_average_cpu_usage(cpus: &[sysinfo::Cpu]) -> f32 {
  match cpus.len() {
    0 => 0.0,
    len => (cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / len as f32).round(),
  }
}

fn calculate_memory_usage_percentage(used: u64, total: u64) -> f32 {
  match total {
    0 => 0.0,
    total => ((used as f64 / total as f64) * 100.0).round() as f32,
  }
}

#[cfg(target_os = "windows")]
pub async fn sample_gpu(store: &HistoryStore) -> Vec<GpuSample> {
  use crate::infrastructure::providers::nvapi_provider;
  use nvapi::PhysicalGpu;

  let mut gpu_metrics: Vec<GpuSample> = Vec::new();

  // ── NVIDIA GPUs via NVAPI ──
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

  // ── AMD GPUs via ADL ──
  if crate::infrastructure::providers::adl_provider::is_available() {
    sample_amd_gpu(&mut gpu_metrics).await;
  }

  // ── Intel GPUs via PDH + DXGI ──
  sample_intel_gpu(&mut gpu_metrics).await;

  if !gpu_metrics.is_empty() {
    store.update_gpu_histories(&gpu_metrics);
  }

  gpu_metrics
}

/// Resolve a GPU's canonical name using a pre-built BDF→name lookup table.
/// Returns the mapped name on hit, or the original `adl_name` on miss.
#[cfg(any(target_os = "windows", test))]
fn resolve_gpu_name_from_map(
  bdf_map: &std::collections::HashMap<(i32, i32, i32), String>,
  adl_name: &str,
  bus: i32,
  device: i32,
  function: i32,
) -> String {
  bdf_map
    .get(&(bus, device, function))
    .cloned()
    .unwrap_or_else(|| adl_name.to_string())
}

/// Build a lookup table that maps PCI BDF → DXGI device description
/// (= the canonical GPU name used by `get_gpu_info` / `GraphicInfo`).
///
/// The table is computed once via SetupDi and cached for the lifetime of
/// the process. SetupDi calls are blocking Win32 APIs, so the first
/// invocation offloads them to the Tokio blocking thread pool.
#[cfg(target_os = "windows")]
async fn bdf_to_dxgi_name() -> &'static std::collections::HashMap<(i32, i32, i32), String>
{
  use crate::infrastructure::providers::setupdi_provider;
  use crate::log_error;

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
            "collector::sampling::bdf_to_dxgi_name",
            None::<&str>
          );
          std::collections::HashMap::new()
        }
      }
    })
    .await
}

/// Collect AMD GPU usage and temperature via ADL.
/// VRAM usage is not available via ADL.
#[cfg(target_os = "windows")]
async fn sample_amd_gpu(gpu_metrics: &mut Vec<GpuSample>) {
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
    let name = resolve_gpu_name_from_map(
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
#[cfg(target_os = "windows")]
async fn sample_intel_gpu(gpu_metrics: &mut Vec<GpuSample>) {
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

#[cfg(target_os = "linux")]
pub async fn sample_gpu(store: &HistoryStore) -> Vec<GpuSample> {
  let gpu_metrics = collect_linux_gpu_metrics().await;
  if !gpu_metrics.is_empty() {
    store.update_gpu_histories(&gpu_metrics);
  }
  gpu_metrics
}

#[cfg(target_os = "macos")]
pub async fn sample_gpu(store: &HistoryStore) -> Vec<GpuSample> {
  use crate::infrastructure::providers::macos::io_kit::iokit_info;
  use crate::infrastructure::providers::macos::{gpu, gpu_info};

  static CACHED_GPU_NAME: tokio::sync::OnceCell<String> =
    tokio::sync::OnceCell::const_new();

  // Start the IOKit usage sampler thread (first call only)
  let _ = gpu::init_gpu_usage_sampler_thread();

  // Cache GPU name (fetched via IOKit + system_profiler on first call)
  let gpu_name = CACHED_GPU_NAME
    .get_or_init(|| async {
      gpu_info::get_gpu_info()
        .await
        .ok()
        .and_then(|list| list.into_iter().next())
        .map(|info| info.name)
        .unwrap_or_else(|| "Apple GPU".to_string())
    })
    .await;

  let usage = gpu::read_gpu_usage_cached().map(|v| v * 100.0);
  let temperature: Option<f32> = None;

  let memory_kb: Option<f32> =
    tokio::task::spawn_blocking(iokit_info::get_gpu_memory_usage_from_iokit)
      .await
      .ok()
      .flatten()
      .and_then(|mem| mem.in_use_bytes)
      .map(|bytes| (bytes / 1024) as f32);

  let gpu_metrics = vec![GpuSample {
    gpu_id: format!("iokit:{}", gpu_name),
    name: gpu_name.clone(),
    usage,
    temperature,
    dedicated_memory_kb: memory_kb,
    cooler_level: None,
    source: "IOKit".to_string(),
  }];
  store.update_gpu_histories(&gpu_metrics);

  gpu_metrics
}

/// Collect GPU metrics on Linux using the existing platform layer
/// (DRM/sysfs for AMD, DRM for Intel).
#[cfg(target_os = "linux")]
async fn collect_linux_gpu_metrics() -> Vec<GpuSample> {
  use crate::infrastructure::providers::drm_sys;
  use crate::infrastructure::providers::hwmon;
  use crate::infrastructure::providers::lspci;

  let mut metrics: Vec<GpuSample> = Vec::new();
  let card_ids = drm_sys::get_all_card_ids();

  let mut lspci_output: Option<Option<String>> = None;

  for card_id in card_ids {
    let vendor = drm_sys::detect_gpu_vendor(card_id);

    let (name, usage, temperature, source) = match vendor {
      drm_sys::GpuVendor::Amd => {
        let cached = lspci_output.get_or_insert_with(lspci::get_lspci_nn_output);
        let name = drm_sys::get_card_bdf(card_id)
          .and_then(|bdf| {
            cached
              .as_deref()
              .and_then(|out| lspci::parse_gpu_name_by_bdf(out, &bdf))
          })
          .unwrap_or_else(|| format!("AMD GPU (card{})", card_id));
        let usage = drm_sys::get_amd_gpu_usage(card_id as u32)
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        let temperature = hwmon::read_hwmon_temperatures(card_id)
          .ok()
          .and_then(|temps| temps.first().map(|t| t.value as f32));
        (name, usage, temperature, "DRM (AMD)".to_string())
      }
      drm_sys::GpuVendor::Intel => {
        let usage = drm_sys::get_intel_gpu_usage()
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        (
          format!("Intel GPU (card{})", card_id),
          usage,
          None,
          "DRM (Intel)".to_string(),
        )
      }
      _ => continue,
    };

    let gpu_id = drm_sys::get_card_bdf(card_id)
      .map(|bdf| format!("pci:{bdf}"))
      .unwrap_or_else(|| format!("drm:card{card_id}"));

    metrics.push(GpuSample {
      gpu_id,
      name,
      usage,
      temperature,
      dedicated_memory_kb: None,
      cooler_level: None,
      source,
    });
  }

  metrics
}

/// Build the per-GPU metrics carried in [`MetricsSnapshot`]. Temperatures
/// are forwarded as raw °C; the App-side adapter applies the user's
/// preferred unit.
fn build_gpu_metrics(gpu_samples: &[GpuSample]) -> Vec<GpuMetric> {
  gpu_samples
    .iter()
    .map(|s| GpuMetric {
      gpu_id: s.gpu_id.clone(),
      gpu_name: s.name.clone(),
      gpu_usage: s.usage.map(|u| u.round()),
      gpu_temperature: s.temperature.map(|t| t.round()),
      gpu_source: s.source.clone(),
      gpu_dedicated_memory_usage_kb: s.dedicated_memory_kb,
      gpu_cooler_level: s.cooler_level,
    })
    .collect()
}

/// Compose a [`MetricsSnapshot`] from one cycle's system + GPU +
/// temperature samples. Temperatures are forwarded as raw °C (rounded);
/// the App-side adapter applies the user's preferred unit.
pub fn build_metrics_snapshot(
  system_sample: &SystemSample,
  gpu_samples: &[GpuSample],
  temperature_sample: &TemperatureSample,
  motherboard_sample: &MotherboardSensorSample,
) -> MetricsSnapshot {
  MetricsSnapshot {
    cpu_usage: system_sample.cpu_usage,
    memory_usage: system_sample.memory_usage,
    processors_usage: system_sample.processors_usage.clone(),
    gpus: build_gpu_metrics(gpu_samples),
    processes: system_sample.processes.clone(),
    cpu_temperature: temperature_sample.cpu_temperature.map(|t| t.round()),
    sensor_temperatures: temperature_sample
      .sensor_temperatures
      .iter()
      .map(|s| SensorTemperature {
        name: s.name.clone(),
        temperature: s.temperature.round(),
      })
      .collect(),
    motherboard_temperatures: motherboard_sample
      .temperatures
      .iter()
      .map(|s| crate::models::MotherboardTemperature {
        name: s.name.clone(),
        temperature: s.temperature.round(),
        source: s.source.clone(),
      })
      .collect(),
    motherboard_fan_speeds: motherboard_sample.fan_speeds.clone(),
    external_component_guidance_candidates: temperature_sample
      .guidance_candidates
      .clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_sample(
    gpu_id: &str,
    name: &str,
    usage: Option<f32>,
    temp: Option<f32>,
  ) -> GpuSample {
    GpuSample {
      gpu_id: gpu_id.to_string(),
      name: name.to_string(),
      usage,
      temperature: temp,
      dedicated_memory_kb: Some(4096.0),
      cooler_level: Some(60),
      source: "Test".to_string(),
    }
  }

  // ── calculate_memory_usage_percentage ──

  #[test]
  fn memory_usage_percentage_total_zero() {
    assert_eq!(calculate_memory_usage_percentage(1000, 0), 0.0);
  }

  #[test]
  fn memory_usage_percentage_half_used() {
    assert_eq!(calculate_memory_usage_percentage(500, 1000), 50.0);
  }

  #[test]
  fn memory_usage_percentage_fully_used() {
    assert_eq!(calculate_memory_usage_percentage(1000, 1000), 100.0);
  }

  #[test]
  fn memory_usage_percentage_zero_used() {
    assert_eq!(calculate_memory_usage_percentage(0, 1000), 0.0);
  }

  #[test]
  fn memory_usage_percentage_rounding() {
    assert_eq!(calculate_memory_usage_percentage(333, 1000), 33.0);
  }

  #[test]
  fn memory_usage_percentage_large_values() {
    assert_eq!(
      calculate_memory_usage_percentage(16_000_000_000, 32_000_000_000),
      50.0
    );
  }

  // ── build_gpu_metrics ──

  #[test]
  fn build_gpus_from_empty_samples() {
    let result = build_gpu_metrics(&[]);
    assert!(result.is_empty());
  }

  #[test]
  fn build_gpus_from_single_sample() {
    let samples = vec![make_sample("gpu:0", "RTX 4090", Some(75.3), Some(65.0))];
    let result = build_gpu_metrics(&samples);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].gpu_id, "gpu:0");
    assert_eq!(result[0].gpu_name, "RTX 4090");
    assert_eq!(result[0].gpu_usage, Some(75.0)); // rounded
    assert_eq!(result[0].gpu_temperature, Some(65.0));
    assert_eq!(result[0].gpu_dedicated_memory_usage_kb, Some(4096.0));
    assert_eq!(result[0].gpu_cooler_level, Some(60));
  }

  #[test]
  fn build_gpus_passes_through_celsius_unchanged() {
    let samples = vec![make_sample("gpu:0", "GPU", None, Some(100.0))];
    let result = build_gpu_metrics(&samples);
    // Core publishes raw °C; the App-side adapter does any conversion.
    assert_eq!(result[0].gpu_temperature, Some(100.0));
  }

  #[test]
  fn build_gpus_temperature_none_preserved() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(50.0), None)];
    let result = build_gpu_metrics(&samples);
    assert!(result[0].gpu_temperature.is_none());
  }

  #[test]
  fn build_gpus_usage_rounded() {
    let samples = vec![make_sample("gpu:0", "GPU", Some(33.7), None)];
    let result = build_gpu_metrics(&samples);
    assert_eq!(result[0].gpu_usage, Some(34.0));
  }

  #[test]
  fn build_gpus_from_multiple_samples() {
    let samples = vec![
      make_sample("pci:0:2.0", "RTX 4090", Some(80.0), Some(70.0)),
      make_sample("pci:0:3.0", "RX 7900 XTX", Some(50.0), Some(60.0)),
    ];
    let result = build_gpu_metrics(&samples);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].gpu_id, "pci:0:2.0");
    assert_eq!(result[1].gpu_id, "pci:0:3.0");
  }

  // ── build_metrics_snapshot ──

  #[test]
  fn snapshot_carries_system_fields_and_gpus() {
    let sys = SystemSample {
      cpu_usage: 12.5,
      memory_usage: 67.0,
      processors_usage: vec![10.0, 20.0, 30.0, 40.0],
      processes: vec![ProcessSample {
        pid: 1,
        name: "init".into(),
        cpu_usage: 0.0,
        memory_kb: 1024.0,
        run_time_secs: 60,
      }],
    };
    let gpus = vec![make_sample("gpu:0", "RTX 4090", Some(50.0), Some(70.0))];
    let snap = build_metrics_snapshot(
      &sys,
      &gpus,
      &TemperatureSample::default(),
      &MotherboardSensorSample::default(),
    );
    assert_eq!(snap.cpu_usage, 12.5);
    assert_eq!(snap.memory_usage, 67.0);
    assert_eq!(snap.processors_usage, vec![10.0, 20.0, 30.0, 40.0]);
    assert_eq!(snap.gpus.len(), 1);
    assert_eq!(snap.gpus[0].gpu_id, "gpu:0");
    assert_eq!(snap.processes.len(), 1);
    assert_eq!(snap.processes[0].pid, 1);
    assert_eq!(snap.processes[0].name, "init");
    assert_eq!(snap.cpu_temperature, None);
    assert!(snap.sensor_temperatures.is_empty());
  }

  #[test]
  fn snapshot_rounds_cpu_and_sensor_temperatures() {
    let sys = SystemSample {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      processes: vec![],
    };
    let temps = TemperatureSample {
      cpu_temperature: Some(49.95),
      sensor_temperatures: vec![
        SensorTemperature {
          name: "CPUZ".into(),
          temperature: 49.95,
        },
        SensorTemperature {
          name: "TZ01".into(),
          temperature: 40.2,
        },
      ],
      unavailable_reason: None,
      guidance_candidates: Vec::new(),
    };
    let snap =
      build_metrics_snapshot(&sys, &[], &temps, &MotherboardSensorSample::default());
    assert_eq!(snap.cpu_temperature, Some(50.0));
    assert_eq!(
      snap.sensor_temperatures,
      vec![
        SensorTemperature {
          name: "CPUZ".into(),
          temperature: 50.0,
        },
        SensorTemperature {
          name: "TZ01".into(),
          temperature: 40.0,
        },
      ]
    );
  }

  #[test]
  fn snapshot_carries_external_component_guidance_candidates() {
    let sys = SystemSample {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      processes: vec![],
    };
    let candidate = ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
      "PawnIOLib.dll not found".to_string(),
    );
    let temps = TemperatureSample {
      cpu_temperature: None,
      sensor_temperatures: vec![],
      unavailable_reason: Some(
        "PawnIO unavailable; ACPI thermal zones unavailable".to_string(),
      ),
      guidance_candidates: vec![candidate.clone()],
    };

    let snap =
      build_metrics_snapshot(&sys, &[], &temps, &MotherboardSensorSample::default());

    assert_eq!(snap.external_component_guidance_candidates, vec![candidate]);
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn windows_temperature_sample_prefers_pawnio_cpu_package() {
    let sample = build_temperature_sample(
      Ok(
        crate::infrastructure::providers::cpu_temperature::CpuPackageTemperature {
          temperature_celsius: 61.25,
          source: crate::infrastructure::providers::cpu_temperature::CpuTemperatureSource::IntelDtsPackageMsr,
        },
      ),
      vec![SensorTemperature {
        name: "TZ00".into(),
        temperature: 45.0,
      }],
    );

    assert_eq!(sample.cpu_temperature, Some(61.25));
    assert_eq!(
      sample.sensor_temperatures[0].name,
      "CPU Package (PawnIO Intel DTS)"
    );
    assert_eq!(sample.sensor_temperatures[0].temperature, 61.25);
    assert!(sample.unavailable_reason.is_none());
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn windows_temperature_sample_falls_back_to_acpi_cpu_zone() {
    let sample = build_temperature_sample(
      Err("PawnIOLib.dll not found".to_string()),
      vec![
        SensorTemperature {
          name: "TZ00".into(),
          temperature: 70.0,
        },
        SensorTemperature {
          name: "CPUZ".into(),
          temperature: 51.0,
        },
      ],
    );

    assert_eq!(sample.cpu_temperature, Some(51.0));
    assert!(sample.unavailable_reason.is_none());
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn windows_temperature_sample_returns_unavailable_reason_when_all_sources_fail() {
    let sample =
      build_temperature_sample(Err("pawnio_open failed".to_string()), Vec::new());

    assert_eq!(sample.cpu_temperature, None);
    assert_eq!(
      sample.unavailable_reason.as_deref(),
      Some("PawnIO unavailable (pawnio_open failed); ACPI thermal zones unavailable")
    );
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn windows_temperature_sample_returns_guidance_when_pawnio_and_acpi_fail() {
    let sample =
      build_temperature_sample(Err("PawnIOLib.dll not found".to_string()), Vec::new());

    assert_eq!(sample.cpu_temperature, None);
    assert_eq!(sample.guidance_candidates.len(), 1);
    assert_eq!(
      sample.guidance_candidates[0].key,
      "pawnio:cpu-package-temperature:v1"
    );
    assert_eq!(
      sample.guidance_candidates[0].missing_signals,
      vec!["cpu-temperature".to_string()]
    );
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn windows_temperature_sample_does_not_return_guidance_when_acpi_fallback_succeeds() {
    let sample = build_temperature_sample(
      Err("PawnIOLib.dll not found".to_string()),
      vec![SensorTemperature {
        name: "CPUZ".into(),
        temperature: 51.0,
      }],
    );

    assert_eq!(sample.cpu_temperature, Some(51.0));
    assert!(sample.guidance_candidates.is_empty());
  }

  // ── resolve_gpu_name_from_map ──

  #[test]
  fn resolve_gpu_name_returns_dxgi_name_on_bdf_hit() {
    let mut map = std::collections::HashMap::new();
    map.insert((3, 0, 0), "AMD Radeon RX 7900 XTX".to_string());
    let result = resolve_gpu_name_from_map(&map, "Radeon RX 7900 XTX", 3, 0, 0);
    assert_eq!(result, "AMD Radeon RX 7900 XTX");
  }

  #[test]
  fn resolve_gpu_name_falls_back_on_miss() {
    let map = std::collections::HashMap::new();
    let result = resolve_gpu_name_from_map(&map, "Radeon RX 7900 XTX", 3, 0, 0);
    assert_eq!(result, "Radeon RX 7900 XTX");
  }

  #[test]
  fn resolve_gpu_name_distinguishes_by_bdf() {
    let mut map = std::collections::HashMap::new();
    map.insert((3, 0, 0), "GPU on bus 3".to_string());
    map.insert((6, 0, 0), "GPU on bus 6".to_string());

    assert_eq!(
      resolve_gpu_name_from_map(&map, "fallback", 3, 0, 0),
      "GPU on bus 3"
    );
    assert_eq!(
      resolve_gpu_name_from_map(&map, "fallback", 6, 0, 0),
      "GPU on bus 6"
    );
    assert_eq!(
      resolve_gpu_name_from_map(&map, "fallback", 9, 0, 0),
      "fallback"
    );
  }
}
