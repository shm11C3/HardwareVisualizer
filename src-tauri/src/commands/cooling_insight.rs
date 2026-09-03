//! Cooling Insight query commands (#2017): long-range trend, load-band
//! comparison, and baseline delta. Every judgment - qualifying windows,
//! comparability thresholds, and the delta observation classification -
//! is decided in `hardviz_core::persistence`; these commands only fetch
//! and convert to the wire DTOs in `crate::models::cooling_insight`.
//!
//! Periods of 30 days and below keep using `get_data_archive_series`
//! (see `commands::hardware`) - these commands exist only for the
//! 90-day and 1-year windows the daily rollup backs.

use crate::models::cooling_insight::{
  CoolingBandComparison, CoolingBaselineDelta, CoolingCovariateComparison,
  CoolingDailyTrendPoint, CoolingFanTrend, CoolingLoadBand,
  CoolingLoadTemperatureExplorer,
};
use tauri::command;

///
/// ## Get the long-range cooling trend (90-day / 1-year)
///
#[command]
#[specta::specta]
pub async fn get_cooling_trend(days: u32) -> Result<Vec<CoolingDailyTrendPoint>, String> {
  use crate::services::cooling_insight_service;

  cooling_insight_service::fetch_cooling_trend(days)
    .await
    .map(|days| days.into_iter().map(Into::into).collect())
}

///
/// ## Get the long-range per-fan speed trend (90-day / 1-year)
///
#[command]
#[specta::specta]
pub async fn get_cooling_fan_trend(days: u32) -> Result<CoolingFanTrend, String> {
  use crate::services::cooling_insight_service;

  cooling_insight_service::fetch_cooling_fan_trend(days)
    .await
    .map(Into::into)
}

///
/// ## Get the per-load-band baseline-vs-recent cooling comparison
///
#[command]
#[specta::specta]
pub async fn get_cooling_band_comparison() -> Result<CoolingBandComparison, String> {
  use crate::services::cooling_insight_service;

  cooling_insight_service::fetch_cooling_band_comparison()
    .await
    .map(Into::into)
}

///
/// ## Get the idle cooling baseline delta and its observation state
///
#[command]
#[specta::specta]
pub async fn get_cooling_baseline_delta() -> Result<CoolingBaselineDelta, String> {
  use crate::services::cooling_insight_service;

  cooling_insight_service::fetch_cooling_baseline_delta()
    .await
    .map(Into::into)
}

///
/// ## Get the CPU load vs. CPU temperature Explorer for two windows
///
/// `recent_days` is the requested length of the trailing window; Core
/// clamps it to the range the hourly rollup can answer for.
///
#[command]
#[specta::specta]
pub async fn get_cooling_load_temperature_explorer(
  recent_days: u32,
) -> Result<CoolingLoadTemperatureExplorer, String> {
  use crate::services::cooling_insight_service;

  cooling_insight_service::fetch_cooling_load_temperature_explorer(recent_days)
    .await
    .map(Into::into)
}

/// Get the co-variate comparison of the Thermal Delta windows for `band` - which archived factors moved with the Thermal Delta and which stayed within range, with each window's ΔT-per-watt fit (#2068). `band` is the CPU-load band the observation strip compares under; the windows, the comparability gate, and every judgement are Core's.
#[command]
#[specta::specta]
pub async fn get_cooling_covariate_comparison(
  band: CoolingLoadBand,
) -> Result<CoolingCovariateComparison, String> {
  use crate::services::cooling_insight_service;

  cooling_insight_service::fetch_cooling_covariate_comparison(band.into())
    .await
    .map(Into::into)
}
