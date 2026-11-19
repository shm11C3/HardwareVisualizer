use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::monitor::Phase;

pub const STABILITY_WINDOW_SAMPLES: usize = 10;

#[derive(Debug, Deserialize)]
pub struct Config {
  pub thresholds: Thresholds,

  #[serde(default)]
  pub timing: Timing,

  #[serde(default)]
  pub platforms: HashMap<String, ThresholdOverride>,

  /// Overrides applied on top of the platform-resolved thresholds for the
  /// background phase. Unset fields inherit the desktop phase thresholds.
  #[serde(default)]
  pub background: ThresholdOverride,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Thresholds {
  pub max_avg_cpu_percent: f32,
  pub max_p95_cpu_percent: f32,
  pub max_avg_app_memory_mb: f64,
  pub max_p95_app_memory_mb: f64,
  pub max_app_memory_growth_mb: f64,
  pub max_avg_memory_mb: f64,
  pub max_p95_memory_mb: f64,
  pub max_memory_growth_mb: f64,
}

#[derive(Debug, Deserialize)]
pub struct Timing {
  pub warmup_seconds: u64,
  #[serde(default = "default_stabilization_timeout_seconds")]
  pub stabilization_timeout_seconds: u64,
  pub measurement_seconds: u64,
  pub sample_interval_ms: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct ThresholdOverride {
  pub max_avg_cpu_percent: Option<f32>,
  pub max_p95_cpu_percent: Option<f32>,
  pub max_avg_app_memory_mb: Option<f64>,
  pub max_p95_app_memory_mb: Option<f64>,
  pub max_app_memory_growth_mb: Option<f64>,
  pub max_avg_memory_mb: Option<f64>,
  pub max_p95_memory_mb: Option<f64>,
  pub max_memory_growth_mb: Option<f64>,
}

#[derive(Debug)]
pub struct ValidationError(String);

impl fmt::Display for ValidationError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for ValidationError {}

impl Default for Timing {
  fn default() -> Self {
    Self {
      warmup_seconds: 15,
      stabilization_timeout_seconds: default_stabilization_timeout_seconds(),
      measurement_seconds: 60,
      sample_interval_ms: 1000,
    }
  }
}

impl Config {
  pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    config.validate()?;
    Ok(config)
  }

  fn validate(&self) -> Result<(), ValidationError> {
    if self.timing.sample_interval_ms == 0 {
      return Err(ValidationError(
        "sample_interval_ms must be greater than 0".to_string(),
      ));
    }

    let total_ms = self.timing.measurement_seconds * 1000;
    if total_ms < self.timing.sample_interval_ms {
      return Err(ValidationError(format!(
        "measurement_seconds ({}s) must be >= sample_interval_ms ({}ms)",
        self.timing.measurement_seconds, self.timing.sample_interval_ms
      )));
    }

    if self.timing.stabilization_timeout_seconds < STABILITY_WINDOW_SAMPLES as u64 {
      return Err(ValidationError(format!(
        "stabilization_timeout_seconds ({}s) must be at least {}s",
        self.timing.stabilization_timeout_seconds, STABILITY_WINDOW_SAMPLES
      )));
    }

    Ok(())
  }

  /// Thresholds for `phase`: `[thresholds]`, then the current platform's
  /// overrides, then (background only) the `[background]` overrides.
  pub fn effective_thresholds(&self, phase: Phase) -> Thresholds {
    self.thresholds_for(phase, current_platform())
  }

