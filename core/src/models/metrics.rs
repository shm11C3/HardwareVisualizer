use super::ExternalComponentGuidanceCandidate;

/// One GPU sample collected per physical GPU.
///
/// `None` means the metric is unavailable for this GPU vendor / platform.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuSample {
  pub gpu_id: String,
  pub name: String,
  pub usage: Option<f32>,
  pub temperature: Option<f32>,
  pub dedicated_memory_kb: Option<f32>,
  pub cooler_level: Option<u32>,
  pub source: String,
}

/// One sample of per-GPU metrics, normalized across vendors and platforms.
///
/// `None` indicates the metric is unavailable for this vendor/platform.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuMetric {
  pub gpu_id: String,
  pub gpu_name: String,
  pub gpu_usage: Option<f32>,
  pub gpu_temperature: Option<f32>,
  pub gpu_source: String,
  pub gpu_dedicated_memory_usage_kb: Option<f32>,
  pub gpu_cooler_level: Option<u32>,
}

/// Availability state for best-effort live sensor sampling.
///
/// Unsupported means the platform or hardware path is not available by design.
/// Unavailable means the platform supports the path, but the current sample
/// could not produce user-visible values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SensorAvailability {
  #[default]
  Available,
  Unsupported {
    reason: String,
  },
  Unavailable {
    reason: String,
  },
}

impl SensorAvailability {
  pub fn unsupported(reason: impl Into<String>) -> Self {
    Self::Unsupported {
      reason: reason.into(),
    }
  }

  pub fn unavailable(reason: impl Into<String>) -> Self {
    Self::Unavailable {
      reason: reason.into(),
    }
  }
}

/// Verification confidence for a successfully decoded sensor reading.
///
/// This is independent from [`SensorAvailability`]. An experimental reading
/// that passed the provider's runtime and plausibility checks is still
/// available; this value tells consumers how thoroughly the hardware path has
/// been verified.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SensorVerification {
  #[default]
  Verified,
  Experimental,
}

/// Runtime enablement decision for a native sensor path.
///
/// Prefer `Experimental` when an existing read-only access path recognizes the
/// hardware and an existing plausibility-gated decode can be attempted safely.
/// Use `Unsupported` when attempting the path would require guessing hardware
/// facts such as an address, register map, or chip selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorEnablement {
  Verified,
  Experimental,
  Unsupported,
}

impl SensorEnablement {
  pub const fn verification(self) -> Option<SensorVerification> {
    match self {
      Self::Verified => Some(SensorVerification::Verified),
      Self::Experimental => Some(SensorVerification::Experimental),
      Self::Unsupported => None,
    }
  }
}

/// One named temperature sensor reading (e.g. an ACPI thermal zone),
/// always in raw °C.
///
/// `name` is a short zone label such as `TZ00` or `CPUZ`. Presentation
/// conversion (Celsius/Fahrenheit) is the App's responsibility — Core
/// never reads the user's preferred unit.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorTemperature {
  pub name: String,
  pub temperature: f32,
  pub verification: SensorVerification,
}

/// One named motherboard temperature reading, always in raw degrees C.
#[derive(Debug, Clone, PartialEq)]
pub struct MotherboardTemperature {
  pub name: String,
  pub temperature: f32,
  pub source: String,
  pub verification: SensorVerification,
}

/// Display classification for one motherboard fan-speed reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanSpeedStatus {
  Active,
  Inactive,
  Invalid,
}

/// One named motherboard fan-speed reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotherboardFanSpeed {
  pub name: String,
  pub rpm: Option<u32>,
  pub status: FanSpeedStatus,
  pub source: String,
  pub verification: SensorVerification,
}

/// One round of motherboard sensor readings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotherboardSensorSample {
  pub temperatures: Vec<MotherboardTemperature>,
  pub fan_speeds: Vec<MotherboardFanSpeed>,
}

/// One round of CPU / sensor temperature readings, always in raw °C.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TemperatureSample {
  pub cpu_temperature: Option<f32>,
  pub cpu_temperature_verification: Option<SensorVerification>,
  pub sensor_temperatures: Vec<SensorTemperature>,
  pub availability: SensorAvailability,
  pub guidance_candidates: Vec<ExternalComponentGuidanceCandidate>,
}

impl TemperatureSample {
  pub fn unsupported(reason: impl Into<String>) -> Self {
    Self {
      availability: SensorAvailability::unsupported(reason),
      ..Self::default()
    }
  }

  pub fn unavailable(reason: impl Into<String>) -> Self {
    Self {
      availability: SensorAvailability::unavailable(reason),
      ..Self::default()
    }
  }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotherboardSensorCollection {
  pub sample: MotherboardSensorSample,
  pub availability: SensorAvailability,
  pub guidance_candidates: Vec<ExternalComponentGuidanceCandidate>,
}

impl MotherboardSensorCollection {
  pub fn unsupported(reason: impl Into<String>) -> Self {
    Self {
      availability: SensorAvailability::unsupported(reason),
      ..Self::default()
    }
  }

  pub fn unavailable(reason: impl Into<String>) -> Self {
    Self {
      availability: SensorAvailability::unavailable(reason),
      ..Self::default()
    }
  }
}

/// One sample of per-process metrics carried inside [`MetricsSnapshot`].
///
/// Persistence subscribers consume these to build per-process rolling
/// stats without sharing the collector's `Arc<Mutex<...>>` history maps.
/// The window adapter ignores them.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessSample {
  pub pid: u32,
  pub name: String,
  /// Raw per-tick CPU usage as reported by sysinfo (sum across all cores).
  pub cpu_usage: f32,
  /// Memory usage in KB, matching the existing on-disk archive format.
  pub memory_kb: f32,
  /// Process run time in seconds.
  pub run_time_secs: u64,
}

/// One real-time snapshot of system + GPU metrics, fanned out via
/// [`crate::event_bus::EventBus`] to all in-process subscribers
/// (window adapter, persistence, future tray/overlay/alert consumers).
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsSnapshot {
  pub cpu_usage: f32,
  pub memory_usage: f32,
  pub processors_usage: Vec<f32>,
  pub gpus: Vec<GpuMetric>,
  pub processes: Vec<ProcessSample>,
  /// Headline CPU temperature in raw °C. `None` when no readable sensor
  /// exists on this platform (currently collected on Windows only).
  pub cpu_temperature: Option<f32>,
  /// Verification confidence for `cpu_temperature` when it is available.
  pub cpu_temperature_verification: Option<SensorVerification>,
  /// All named temperature sensors in raw °C (ACPI thermal zones on
  /// Windows). Empty when unsupported.
  pub sensor_temperatures: Vec<SensorTemperature>,
  /// Live motherboard temperature readings in raw degrees C. Empty when
  /// unsupported or unavailable.
  pub motherboard_temperatures: Vec<MotherboardTemperature>,
  /// Live motherboard fan-speed readings. Empty when unsupported or unavailable.
  pub motherboard_fan_speeds: Vec<MotherboardFanSpeed>,
  /// Diagnostic side data for optional runtime components that were
  /// attempted but unavailable while user-visible data remains missing.
  pub external_component_guidance_candidates: Vec<ExternalComponentGuidanceCandidate>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sensor_enablement_maps_only_readable_paths_to_verification() {
    assert_eq!(
      SensorEnablement::Verified.verification(),
      Some(SensorVerification::Verified)
    );
    assert_eq!(
      SensorEnablement::Experimental.verification(),
      Some(SensorVerification::Experimental)
    );
    assert_eq!(SensorEnablement::Unsupported.verification(), None);
  }
}
