use std::collections::HashMap;
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
  pub max_avg_memory_mb: Option<f64>,
  pub max_p95_memory_mb: Option<f64>,
  pub max_memory_growth_mb: Option<f64>,
}

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
    Ok(config)
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