  fn thresholds_for(&self, phase: Phase, platform: &str) -> Thresholds {
    let mut thresholds = self.thresholds.clone();

    if let Some(overrides) = self.platforms.get(platform) {
      thresholds.apply(overrides);
    }
    if phase == Phase::Background {
      thresholds.apply(&self.background);
    }

    thresholds
  }
}

impl Thresholds {
  fn apply(&mut self, overrides: &ThresholdOverride) {
    if let Some(v) = overrides.max_avg_cpu_percent {
      self.max_avg_cpu_percent = v;
    }
    if let Some(v) = overrides.max_p95_cpu_percent {
      self.max_p95_cpu_percent = v;
    }
    if let Some(v) = overrides.max_avg_app_memory_mb {
      self.max_avg_app_memory_mb = v;
    }
    if let Some(v) = overrides.max_p95_app_memory_mb {
      self.max_p95_app_memory_mb = v;
    }
    if let Some(v) = overrides.max_app_memory_growth_mb {
      self.max_app_memory_growth_mb = v;
    }
    if let Some(v) = overrides.max_avg_memory_mb {
      self.max_avg_memory_mb = v;
    }
    if let Some(v) = overrides.max_p95_memory_mb {
      self.max_p95_memory_mb = v;
    }
    if let Some(v) = overrides.max_memory_growth_mb {
      self.max_memory_growth_mb = v;
    }
  }
}

fn default_stabilization_timeout_seconds() -> u64 {
  45
}

fn current_platform() -> &'static str {
  if cfg!(target_os = "windows") {
    "windows"
  } else if cfg!(target_os = "linux") {
    "linux"
  } else if cfg!(target_os = "macos") {
    "macos"
  } else {
    "unknown"
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn timing_defaults_stabilization_timeout_for_older_configs() {
    let timing: Timing = toml::from_str(
      r#"
        warmup_seconds = 15
        measurement_seconds = 60
        sample_interval_ms = 1000
      "#,
    )
    .expect("legacy timing config should deserialize");

    assert_eq!(timing.stabilization_timeout_seconds, 45);
  }

  #[test]
  fn background_overrides_apply_after_platform_overrides() {
    let config: Config = toml::from_str(
      r#"
        [thresholds]
        max_avg_cpu_percent = 5.0
        max_p95_cpu_percent = 10.0
        max_avg_app_memory_mb = 100.0
        max_p95_app_memory_mb = 100.0
        max_app_memory_growth_mb = 50.0
        max_avg_memory_mb = 450.0
        max_p95_memory_mb = 500.0
        max_memory_growth_mb = 50.0

        [platforms.windows]
        max_avg_app_memory_mb = 110.0
        max_avg_memory_mb = 600.0

        [background]
        max_avg_cpu_percent = 2.0
        max_avg_memory_mb = 300.0
      "#,
    )
    .expect("config should deserialize");

    let desktop = config.thresholds_for(Phase::Desktop, "windows");
    assert_eq!(desktop.max_avg_cpu_percent, 5.0);
    assert_eq!(desktop.max_avg_app_memory_mb, 110.0);
    assert_eq!(desktop.max_avg_memory_mb, 600.0);

    let background = config.thresholds_for(Phase::Background, "windows");
    assert_eq!(background.max_avg_cpu_percent, 2.0);
    assert_eq!(background.max_avg_app_memory_mb, 110.0);
    assert_eq!(background.max_avg_memory_mb, 300.0);
    assert_eq!(background.max_p95_memory_mb, 500.0);
  }

  #[test]
  fn background_inherits_desktop_thresholds_when_unset() {
    let config: Config = toml::from_str(
      r#"
        [thresholds]
        max_avg_cpu_percent = 5.0
        max_p95_cpu_percent = 10.0
        max_avg_app_memory_mb = 100.0
        max_p95_app_memory_mb = 100.0
        max_app_memory_growth_mb = 50.0
        max_avg_memory_mb = 450.0
        max_p95_memory_mb = 500.0
        max_memory_growth_mb = 50.0
      "#,
    )
    .expect("config without [background] should deserialize");

    let desktop = config.thresholds_for(Phase::Desktop, "linux");
    let background = config.thresholds_for(Phase::Background, "linux");
    assert_eq!(background.max_avg_cpu_percent, desktop.max_avg_cpu_percent);
    assert_eq!(background.max_avg_memory_mb, desktop.max_avg_memory_mb);
  }
}
