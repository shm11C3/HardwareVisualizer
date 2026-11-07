use std::fmt::Write as _;
use std::sync::Mutex;

use hardviz_core::models::{
  ExternalComponentReasonKind, ExternalComponentUsage, FanSpeedStatus, MetricsSnapshot,
  ProviderAttemptOutcome,
};
use tauri_plugin_dialog::DialogExt as _;

use crate::models::hardware::SysInfo;
use crate::services::external_component_guidance_service::ExternalComponentGuidanceState;
use crate::workers::WorkersState;

#[derive(Clone, Default)]
struct ReportInventory {
  cpu_model: Option<String>,
  gpu_models: Vec<String>,
  board_vendor: Option<String>,
  board_model: Option<String>,
}

impl From<&SysInfo> for ReportInventory {
  fn from(info: &SysInfo) -> Self {
    Self {
      cpu_model: info.cpu.as_ref().map(|cpu| cpu.name.clone()),
      gpu_models: info
        .gpus
        .iter()
        .flatten()
        .map(|gpu| gpu.name.clone())
        .collect(),
      board_vendor: info
        .motherboard
        .as_ref()
        .map(|board| board.manufacturer.clone()),
      board_model: info.motherboard.as_ref().map(|board| board.product.clone()),
    }
  }
}

/// Populated by the existing inventory and Storage Health reads. Opening a
/// compatibility report never initiates either read or a sensor probe.
#[derive(Default)]
pub struct ReportEvidenceCache(Mutex<ReportEvidence>);

#[derive(Clone, Default)]
struct ReportEvidence {
  inventory: ReportInventory,
  storage_records: Option<bool>,
}

impl ReportEvidenceCache {
  pub(crate) fn record_inventory(&self, inventory: &SysInfo) {
    if let Ok(mut cached) = self.0.lock() {
      cached.inventory = inventory.into();
    }
  }

  pub(crate) fn record_storage(&self, available: bool) {
    if let Ok(mut cached) = self.0.lock() {
      cached.storage_records = Some(available);
    }
  }
}

#[derive(Default)]
struct ProviderEvidence {
  thermal_zones: bool,
  smartctl_guidance: Option<ExternalComponentReasonKind>,
  switchbot: bool,
}

pub(crate) fn build(
  version: &str,
  evidence: &ReportEvidenceCache,
  workers: &WorkersState,
  guidance: &ExternalComponentGuidanceState,
) -> Result<String, String> {
  let snapshot = workers
    .window_adapter
    .lock()
    .map_err(|_| "Monitor snapshot is unavailable".to_owned())?
    .as_ref()
    .and_then(|adapter| adapter.latest_snapshot())
    .ok_or_else(|| "Monitor snapshot is not ready".to_owned())?;
  #[cfg(target_os = "windows")]
  let (thermal_zones, switchbot) = {
    use hardviz_core::infrastructure::providers::windows::thermal_zone::read_thermal_zones_cached;

    let switchbot = workers
      .switchbot_provider
      .lock()
      .ok()
      .is_some_and(|provider| {
        provider.as_ref().is_some_and(|provider| {
          !provider.discovered_sensors(chrono::Utc::now()).is_empty()
        })
      });
    (!read_thermal_zones_cached().is_empty(), switchbot)
  };
  #[cfg(not(target_os = "windows"))]
  let (thermal_zones, switchbot) = (false, false);
  let providers = ProviderEvidence {
    thermal_zones,
    smartctl_guidance: guidance.recorded_storage_reason(),
    switchbot,
  };

  let cached = evidence
    .0
    .lock()
    .map(|cached| cached.clone())
    .unwrap_or_default();
  Ok(render(version, &cached, &snapshot, providers))
}

pub(crate) async fn save(
  window: tauri::WebviewWindow,
  report: String,
) -> Result<(), String> {
  tauri::async_runtime::spawn_blocking(move || {
    let file = window
      .dialog()
      .file()
      .set_parent(&window)
      .set_file_name("hardware-compatibility-report.md")
      .add_filter("Markdown", &["md"])
      .blocking_save_file();
    if let Some(file) = file {
      let path = file
        .into_path()
        .map_err(|_| "Invalid report file".to_owned())?;
      std::fs::write(path, report)
        .map_err(|_| "Could not write report file".to_owned())?;
    }
    Ok(())
  })
  .await
  .map_err(|_| "Could not save report".to_owned())?
}

