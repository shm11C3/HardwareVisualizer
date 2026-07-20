//! Tauri-independent data models shared across the core crate.

pub mod external_component_guidance;
pub mod hardware;
mod metrics;

pub use external_component_guidance::{
  ExternalComponent, ExternalComponentGuidanceCandidate, ExternalComponentReasonKind,
  ExternalComponentUsage, SmartInfoCollectionOutcome,
};
pub use metrics::{
  FanSpeedStatus, GpuMetric, GpuSample, MetricsSnapshot, MotherboardFanSpeed,
  MotherboardSensorCollection, MotherboardSensorSample, MotherboardTemperature,
  ProcessSample, SensorAvailability, SensorEnablement, SensorTemperature,
  SensorVerification, TemperatureSample,
};
