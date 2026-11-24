use hardviz_core::models as core_models;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ExternalComponent {
  Pawnio,
  Smartctl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ExternalComponentUsage {
  CpuPackageTemperature,
  MotherboardSensors,
  StorageHealth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ExternalComponentReasonKind {
  Missing,
  Permission,
  Misconfigured,
  Failed,
}

impl From<core_models::ExternalComponentGuidanceCandidate>
  for ExternalComponentGuidanceCandidate
{
  fn from(src: core_models::ExternalComponentGuidanceCandidate) -> Self {
    Self {
      key: src.key,
      component: src.component.into(),
      usage: src.usage.into(),
      reason_kind: src.reason_kind.into(),
      missing_signals: src.missing_signals,
      affected_device_count: src.affected_device_count,
      diagnostic_detail: src.diagnostic_detail,
    }
  }
}

impl From<core_models::ExternalComponent> for ExternalComponent {
  fn from(value: core_models::ExternalComponent) -> Self {
    match value {
      core_models::ExternalComponent::Pawnio => Self::Pawnio,
      core_models::ExternalComponent::Smartctl => Self::Smartctl,
    }
  }
}

impl From<core_models::ExternalComponentUsage> for ExternalComponentUsage {
  fn from(value: core_models::ExternalComponentUsage) -> Self {
    match value {
      core_models::ExternalComponentUsage::CpuPackageTemperature => {
        Self::CpuPackageTemperature
      }
      core_models::ExternalComponentUsage::MotherboardSensors => Self::MotherboardSensors,
      core_models::ExternalComponentUsage::StorageHealth => Self::StorageHealth,
    }
  }
}

impl From<core_models::ExternalComponentReasonKind> for ExternalComponentReasonKind {
  fn from(value: core_models::ExternalComponentReasonKind) -> Self {
    match value {
      core_models::ExternalComponentReasonKind::Missing => Self::Missing,
      core_models::ExternalComponentReasonKind::Permission => Self::Permission,
      core_models::ExternalComponentReasonKind::Misconfigured => Self::Misconfigured,
      core_models::ExternalComponentReasonKind::Failed => Self::Failed,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn converts_core_guidance_candidate_to_wire_shape() {
    let candidate =
      core_models::ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
        "PawnIOLib.dll not found".to_string(),
      );

    let wire = ExternalComponentGuidanceCandidate::from(candidate);

    assert_eq!(wire.key, "pawnio:cpu-package-temperature:v1");
    assert_eq!(wire.component, ExternalComponent::Pawnio);
    assert_eq!(wire.usage, ExternalComponentUsage::CpuPackageTemperature);
    assert_eq!(wire.reason_kind, ExternalComponentReasonKind::Missing);
    assert_eq!(wire.missing_signals, vec!["cpu-temperature"]);
  }

  #[test]
  fn converts_motherboard_sensor_guidance_candidate_to_wire_shape() {
    let candidate =
      core_models::ExternalComponentGuidanceCandidate::pawnio_motherboard_sensors(
        "pawnio_open failed: 0x80070005".to_string(),
      );

    let wire = ExternalComponentGuidanceCandidate::from(candidate);

    assert_eq!(wire.key, "pawnio:motherboard-sensors:v1");
    assert_eq!(wire.component, ExternalComponent::Pawnio);
    assert_eq!(wire.usage, ExternalComponentUsage::MotherboardSensors);
    assert_eq!(wire.reason_kind, ExternalComponentReasonKind::Permission);
    assert_eq!(
      wire.missing_signals,
      vec!["motherboard-temperature", "motherboard-fan-speed"]
    );
  }
}