fn render(
  version: &str,
  evidence: &ReportEvidence,
  snapshot: &MetricsSnapshot,
  providers: ProviderEvidence,
) -> String {
  let inventory = &evidence.inventory;
  let mut report = String::from("# Hardware Compatibility Report\n\n");
  let os_version = sysinfo::System::os_version();
  let gpu_models = inventory.gpu_models.join(", ");
  for (label, value) in [
    ("App version", Some(version)),
    ("OS family", Some(std::env::consts::OS)),
    ("OS version", os_version.as_deref()),
    ("CPU architecture", Some(std::env::consts::ARCH)),
    ("CPU model", inventory.cpu_model.as_deref()),
    ("GPU models", Some(gpu_models.as_str())),
    ("Motherboard vendor", inventory.board_vendor.as_deref()),
    ("Motherboard model", inventory.board_model.as_deref()),
    (
      "Super I/O identification",
      super_io_identification(snapshot),
    ),
  ] {
    field(&mut report, label, value);
  }

  report.push_str("\n## Sensor providers\n\n");
  let guidance_reason = |usage| {
    snapshot
      .external_component_guidance_candidates
      .iter()
      .find(|candidate| candidate.usage == usage)
      .map(|candidate| candidate.reason_kind)
  };
  provider(
    &mut report,
    "PawnIO CPU package temperature",
    snapshot.optional_sensor_providers.cpu_package_temperature,
    guidance_reason(ExternalComponentUsage::CpuPackageTemperature),
  );
  provider(
    &mut report,
    "Super I/O",
    snapshot.optional_sensor_providers.motherboard_sensors,
    guidance_reason(ExternalComponentUsage::MotherboardSensors),
  );
  field(
    &mut report,
    "Platform thermal zones",
    providers.thermal_zones.then_some("Available"),
  );
  for (label, source) in [
    ("NVIDIA NVAPI", "NVAPI"),
    ("AMD ADL", "ADL"),
    ("Windows GPU performance counters", "PDH"),
    ("AMD DRM/hwmon", "DRM (AMD)"),
    ("intel_gpu_top", "DRM (Intel)"),
    ("macOS IOKit", "IOKit"),
  ] {
    let mut samples = snapshot
      .gpus
      .iter()
      .filter(|gpu| gpu.gpu_source == source)
      .peekable();
    let status = if samples.peek().is_none() {
      "Not recorded"
    } else if samples.any(|gpu| gpu.gpu_usage.is_some() || gpu.gpu_temperature.is_some())
    {
      "Available"
    } else {
      "Unavailable"
    };
    field(&mut report, label, Some(status));
  }
  field(
    &mut report,
    "smartctl (recorded guidance)",
    providers.smartctl_guidance.map(reason_label),
  );
  field(
    &mut report,
    "SwitchBot Meter",
    providers.switchbot.then_some("Available"),
  );

  report.push_str("\n## Readings available to the app\n\n");
  let readings = [
    ("CPU temperature", Some(snapshot.cpu_temperature.is_some())),
    ("CPU power", Some(snapshot.power_draw.cpu_watts.is_some())),
    ("Thermal zone temperatures", Some(providers.thermal_zones)),
    (
      "GPU usage",
      Some(snapshot.gpus.iter().any(|gpu| gpu.gpu_usage.is_some())),
    ),
    (
      "GPU temperature",
      Some(
        snapshot
          .gpus
          .iter()
          .any(|gpu| gpu.gpu_temperature.is_some()),
      ),
    ),
    (
      "Motherboard temperatures",
      Some(!snapshot.motherboard_temperatures.is_empty()),
    ),
    (
      "Motherboard fans",
      Some(
        snapshot
          .motherboard_fan_speeds
          .iter()
          .any(|fan| fan.status != FanSpeedStatus::Invalid && fan.rpm.is_some()),
      ),
    ),
    ("Storage Health records", evidence.storage_records),
  ];
  for (label, visible) in readings {
    field(
      &mut report,
      label,
      Some(match visible {
        Some(true) => "Yes",
        Some(false) => "No",
        None => "Not recorded",
      }),
    );
  }
  report
}

fn super_io_identification(snapshot: &MetricsSnapshot) -> Option<&'static str> {
  // Report only the identification normal monitoring already publishes.
  snapshot
    .motherboard_temperatures
    .iter()
    .map(|sample| sample.source.as_str())
    .chain(
      snapshot
        .motherboard_fan_speeds
        .iter()
        .map(|sample| sample.source.as_str()),
    )
    .find_map(|source| match source {
      "IT8728F/EX / Super I/O" => Some("IT8728F/EX"),
      "NCT6799D / Super I/O" => Some("NCT6799D"),
      "Nuvoton / Super I/O" => Some("Nuvoton (family only)"),
      _ => None,
    })
}

fn provider(
  report: &mut String,
  label: &str,
  outcome: Option<ProviderAttemptOutcome>,
  reason: Option<ExternalComponentReasonKind>,
) {
  let status = match outcome {
    Some(ProviderAttemptOutcome::Answered) => "Available",
    Some(ProviderAttemptOutcome::Pending) => "Pending",
    Some(ProviderAttemptOutcome::NotAnswered) => {
      reason.map_or("Unavailable", reason_label)
    }
    None => "Not applicable",
  };
  field(report, label, Some(status));
}

fn reason_label(reason: ExternalComponentReasonKind) -> &'static str {
  match reason {
    ExternalComponentReasonKind::Missing => "Unavailable (component missing)",
    ExternalComponentReasonKind::Permission => "Unavailable (access denied)",
    ExternalComponentReasonKind::Misconfigured => {
      "Unavailable (invalid or unsupported configuration)"
    }
    ExternalComponentReasonKind::Failed => "Unavailable (provider failed)",
  }
}

