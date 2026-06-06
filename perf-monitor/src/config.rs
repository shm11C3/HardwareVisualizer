use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
  pub thresholds: Thresholds,

  #[serde(default)]
  pub timing: Timing,

  #[serde(default)]
  pub platforms: HashMap<String, PlatformOverride>,
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
  pub measurement_seconds: u64,
  pub sample_interval_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct PlatformOverride {
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
      warmup_seconds: 10,
      measurement_seconds: 30,
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

    Ok(())
  }

  pub fn effective_thresholds(&self) -> Thresholds {
    let mut thresholds = self.thresholds.clone();
    let platform = current_platform();

    if let Some(overrides) = self.platforms.get(platform) {
      if let Some(v) = overrides.max_avg_cpu_percent {
        thresholds.max_avg_cpu_percent = v;
      }
      if let Some(v) = overrides.max_p95_cpu_percent {
        thresholds.max_p95_cpu_percent = v;
      }
      if let Some(v) = overrides.max_avg_app_memory_mb {
        thresholds.max_avg_app_memory_mb = v;
      }
      if let Some(v) = overrides.max_p95_app_memory_mb {
        thresholds.max_p95_app_memory_mb = v;
      }
      if let Some(v) = overrides.max_app_memory_growth_mb {
        thresholds.max_app_memory_growth_mb = v;
      }
      if let Some(v) = overrides.max_avg_memory_mb {
        thresholds.max_avg_memory_mb = v;
      }
      if let Some(v) = overrides.max_p95_memory_mb {
        thresholds.max_p95_memory_mb = v;
      }
      if let Some(v) = overrides.max_memory_growth_mb {
        thresholds.max_memory_growth_mb = v;
      }
    }

    thresholds
  }
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
