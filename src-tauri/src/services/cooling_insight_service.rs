//! Thin service wrapping Cooling Insight's Core-owned query functions
//! (#2017). Mirrors `archive_history_service`: each function just calls
//! into `hardviz_core::persistence` and maps a `sqlx::Error` into the
//! `String` shape the command layer's `Result<T, String>` uses.

use hardviz_core::persistence::cooling_band_comparison::{self, CoolingBandComparison};
use hardviz_core::persistence::cooling_baseline_delta::{self, CoolingBaselineDelta};
use hardviz_core::persistence::cooling_covariate_comparison::{
  self, CoolingCovariateComparison,
};
use hardviz_core::persistence::cooling_fan_trend::{self, CoolingFanTrend};
use hardviz_core::persistence::cooling_load_temperature_explorer::{
  self, CoolingLoadTemperatureExplorer,
};
use hardviz_core::persistence::cooling_rollup::{CpuLoadBand, DailyCoolingSummary};
use hardviz_core::persistence::cooling_trend;

pub async fn fetch_cooling_trend(days: u32) -> Result<Vec<DailyCoolingSummary>, String> {
  cooling_trend::load_cooling_trend(days)
    .await
    .map_err(|e| format!("Failed to load cooling trend: {e}"))
}

pub async fn fetch_cooling_fan_trend(days: u32) -> Result<CoolingFanTrend, String> {
  cooling_fan_trend::load_cooling_fan_trend(days)
    .await
    .map_err(|e| format!("Failed to load cooling fan trend: {e}"))
}

pub async fn fetch_cooling_band_comparison() -> Result<CoolingBandComparison, String> {
  cooling_band_comparison::load_cooling_band_comparison()
    .await
    .map_err(|e| format!("Failed to load cooling band comparison: {e}"))
}

pub async fn fetch_cooling_baseline_delta() -> Result<CoolingBaselineDelta, String> {
  cooling_baseline_delta::load_cooling_baseline_delta()
    .await
    .map_err(|e| format!("Failed to load cooling baseline delta: {e}"))
}

pub async fn fetch_cooling_load_temperature_explorer(
  recent_days: u32,
) -> Result<CoolingLoadTemperatureExplorer, String> {
  cooling_load_temperature_explorer::load_cooling_load_temperature_explorer(recent_days)
    .await
    .map_err(|e| format!("Failed to load cooling load-temperature explorer: {e}"))
}

pub async fn fetch_cooling_covariate_comparison(
  band: CpuLoadBand,
) -> Result<CoolingCovariateComparison, String> {
  cooling_covariate_comparison::load_cooling_covariate_comparison(band)
    .await
    .map_err(|e| format!("Failed to load cooling covariate comparison: {e}"))
}