fn field(report: &mut String, label: &str, value: Option<&str>) {
  let value = value
    .unwrap_or_default()
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ");
  let value = if value.is_empty() {
    "Not recorded"
  } else {
    &value
  };
  // Keep inventory text literal in Markdown and prevent it adding rows or links.
  let escaped = value
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('\\', "\\\\")
    .replace('[', "\\[")
    .replace(']', "\\]")
    .replace('`', "\\`")
    .replace('*', "\\*")
    .replace('_', "\\_");
  let _ = writeln!(report, "- {label}: {escaped}");
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::hardware::{CpuInfo, GraphicInfo, MotherboardInfo};
  use hardviz_core::models::{
    ExternalComponentGuidanceCandidate, GpuMetric, MotherboardTemperature,
    OptionalSensorProviderOutcomes, PowerDraw,
  };

  #[test]
  fn report_only_exports_models_enumerated_diagnostics_and_reading_presence() {
    let inventory = SysInfo {
      cpu: Some(CpuInfo {
        name: "Example CPU".into(),
        vendor: "Example".into(),
        core_count: 8,
        clock: 5432,
        clock_unit: "MHz".into(),
        cpu_name: "internal CPU description".into(),
      }),
      memory: None,
      gpus: Some(vec![GraphicInfo {
        id: "private PCI identifier".into(),
        name: "Example GPU".into(),
        vendor_name: "Example".into(),
        clock: 7654,
        memory_size: "private memory size".into(),
        memory_size_dedicated: String::new(),
        core_count: None,
      }]),
      storage: vec![],
      motherboard: Some(MotherboardInfo {
        manufacturer: "Example vendor".into(),
        product: "Example board".into(),
        version: String::new(),
        serial_number: "PRIVATE-SERIAL".into(),
        bios_vendor: "private BIOS vendor".into(),
        bios_version: String::new(),
        bios_release_date: String::new(),
      }),
    };
    let mut snapshot = MetricsSnapshot {
      cpu_usage: 43.21,
      memory_usage: 12.34,
      processors_usage: vec![],
      gpus: vec![GpuMetric {
        gpu_id: "private GPU id".into(),
        gpu_name: "Example GPU".into(),
        gpu_usage: Some(56.78),
        gpu_temperature: None,
        gpu_source: "NVAPI".into(),
        gpu_dedicated_memory_usage_kb: None,
        gpu_cooler_level: None,
      }],
      power_draw: PowerDraw { cpu_watts: Some(78.91), ..Default::default() },
      cpu_power_support: Default::default(),
      processes: vec![],
      cpu_temperature: Some(54.32),
      cpu_package_thermal_status: None,
      sensor_temperatures: vec![],
      motherboard_temperatures: vec![MotherboardTemperature {
        name: "private sensor label".into(),
        temperature: 65.43,
        source: "NCT6799D / Super I/O".into(),
      }],
      motherboard_fan_speeds: vec![],
      motherboard_fan_support: Default::default(),
      external_component_guidance_candidates: vec![
        ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
          r"Access denied on PRIVATE-HOST for PRIVATE-USER at C:\private\sensor.dll (00:11:22:33:44:55)".into(),
        ),
      ],
      optional_sensor_providers: OptionalSensorProviderOutcomes {
        cpu_package_temperature: Some(ProviderAttemptOutcome::NotAnswered),
        motherboard_sensors: Some(ProviderAttemptOutcome::Answered),
      },
    };

    let report = render(
      "1.2.3",
      &ReportEvidence {
        inventory: (&inventory).into(),
        storage_records: Some(false),
      },
      &snapshot,
      ProviderEvidence {
        smartctl_guidance: Some(ExternalComponentReasonKind::Missing),
        ..Default::default()
      },
    );
    for expected in [
      "CPU model: Example CPU",
      "GPU models: Example GPU",
      "Motherboard vendor: Example vendor",
      "Motherboard model: Example board",
      "Super I/O identification: NCT6799D",
      "PawnIO CPU package temperature: Unavailable (access denied)",
      "CPU temperature: Yes",
      "CPU power: Yes",
      "Motherboard temperatures: Yes",
      "Motherboard fans: No",
      "Storage Health records: No",
      "smartctl (recorded guidance): Unavailable (component missing)",
    ] {
      assert!(report.contains(expected), "{expected}\n{report}");
    }
    for excluded in [
      "PRIVATE-SERIAL",
      "PRIVATE-HOST",
      "PRIVATE-USER",
      "sensor.dll",
      "00:11:22:33:44:55",
      "private ",
      "5432",
      "7654",
      "43.21",
      "12.34",
      "56.78",
      "78.91",
      "54.32",
      "65.43",
    ] {
      assert!(!report.contains(excluded), "{excluded}\n{report}");
    }
    snapshot.motherboard_temperatures[0].source = "Nuvoton / Super I/O".into();
    assert_eq!(
      super_io_identification(&snapshot),
      Some("Nuvoton (family only)")
    );
    snapshot.motherboard_temperatures[0].source = "private provider description".into();
    assert_eq!(super_io_identification(&snapshot), None);
  }
}
