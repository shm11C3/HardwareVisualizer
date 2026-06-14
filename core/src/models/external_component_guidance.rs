use serde::{Deserialize, Serialize};

use super::hardware::SmartDiskInfo;

pub const PAWNIO_CPU_PACKAGE_TEMPERATURE_KEY: &str = "pawnio:cpu-package-temperature:v1";
pub const SMARTCTL_STORAGE_HEALTH_KEY: &str = "smartctl:storage-health:v1";

pub const SIGNAL_CPU_TEMPERATURE: &str = "cpu-temperature";
pub const SIGNAL_SMART_OVERALL_HEALTH: &str = "smart-overall-health";
pub const SIGNAL_TEMPERATURE: &str = "temperature";
pub const SIGNAL_PERCENTAGE_USED: &str = "percentage-used";
pub const SIGNAL_AVAILABLE_SPARE: &str = "available-spare";
pub const SIGNAL_REALLOCATED_SECTORS: &str = "reallocated-sectors";
pub const SIGNAL_CURRENT_PENDING_SECTORS: &str = "current-pending-sectors";
pub const SIGNAL_OFFLINE_UNCORRECTABLE_SECTORS: &str = "offline-uncorrectable-sectors";
pub const SIGNAL_MEDIA_ERRORS: &str = "media-errors";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalComponentGuidanceCandidate {
  pub key: String,
  pub component: ExternalComponent,
  pub usage: ExternalComponentUsage,
  pub reason_kind: ExternalComponentReasonKind,
  pub missing_signals: Vec<String>,
  pub affected_device_count: Option<u32>,
  pub diagnostic_detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmartInfoCollectionOutcome {
  pub disks: Result<Vec<SmartDiskInfo>, String>,
  pub guidance_candidates: Vec<ExternalComponentGuidanceCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalComponent {
  Pawnio,
  Smartctl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalComponentUsage {
  CpuPackageTemperature,
  StorageHealth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalComponentReasonKind {
  Missing,
  Permission,
  Misconfigured,
  Failed,
}

impl ExternalComponentGuidanceCandidate {
  pub fn pawnio_cpu_package_temperature(diagnostic_detail: String) -> Self {
    Self {
      key: PAWNIO_CPU_PACKAGE_TEMPERATURE_KEY.to_string(),
      component: ExternalComponent::Pawnio,
      usage: ExternalComponentUsage::CpuPackageTemperature,
      reason_kind: classify_external_component_reason(&diagnostic_detail),
      missing_signals: vec![SIGNAL_CPU_TEMPERATURE.to_string()],
      affected_device_count: None,
      diagnostic_detail: Some(diagnostic_detail),
    }
  }

  pub fn smartctl_storage_health(
    missing_signals: Vec<String>,
    affected_device_count: Option<u32>,
    diagnostic_detail: String,
  ) -> Self {
    Self {
      key: SMARTCTL_STORAGE_HEALTH_KEY.to_string(),
      component: ExternalComponent::Smartctl,
      usage: ExternalComponentUsage::StorageHealth,
      reason_kind: classify_external_component_reason(&diagnostic_detail),
      missing_signals,
      affected_device_count,
      diagnostic_detail: Some(diagnostic_detail),
    }
  }
}

pub fn classify_external_component_reason(detail: &str) -> ExternalComponentReasonKind {
  let normalized = detail.to_ascii_lowercase();

  if normalized.contains("permission denied")
    || normalized.contains("access is denied")
    || normalized.contains("operation not permitted")
    || normalized.contains("administrator")
  {
    return ExternalComponentReasonKind::Permission;
  }

  if normalized.contains("not found")
    || normalized.contains("no such file")
    || normalized.contains("cannot find")
    || normalized.contains("failed to execute")
    || normalized.contains("not installed")
  {
    return ExternalComponentReasonKind::Missing;
  }

  if normalized.contains("invalid")
    || normalized.contains("misconfigured")
    || normalized.contains("unsupported")
  {
    return ExternalComponentReasonKind::Misconfigured;
  }

  ExternalComponentReasonKind::Failed
}

pub fn all_storage_health_signal_names() -> Vec<String> {
  [
    SIGNAL_SMART_OVERALL_HEALTH,
    SIGNAL_TEMPERATURE,
    SIGNAL_PERCENTAGE_USED,
    SIGNAL_AVAILABLE_SPARE,
    SIGNAL_REALLOCATED_SECTORS,
    SIGNAL_CURRENT_PENDING_SECTORS,
    SIGNAL_OFFLINE_UNCORRECTABLE_SECTORS,
    SIGNAL_MEDIA_ERRORS,
  ]
  .into_iter()
  .map(ToOwned::to_owned)
  .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn classify_external_component_reason_treats_not_installed_as_missing() {
    assert_eq!(
      classify_external_component_reason("smartctl is not installed"),
      ExternalComponentReasonKind::Missing
    );
  }
}
