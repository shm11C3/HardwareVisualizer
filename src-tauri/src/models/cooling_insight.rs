//! Wire-format mirrors of Cooling Insight's Core-owned query results
//! (#2017). Core keeps deriving every threshold, weighted average, and
//! lifecycle state; these types only carry the already-decided result
//! across IPC, following the manual `From` conversion pattern used by
//! `models::archive_history` rather than the ADR 0009 build-time
//! generator (these are not mirrors of `core/src/models/hardware.rs`).
//!
//! Dates cross the wire as `"%Y-%m-%d"` strings, matching how
//! `cooling_daily_summary.date` is already stored and compared in Core.

use chrono::NaiveDate;
use hardviz_core::persistence::cooling_band_comparison::{
  AmbientAdjustedBandComparison as CoreAmbientAdjustedBandComparison,
  BandComparison as CoreBandComparison,
  BandDeltaWindowSummary as CoreBandDeltaWindowSummary,
  BandWindowSummary as CoreBandWindowSummary,
  CoolingBandComparison as CoreCoolingBandComparison,
};
use hardviz_core::persistence::cooling_baseline::{
  BaselineState as CoreBaselineState, RecentIdleSummary as CoreRecentIdleSummary,
};
use hardviz_core::persistence::cooling_baseline_delta::{
  AmbientAdjustedBaselineDelta as CoreAmbientAdjustedBaselineDelta,
  CoolingBaselineDelta as CoreCoolingBaselineDelta,
  CoolingDeltaObservation as CoreCoolingDeltaObservation, DailyDelta as CoreDailyDelta,
};
use hardviz_core::persistence::cooling_covariate_comparison::{
  CoolingCovariateComparison as CoreCoolingCovariateComparison,
  CovariateComparability as CoreCovariateComparability,
  EstablishedCovariateComparison as CoreEstablishedCovariateComparison,
  FactorComparison as CoreFactorComparison, FactorJudgement as CoreFactorJudgement,
  FanCovariateComparison as CoreFanCovariateComparison,
  LeastSquaresFit as CoreLeastSquaresFit,
};
use hardviz_core::persistence::cooling_delta_baseline::DeltaBaselineState as CoreDeltaBaselineState;
use hardviz_core::persistence::cooling_fan_rollup::FanDailySummary as CoreFanDailySummary;
use hardviz_core::persistence::cooling_fan_trend::{
  CoolingFanTrend as CoreCoolingFanTrend, FanTrendSeries as CoreFanTrendSeries,
};
use hardviz_core::persistence::cooling_load_temperature_explorer::{
  BandMedian as CoreBandMedian, BandMedianDelta as CoreBandMedianDelta,
  CoolingLoadTemperatureExplorer as CoreCoolingLoadTemperatureExplorer,
  ExplorerWindow as CoreExplorerWindow, LoadTemperaturePoint as CoreLoadTemperaturePoint,
};
// The Thermal Delta rollup (`cooling_thermal_delta_rollup`) is
// deliberately absent here: the daily trend point carries no ambient field
// (#2045 exposes the thermal delta through the baseline/recent aggregates,
// not the long-range series).
use hardviz_core::persistence::cooling_rollup::{
  BandSummary as CoreBandSummary, CpuLoadBand as CoreCpuLoadBand,
  DailyCoolingSummary as CoreDailyCoolingSummary, PowerSummary as CorePowerSummary,
};
use serde::{Deserialize, Serialize};
use specta::Type;

fn format_date(date: NaiveDate) -> String {
  date.format("%Y-%m-%d").to_string()
}

/// One CPU-load band's temperature summary for a single day.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBandTemperature {
  pub avg: Option<f32>,
  pub max: Option<f32>,
  pub min: Option<f32>,
  pub sample_minutes: u32,
}

impl From<CoreBandSummary> for CoolingBandTemperature {
  fn from(value: CoreBandSummary) -> Self {
    Self {
      avg: value.avg,
      max: value.max,
      min: value.min,
      sample_minutes: value.sample_minutes,
    }
  }
}

/// One day's CPU package power draw in watts (#2021). Not a
/// [`CoolingBandTemperature`] despite the identical shape: power is
/// summarized over the whole day rather than per CPU-load band, and it is
/// a different unit. `sampleMinutes == 0` means no archived minute that
/// day carried a power reading, and `avg`/`max`/`min` are then all null -
/// the machine has no CPU power source, not 0 W.
//
// Kept to a single paragraph deliberately: tauri-specta renders a blank
// `///` line as `" * "` in `bindings.ts`, whose trailing space fails CI's
// `git diff --check`. The generated file must not be hand-edited, so the
// paragraph break has to go here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingPowerSummary {
  pub avg: Option<f32>,
  pub max: Option<f32>,
  pub min: Option<f32>,
  pub sample_minutes: u32,
}

impl From<CorePowerSummary> for CoolingPowerSummary {
  fn from(value: CorePowerSummary) -> Self {
    Self {
      avg: value.avg,
      max: value.max,
      min: value.min,
      sample_minutes: value.sample_minutes,
    }
  }
}

/// One day's `cooling_daily_summary` row, for the 90-day/1-year Cooling
/// Insight trend. A date the rollup has no row for is simply absent from
/// the response array - never a zero-filled entry.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingDailyTrendPoint {
  pub date: String,
  pub coverage_minutes: u32,
  pub idle: CoolingBandTemperature,
  pub low: CoolingBandTemperature,
  pub mid: CoolingBandTemperature,
  pub high: CoolingBandTemperature,
  /// The day's CPU package power, independent of the bands above. Absent
  /// on a machine with no CPU power source, which is what makes the
  /// timeline's power lane capability-dependent.
  pub power: CoolingPowerSummary,
}

impl From<CoreDailyCoolingSummary> for CoolingDailyTrendPoint {
  fn from(value: CoreDailyCoolingSummary) -> Self {
    Self {
      date: format_date(value.date),
      coverage_minutes: value.coverage_minutes,
      idle: value.idle.into(),
      low: value.low.into(),
      mid: value.mid.into(),
      high: value.high.into(),
      power: value.power.into(),
    }
  }
}

/// One fan's daily series for the 90-day / 1-year fan lane (#2022). One
/// series per fan rather than one row per day, because how many fans a
/// machine exposes is configuration-dependent; an empty response is how a
/// machine with no readable fan reports itself.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingFanTrendSeries {
  /// The fan's stable channel-derived identifier, as archived.
  pub source: String,
  pub days: Vec<CoolingFanDay>,
}

/// One `(date, fan)` row of the fan rollup, carried with its own date so a
/// day the fan recorded nothing is simply absent from the series rather
/// than present as 0 RPM. Not `Option`-shaped: every row that exists
/// carries a real measurement, and an Inactive Fan Reading is a real 0.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingFanDay {
  pub date: String,
  pub rpm_avg: f32,
  pub rpm_max: u32,
  pub rpm_min: u32,
  pub sample_minutes: u32,
}

impl From<CoreFanDailySummary> for CoolingFanDay {
  fn from(value: CoreFanDailySummary) -> Self {
    Self {
      date: format_date(value.date),
      rpm_avg: value.rpm_avg,
      rpm_max: value.rpm_max,
      rpm_min: value.rpm_min,
      sample_minutes: value.sample_minutes,
    }
  }
}

impl From<CoreFanTrendSeries> for CoolingFanTrendSeries {
  fn from(value: CoreFanTrendSeries) -> Self {
    Self {
      source: value.source,
      days: value.days.into_iter().map(Into::into).collect(),
    }
  }
}

/// The long-range fan trend plus the evidence the caller needs to read an
/// empty `series` correctly (#2022). An empty series means either that the
/// machine has no readable fan or that the rollup has not summarized one
/// yet - it only summarizes completed days - and only
/// `archiveHasReadings` tells those apart, because the one-minute fan
/// archive holds a reading the moment collection starts.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingFanTrend {
  pub series: Vec<CoolingFanTrendSeries>,
  pub archive_has_readings: bool,
}

impl From<CoreCoolingFanTrend> for CoolingFanTrend {
  fn from(value: CoreCoolingFanTrend) -> Self {
    Self {
      series: value.series.into_iter().map(Into::into).collect(),
      archive_has_readings: value.archive_has_readings,
    }
  }
}

/// A CPU-load band as the frontend names one, both in results and as
/// the band a query is asked for (`get_cooling_covariate_comparison`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CoolingLoadBand {
  Idle,
  Low,
  Mid,
  High,
}

impl From<CoreCpuLoadBand> for CoolingLoadBand {
  fn from(value: CoreCpuLoadBand) -> Self {
    match value {
      CoreCpuLoadBand::Idle => Self::Idle,
      CoreCpuLoadBand::Low => Self::Low,
      CoreCpuLoadBand::Mid => Self::Mid,
      CoreCpuLoadBand::High => Self::High,
    }
  }
}

impl From<CoolingLoadBand> for CoreCpuLoadBand {
  fn from(value: CoolingLoadBand) -> Self {
    match value {
      CoolingLoadBand::Idle => Self::Idle,
      CoolingLoadBand::Low => Self::Low,
      CoolingLoadBand::Mid => Self::Mid,
      CoolingLoadBand::High => Self::High,
    }
  }
}

/// One band's weighted-average temperature and sample coverage over a
/// date window (either the baseline window or the recent window).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBandWindowSummary {
  pub temperature_avg: Option<f32>,
  pub sample_minutes: u32,
}

impl From<CoreBandWindowSummary> for CoolingBandWindowSummary {
  fn from(value: CoreBandWindowSummary) -> Self {
    Self {
      temperature_avg: value.temperature_avg,
      sample_minutes: value.sample_minutes,
    }
  }
}

/// One band's weighted-average thermal delta (CPU package temperature
/// minus ambient) and its paired-sample coverage over a date window
/// (#2045). Named `deltaAvg` rather than `temperatureAvg` because it is a
/// difference, not an absolute temperature; `sampleMinutes` counts only
/// minutes where both readings existed, so it is always at most the
/// matching [`CoolingBandWindowSummary`]'s.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBandDeltaWindowSummary {
  pub delta_avg: Option<f32>,
  pub sample_minutes: u32,
}

impl From<CoreBandDeltaWindowSummary> for CoolingBandDeltaWindowSummary {
  fn from(value: CoreBandDeltaWindowSummary) -> Self {
    Self {
      delta_avg: value.delta_avg,
      sample_minutes: value.sample_minutes,
    }
  }
}

/// One band's ambient-adjusted baseline-vs-recent comparison (#2045): the
/// same two windows as the absolute comparison, but over the thermal
/// delta, so a rise the weather explains can be told apart from a rise
/// the cooling explains. `comparable` follows the same
/// both-sides-or-nothing rule as the absolute reading.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingAmbientAdjustedBandComparison {
  pub baseline: CoolingBandDeltaWindowSummary,
  pub recent: CoolingBandDeltaWindowSummary,
  pub comparable: bool,
}

impl From<CoreAmbientAdjustedBandComparison> for CoolingAmbientAdjustedBandComparison {
  fn from(value: CoreAmbientAdjustedBandComparison) -> Self {
    Self {
      baseline: value.baseline.into(),
      recent: value.recent.into(),
      comparable: value.comparable,
    }
  }
}

/// One CPU-load band's baseline-vs-recent comparison.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBandComparisonEntry {
  pub band: CoolingLoadBand,
  pub baseline: CoolingBandWindowSummary,
  pub recent: CoolingBandWindowSummary,
  pub comparable: bool,
  /// The ambient-adjusted reading of the same two windows (#2045). Null
  /// when neither window recorded a paired minute for this band, which is
  /// the normal state on a machine with no environmental sensor; a
  /// present value with `comparable: false` instead means ambient data
  /// exists but one window is still too thin to compare.
  pub ambient_adjusted: Option<CoolingAmbientAdjustedBandComparison>,
}

impl From<CoreBandComparison> for CoolingBandComparisonEntry {
  fn from(value: CoreBandComparison) -> Self {
    Self {
      band: value.band.into(),
      baseline: value.baseline.into(),
      recent: value.recent.into(),
      comparable: value.comparable,
      ambient_adjusted: value.ambient_adjusted.map(Into::into),
    }
  }
}

/// Cooling Insight's load-band comparison, gated by the same baseline
/// lifecycle as [`CoolingBaselineState`]: no comparison exists yet while
/// the baseline is still establishing.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
  tag = "status",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
pub enum CoolingBandComparison {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    baseline_window_start_date: String,
    baseline_window_end_date: String,
    recent_window_start_date: String,
    recent_window_end_date: String,
    bands: Vec<CoolingBandComparisonEntry>,
    // `ambientAdjustedBaseline` carries the ΔT baseline's own lifecycle
    // (#2045), once for all four bands because its window is a property
    // of the baseline rather than of a band. It advances independently of
    // the window dates above, so the two are generally different ranges;
    // while it is still establishing every band's `ambientAdjusted` is
    // null.
    //
    // A plain comment rather than a doc comment: tauri-specta renders an
    // enum variant as a single-line type literal, and any doc comment on
    // one of its fields leaves trailing whitespace in `bindings.ts` that
    // fails CI's `git diff --check`. The struct fields elsewhere in this
    // file are safe because they render as their own block.
    ambient_adjusted_baseline: CoolingDeltaBaselineState,
  },
}

impl From<CoreCoolingBandComparison> for CoolingBandComparison {
  fn from(value: CoreCoolingBandComparison) -> Self {
    match value {
      CoreCoolingBandComparison::Establishing {
        qualifying_days,
        required_days,
      } => Self::Establishing {
        qualifying_days,
        required_days,
      },
      CoreCoolingBandComparison::Established {
        baseline_window_start_date,
        baseline_window_end_date,
        recent_window_start_date,
        recent_window_end_date,
        bands,
        ambient_adjusted_baseline,
      } => Self::Established {
        baseline_window_start_date: format_date(baseline_window_start_date),
        baseline_window_end_date: format_date(baseline_window_end_date),
        recent_window_start_date: format_date(recent_window_start_date),
        recent_window_end_date: format_date(recent_window_end_date),
        bands: bands.into_iter().map(Into::into).collect(),
        ambient_adjusted_baseline: ambient_adjusted_baseline.into(),
      },
    }
  }
}

/// Lifecycle of the ambient-normalized (ΔT) cooling baseline (#2045),
/// mirroring [`CoolingBaselineState`]. It establishes over its own window
/// of days that carry paired hardware/ambient minutes, which on a machine
/// whose environmental sensor arrived late is a different - often much
/// later - range than the absolute baseline's.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
  tag = "status",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
pub enum CoolingDeltaBaselineState {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    delta_temperature_avg: f32,
    window_start_date: String,
    window_end_date: String,
    sample_minutes: u32,
  },
}

impl From<CoreDeltaBaselineState> for CoolingDeltaBaselineState {
  fn from(value: CoreDeltaBaselineState) -> Self {
    match value {
      CoreDeltaBaselineState::Establishing {
        qualifying_days,
        required_days,
      } => Self::Establishing {
        qualifying_days,
        required_days,
      },
      // The source the baseline was established from (#2062) stays in
      // Core for now: Cooling Insight has no source picker yet, and Core
      // already refuses to compare the baseline against any other source,
      // so the wire shape is unchanged until a view needs to name it.
      CoreDeltaBaselineState::Established {
        source: _,
        delta_temperature_avg,
        window_start_date,
        window_end_date,
        sample_minutes,
      } => Self::Established {
        delta_temperature_avg,
        window_start_date: format_date(window_start_date),
        window_end_date: format_date(window_end_date),
        sample_minutes,
      },
    }
  }
}

/// Lifecycle of the idle cooling baseline (see
/// `hardviz_core::persistence::cooling_baseline::BaselineState`).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
  tag = "status",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
pub enum CoolingBaselineState {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    idle_temperature_avg: f32,
    window_start_date: String,
    window_end_date: String,
    sample_minutes: u32,
  },
}

impl From<CoreBaselineState> for CoolingBaselineState {
  fn from(value: CoreBaselineState) -> Self {
    match value {
      CoreBaselineState::Establishing {
        qualifying_days,
        required_days,
      } => Self::Establishing {
        qualifying_days,
        required_days,
      },
      CoreBaselineState::Established {
        idle_temperature_avg,
        window_start_date,
        window_end_date,
        sample_minutes,
      } => Self::Established {
        idle_temperature_avg,
        window_start_date: format_date(window_start_date),
        window_end_date: format_date(window_end_date),
        sample_minutes,
      },
    }
  }
}

/// The trailing recent-window idle summary the baseline is compared
/// against.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingRecentIdleSummary {
  pub window_start_date: String,
  pub window_end_date: String,
  pub idle_temperature_avg: Option<f32>,
  pub sample_minutes: u32,
}

impl From<CoreRecentIdleSummary> for CoolingRecentIdleSummary {
  fn from(value: CoreRecentIdleSummary) -> Self {
    Self {
      window_start_date: format_date(value.window_start_date),
      window_end_date: format_date(value.window_end_date),
      idle_temperature_avg: value.idle_temperature_avg,
      sample_minutes: value.sample_minutes,
    }
  }
}

/// Cooling Insight's read of the current idle-temperature drift. The
/// frontend renders this enum as-is; the +5/+10 degC thresholds and the
/// 3-day sustain requirement stay behind Core's boundary (#1666).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CoolingDeltaObservation {
  Establishing,
  NotComparable,
  WithinRange,
  SustainedMildRise,
  SustainedLargeRise,
}

impl From<CoreCoolingDeltaObservation> for CoolingDeltaObservation {
  fn from(value: CoreCoolingDeltaObservation) -> Self {
    match value {
      CoreCoolingDeltaObservation::Establishing => Self::Establishing,
      CoreCoolingDeltaObservation::NotComparable => Self::NotComparable,
      CoreCoolingDeltaObservation::WithinRange => Self::WithinRange,
      CoreCoolingDeltaObservation::SustainedMildRise => Self::SustainedMildRise,
      CoreCoolingDeltaObservation::SustainedLargeRise => Self::SustainedLargeRise,
    }
  }
}

/// One trailing-7-day-window delta against the baseline, ending on
/// `date`.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingDailyDelta {
  pub date: String,
  pub delta: f32,
}

impl From<CoreDailyDelta> for CoolingDailyDelta {
  fn from(value: CoreDailyDelta) -> Self {
    Self {
      date: format_date(value.date),
      delta: value.delta,
    }
  }
}

/// The ambient-normalized reading of the same idle drift (#2045): how far
/// the machine's idle rise *above ambient* has moved, rather than how far
/// its absolute idle temperature has moved. A flat delta under a rising
/// absolute temperature says the room warmed up; a rising delta says the
/// machine did. `delta` is null unless `comparable`.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingAmbientAdjustedBaselineDelta {
  /// The ΔT baseline's own lifecycle and window, which advance
  /// independently of the absolute baseline beside it.
  pub baseline: CoolingDeltaBaselineState,
  pub recent: CoolingBandDeltaWindowSummary,
  pub delta: Option<f32>,
  pub comparable: bool,
}

impl From<CoreAmbientAdjustedBaselineDelta> for CoolingAmbientAdjustedBaselineDelta {
  fn from(value: CoreAmbientAdjustedBaselineDelta) -> Self {
    Self {
      baseline: value.baseline_state.into(),
      recent: value.recent.into(),
      delta: value.delta,
      comparable: value.comparable,
    }
  }
}

/// Cooling Insight's baseline delta card: the current drift, its
/// classification, and the daily series that classification was derived
/// from.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBaselineDelta {
  pub baseline: CoolingBaselineState,
  pub recent: CoolingRecentIdleSummary,
  pub delta: Option<f32>,
  pub observation: CoolingDeltaObservation,
  pub daily_deltas: Vec<CoolingDailyDelta>,
  pub sustained_days: u32,
  /// The ambient-normalized reading of the same drift (#2045). Always
  /// present, carrying its own lifecycle: a machine with no environmental
  /// sensor reports an establishing ΔT baseline at zero qualifying days
  /// rather than a fabricated number. Every field above is computed
  /// exactly as it was before #2045 whatever this one says.
  pub ambient_adjusted: CoolingAmbientAdjustedBaselineDelta,
}

impl From<CoreCoolingBaselineDelta> for CoolingBaselineDelta {
  fn from(value: CoreCoolingBaselineDelta) -> Self {
    Self {
      baseline: value.baseline_state.into(),
      recent: value.recent.into(),
      delta: value.delta,
      observation: value.observation.into(),
      daily_deltas: value.daily_deltas.into_iter().map(Into::into).collect(),
      sustained_days: value.sustained_days,
      ambient_adjusted: value.ambient_adjusted.into(),
    }
  }
}

/// One hour's (load, temperature) pair, as scattered by the Explorer.
///
/// `hourStart` is the local wall-clock hour string the hourly rollup
/// stores (`"%Y-%m-%d %H:00"`), for the same reason dates cross as
/// `"%Y-%m-%d"`: it is already the key Core compares and sorts on.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingLoadTemperaturePoint {
  pub hour_start: String,
  pub cpu_usage_avg: f32,
  pub cpu_temperature_avg: f32,
  pub sample_minutes: u32,
}

impl From<CoreLoadTemperaturePoint> for CoolingLoadTemperaturePoint {
  fn from(value: CoreLoadTemperaturePoint) -> Self {
    Self {
      hour_start: hardviz_core::persistence::cooling_hourly_rollup::format_hour_start(
        value.hour_start,
      ),
      cpu_usage_avg: value.cpu_usage_avg,
      cpu_temperature_avg: value.cpu_temperature_avg,
      sample_minutes: value.sample_minutes,
    }
  }
}

/// One band's temperature median within one Explorer window, with the
/// evidence behind it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBandMedian {
  pub temperature_median: Option<f32>,
  pub point_count: u32,
  pub sample_minutes: u32,
}

impl From<CoreBandMedian> for CoolingBandMedian {
  fn from(value: CoreBandMedian) -> Self {
    Self {
      temperature_median: value.temperature_median,
      point_count: value.point_count,
      sample_minutes: value.sample_minutes,
    }
  }
}

/// One of the Explorer's two windows: its calendar range and its scatter
/// points. Per-band medians live on [`CoolingBandMedianDelta`], which
/// pairs both windows' values for a band.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingExplorerWindow {
  pub start_date: String,
  pub end_date: String,
  pub points: Vec<CoolingLoadTemperaturePoint>,
}

impl From<CoreExplorerWindow> for CoolingExplorerWindow {
  fn from(value: CoreExplorerWindow) -> Self {
    Self {
      start_date: format_date(value.start_date),
      end_date: format_date(value.end_date),
      points: value.points.into_iter().map(Into::into).collect(),
    }
  }
}

/// One band's two window medians and the delta between them. `delta` is
/// absent whenever `comparable` is `false`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBandMedianDelta {
  pub band: CoolingLoadBand,
  pub baseline: CoolingBandMedian,
  pub recent: CoolingBandMedian,
  pub delta: Option<f32>,
  pub comparable: bool,
}

impl From<CoreBandMedianDelta> for CoolingBandMedianDelta {
  fn from(value: CoreBandMedianDelta) -> Self {
    Self {
      band: value.band.into(),
      baseline: value.baseline.into(),
      recent: value.recent.into(),
      delta: value.delta,
      comparable: value.comparable,
    }
  }
}

/// Cooling Insight's load-vs-temperature Explorer, gated by the same
/// baseline lifecycle as [`CoolingBandComparison`].
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
  tag = "status",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
pub enum CoolingLoadTemperatureExplorer {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    baseline: CoolingExplorerWindow,
    recent: CoolingExplorerWindow,
    band_deltas: Vec<CoolingBandMedianDelta>,
  },
}

impl From<CoreCoolingLoadTemperatureExplorer> for CoolingLoadTemperatureExplorer {
  fn from(value: CoreCoolingLoadTemperatureExplorer) -> Self {
    match value {
      CoreCoolingLoadTemperatureExplorer::Establishing {
        qualifying_days,
        required_days,
      } => Self::Establishing {
        qualifying_days,
        required_days,
      },
      CoreCoolingLoadTemperatureExplorer::Established {
        baseline,
        recent,
        band_deltas,
      } => Self::Established {
        baseline: baseline.into(),
        recent: recent.into(),
        band_deltas: band_deltas.into_iter().map(Into::into).collect(),
      },
    }
  }
}

/// Where one co-variate's recent median sits against the baseline
/// window's own daily spread (#2068), decided in Core: `withinRange` and
/// `moved` are read against the baseline window's interquartile range of
/// daily medians; `notComparable` is a factor both windows carry that
/// cannot be judged (too few baseline days for a range, no recent day,
/// or the windows as a whole not comparable); `absent` is a factor
/// neither window ever archived - never reported as zero, and never as
/// having stayed within range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CoolingFactorJudgement {
  WithinRange,
  Moved,
  NotComparable,
  Absent,
}

impl From<CoreFactorJudgement> for CoolingFactorJudgement {
  fn from(value: CoreFactorJudgement) -> Self {
    match value {
      CoreFactorJudgement::WithinRange => Self::WithinRange,
      CoreFactorJudgement::Moved => Self::Moved,
      CoreFactorJudgement::NotComparable => Self::NotComparable,
      CoreFactorJudgement::Absent => Self::Absent,
    }
  }
}

/// One archived co-variate across the two windows (#2068): each window's
/// median of its daily medians, `change` as `recent - baseline` (present
/// only when both are), and the judgement Core made. A window that never
/// archived the factor reads null there, not 0 - which is what keeps an
/// `absent` factor from looking like a machine drawing no power.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingFactorComparison {
  pub baseline: Option<f32>,
  pub recent: Option<f32>,
  pub change: Option<f32>,
  pub judgement: CoolingFactorJudgement,
}

impl From<CoreFactorComparison> for CoolingFactorComparison {
  fn from(value: CoreFactorComparison) -> Self {
    Self {
      baseline: value.baseline,
      recent: value.recent,
      change: value.change,
      judgement: value.judgement.into(),
    }
  }
}

/// The least-squares line through one window's paired minutes (#2068).
/// For the ΔT-power fit `slope` is kelvin per watt and `intercept` the
/// ΔT the line reads at zero power; for the ΔT-fan fit the slope is
/// kelvin per rpm. The whole fit is null, rather than a flat line, where
/// the window had fewer than two paired minutes or no spread in one
/// reading.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingLeastSquaresFit {
  pub slope: f32,
  pub intercept: f32,
  pub pearson_r: f32,
  pub paired_minutes: u32,
}

impl From<CoreLeastSquaresFit> for CoolingLeastSquaresFit {
  fn from(value: CoreLeastSquaresFit) -> Self {
    Self {
      slope: value.slope,
      intercept: value.intercept,
      pearson_r: value.pearson_r,
      paired_minutes: value.paired_minutes,
    }
  }
}

/// One fan's speed across the two windows, with each window's
/// ΔT-per-rpm fit (#2068). `fanSource` is the fan's stable
/// channel-derived identifier, as archived.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingFanCovariateComparison {
  pub fan_source: String,
  pub speed: CoolingFactorComparison,
  pub baseline_fit: Option<CoolingLeastSquaresFit>,
  pub recent_fit: Option<CoolingLeastSquaresFit>,
}

impl From<CoreFanCovariateComparison> for CoolingFanCovariateComparison {
  fn from(value: CoreFanCovariateComparison) -> Self {
    Self {
      fan_source: value.fan_source,
      speed: value.speed.into(),
      baseline_fit: value.baseline_fit.map(Into::into),
      recent_fit: value.recent_fit.map(Into::into),
    }
  }
}

/// Why the two windows are, or are not, compared (#2068):
/// `tooFewPairedMinutes` when one window carries fewer Thermal Delta
/// paired minutes in the compared band than Core requires (including a
/// recent window no source paired at all), `differentAmbientSource` when
/// the recent window's dominant source is not the one the Thermal Delta
/// Baseline was established from (#2062).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CoolingCovariateComparability {
  Comparable,
  TooFewPairedMinutes,
  DifferentAmbientSource,
}

impl From<CoreCovariateComparability> for CoolingCovariateComparability {
  fn from(value: CoreCovariateComparability) -> Self {
    match value {
      CoreCovariateComparability::Comparable => Self::Comparable,
      CoreCovariateComparability::TooFewPairedMinutes => Self::TooFewPairedMinutes,
      CoreCovariateComparability::DifferentAmbientSource => Self::DifferentAmbientSource,
    }
  }
}

/// Cooling Insight's co-variate comparison for one CPU-load band
/// (#2068), gated by the Thermal Delta Baseline's lifecycle exactly as
/// the ambient-adjusted band comparison is: while it establishes there is
/// no baseline window to read. Temperatures cross the wire as Core holds
/// them, exactly like the other Cooling Insight readings: the ambient
/// medians in Celsius, and the ΔT change at matched power and every
/// fit's slope and intercept in kelvin - the frontend applies the
/// preferred temperature unit, scaling a difference without the offset.
/// Package power is in watts and fan speed in rpm, which no unit
/// preference touches.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
  tag = "status",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
// Not boxed the way Core boxes its established variant: specta refuses
// to merge a boxed payload into an internal tag, and the established
// fields belong beside `status` as every other cooling lifecycle's do.
// One value per query is built, so the size gap costs nothing.
#[allow(clippy::large_enum_variant)]
pub enum CoolingCovariateComparison {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  // Plain comments rather than doc comments on these fields, for the
  // reason given on `CoolingBandComparison::Established`: a doc comment
  // on a variant field leaves trailing whitespace in `bindings.ts`.
  Established {
    // The CPU-load band both windows are read under.
    band: CoolingLoadBand,
    // The ambient source the Thermal Delta Baseline was established
    // from; the baseline side is read from its rows only.
    baseline_source: String,
    baseline_window_start_date: String,
    baseline_window_end_date: String,
    // The source that covered most of the recent window, or null when
    // no source paired a minute in it.
    recent_source: Option<String>,
    recent_window_start_date: String,
    recent_window_end_date: String,
    // Thermal Delta paired minutes in the band, per window - the
    // evidence the comparability gate was decided on.
    baseline_paired_minutes: u32,
    recent_paired_minutes: u32,
    package_power: CoolingFactorComparison,
    ambient_temperature: CoolingFactorComparison,
    // The band's share of each window's classifiable paired minutes.
    load_band_share: CoolingFactorComparison,
    // One entry per fan either window archived, ordered by fan source;
    // empty on a machine with no readable fan.
    fans: Vec<CoolingFanCovariateComparison>,
    // Each window's ΔT-per-watt line, present wherever that window alone
    // supports one - regardless of `comparable`, which gates only the
    // comparison between them.
    baseline_fit: Option<CoolingLeastSquaresFit>,
    recent_fit: Option<CoolingLeastSquaresFit>,
    // How much higher the recent line sits than the baseline line at
    // the baseline window's median package power - the ΔT change at
    // matched power, in kelvin. Null unless `comparable`, both fits
    // exist, and the baseline window archived power.
    delta_at_baseline_median_power: Option<f32>,
    comparable: bool,
    comparability: CoolingCovariateComparability,
  },
}

impl From<CoreCoolingCovariateComparison> for CoolingCovariateComparison {
  fn from(value: CoreCoolingCovariateComparison) -> Self {
    match value {
      CoreCoolingCovariateComparison::Establishing {
        qualifying_days,
        required_days,
      } => Self::Establishing {
        qualifying_days,
        required_days,
      },
      CoreCoolingCovariateComparison::Established(established) => {
        let CoreEstablishedCovariateComparison {
          band,
          baseline_source,
          baseline_window_start_date,
          baseline_window_end_date,
          recent_source,
          recent_window_start_date,
          recent_window_end_date,
          baseline_paired_minutes,
          recent_paired_minutes,
          package_power,
          ambient_temperature,
          load_band_share,
          fans,
          baseline_fit,
          recent_fit,
          delta_at_baseline_median_power,
          comparable,
          comparability,
        } = *established;
        Self::Established {
          band: band.into(),
          baseline_source,
          baseline_window_start_date: format_date(baseline_window_start_date),
          baseline_window_end_date: format_date(baseline_window_end_date),
          recent_source,
          recent_window_start_date: format_date(recent_window_start_date),
          recent_window_end_date: format_date(recent_window_end_date),
          baseline_paired_minutes,
          recent_paired_minutes,
          package_power: package_power.into(),
          ambient_temperature: ambient_temperature.into(),
          load_band_share: load_band_share.into(),
          fans: fans.into_iter().map(Into::into).collect(),
          baseline_fit: baseline_fit.map(Into::into),
          recent_fit: recent_fit.map(Into::into),
          delta_at_baseline_median_power,
          comparable,
          comparability: comparability.into(),
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use hardviz_core::persistence::cooling_rollup::CpuLoadBand;

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  #[test]
  fn a_core_daily_cooling_summary_carries_its_date_as_an_iso_string() {
    let core = CoreDailyCoolingSummary {
      date: date(2026, 8, 5),
      coverage_minutes: 1440,
      idle: CoreBandSummary::default(),
      low: CoreBandSummary::default(),
      mid: CoreBandSummary::default(),
      high: CoreBandSummary::default(),
      power: CorePowerSummary::default(),
    };

    let wire: CoolingDailyTrendPoint = core.into();

    assert_eq!(wire.date, "2026-08-05");
  }

  #[test]
  fn a_day_without_power_readings_crosses_the_wire_as_absent_not_zero() {
    let core = CoreDailyCoolingSummary {
      date: date(2026, 8, 5),
      coverage_minutes: 1440,
      idle: CoreBandSummary::default(),
      low: CoreBandSummary::default(),
      mid: CoreBandSummary::default(),
      high: CoreBandSummary::default(),
      power: CorePowerSummary::default(),
    };

    let json = serde_json::to_value(CoolingDailyTrendPoint::from(core)).unwrap();

    assert!(json["power"]["avg"].is_null());
    assert!(json["power"]["max"].is_null());
    assert!(json["power"]["min"].is_null());
    assert_eq!(json["power"]["sampleMinutes"], 0);
  }

  #[test]
  fn a_days_power_summary_crosses_the_wire_in_camel_case() {
    let core = CoreDailyCoolingSummary {
      date: date(2026, 8, 5),
      coverage_minutes: 1440,
      idle: CoreBandSummary::default(),
      low: CoreBandSummary::default(),
      mid: CoreBandSummary::default(),
      high: CoreBandSummary::default(),
      power: CorePowerSummary {
        avg: Some(18.5),
        max: Some(42.0),
        min: Some(4.5),
        sample_minutes: 1200,
      },
    };

    let json = serde_json::to_value(CoolingDailyTrendPoint::from(core)).unwrap();

    assert_eq!(json["power"]["avg"], 18.5);
    assert_eq!(json["power"]["max"], 42.0);
    assert_eq!(json["power"]["min"], 4.5);
    assert_eq!(json["power"]["sampleMinutes"], 1200);
    assert!(json["power"].get("sample_minutes").is_none());
  }

  // `rename_all` on a tagged enum only renames variant tags, not the
  // fields inside struct variants - that needs `rename_all_fields` too.
  // These assert on the actual serialized JSON (not just the Rust
  // struct), since a `PartialEq` comparison against another Rust value
  // would not have caught the enum's fields serializing as snake_case.

  #[test]
  fn an_established_baseline_state_serializes_its_fields_as_camel_case() {
    let wire = CoolingBaselineState::Established {
      idle_temperature_avg: 32.5,
      window_start_date: "2026-01-01".to_string(),
      window_end_date: "2026-01-07".to_string(),
      sample_minutes: 210,
    };

    let json = serde_json::to_value(&wire).unwrap();

    assert_eq!(json["status"], "established");
    assert_eq!(json["idleTemperatureAvg"], 32.5);
    assert_eq!(json["windowStartDate"], "2026-01-01");
    assert_eq!(json["windowEndDate"], "2026-01-07");
    assert_eq!(json["sampleMinutes"], 210);
    assert!(
      json.get("idle_temperature_avg").is_none(),
      "must not also serialize the snake_case field name"
    );
  }

  #[test]
  fn an_established_band_comparison_serializes_its_fields_as_camel_case() {
    let wire = CoolingBandComparison::Established {
      baseline_window_start_date: "2026-01-01".to_string(),
      baseline_window_end_date: "2026-01-07".to_string(),
      recent_window_start_date: "2026-08-14".to_string(),
      recent_window_end_date: "2026-08-20".to_string(),
      bands: Vec::new(),
      ambient_adjusted_baseline: CoolingDeltaBaselineState::Establishing {
        qualifying_days: 0,
        required_days: 7,
      },
    };

    let json = serde_json::to_value(&wire).unwrap();

    assert_eq!(json["status"], "established");
    assert_eq!(json["baselineWindowStartDate"], "2026-01-01");
    assert_eq!(json["recentWindowEndDate"], "2026-08-20");
    assert!(
      json.get("baseline_window_start_date").is_none(),
      "must not also serialize the snake_case field name"
    );
  }

  #[test]
  fn an_establishing_baseline_state_carries_its_progress_through() {
    let core = CoreBaselineState::Establishing {
      qualifying_days: 3,
      required_days: 7,
    };

    let wire: CoolingBaselineState = core.into();

    assert_eq!(
      wire,
      CoolingBaselineState::Establishing {
        qualifying_days: 3,
        required_days: 7,
      }
    );
  }

  #[test]
  fn an_established_baseline_state_formats_its_window_dates() {
    let core = CoreBaselineState::Established {
      idle_temperature_avg: 32.5,
      window_start_date: date(2026, 1, 1),
      window_end_date: date(2026, 1, 7),
      sample_minutes: 210,
    };

    let wire: CoolingBaselineState = core.into();

    assert_eq!(
      wire,
      CoolingBaselineState::Established {
        idle_temperature_avg: 32.5,
        window_start_date: "2026-01-01".to_string(),
        window_end_date: "2026-01-07".to_string(),
        sample_minutes: 210,
      }
    );
  }

  #[test]
  fn cpu_load_bands_map_one_to_one() {
    assert_eq!(
      CoolingLoadBand::from(CpuLoadBand::Idle),
      CoolingLoadBand::Idle
    );
    assert_eq!(
      CoolingLoadBand::from(CpuLoadBand::Low),
      CoolingLoadBand::Low
    );
    assert_eq!(
      CoolingLoadBand::from(CpuLoadBand::Mid),
      CoolingLoadBand::Mid
    );
    assert_eq!(
      CoolingLoadBand::from(CpuLoadBand::High),
      CoolingLoadBand::High
    );
  }

  #[test]
  fn an_established_band_comparison_formats_every_window_date() {
    let core = CoreCoolingBandComparison::Established {
      baseline_window_start_date: date(2026, 1, 1),
      baseline_window_end_date: date(2026, 1, 7),
      recent_window_start_date: date(2026, 8, 14),
      recent_window_end_date: date(2026, 8, 20),
      bands: Box::new([
        CoreBandComparison {
          band: CpuLoadBand::Idle,
          baseline: CoreBandWindowSummary {
            temperature_avg: Some(30.0),
            sample_minutes: 210,
          },
          recent: CoreBandWindowSummary {
            temperature_avg: Some(35.0),
            sample_minutes: 210,
          },
          comparable: true,
          ambient_adjusted: Some(CoreAmbientAdjustedBandComparison {
            baseline: CoreBandDeltaWindowSummary {
              delta_avg: Some(8.0),
              sample_minutes: 210,
            },
            recent: CoreBandDeltaWindowSummary {
              delta_avg: Some(9.5),
              sample_minutes: 210,
            },
            comparable: true,
          }),
        },
        CoreBandComparison {
          band: CpuLoadBand::Low,
          baseline: CoreBandWindowSummary::default(),
          recent: CoreBandWindowSummary::default(),
          comparable: false,
          ambient_adjusted: None,
        },
        CoreBandComparison {
          band: CpuLoadBand::Mid,
          baseline: CoreBandWindowSummary::default(),
          recent: CoreBandWindowSummary::default(),
          comparable: false,
          ambient_adjusted: None,
        },
        CoreBandComparison {
          band: CpuLoadBand::High,
          baseline: CoreBandWindowSummary::default(),
          recent: CoreBandWindowSummary::default(),
          comparable: false,
          ambient_adjusted: None,
        },
      ]),
      ambient_adjusted_baseline: CoreDeltaBaselineState::Established {
        source: "Living Room".to_string(),
        delta_temperature_avg: 11.5,
        window_start_date: date(2026, 6, 1),
        window_end_date: date(2026, 6, 7),
        sample_minutes: 420,
      },
    };

    let wire: CoolingBandComparison = core.into();

    match wire {
      CoolingBandComparison::Established {
        baseline_window_start_date,
        baseline_window_end_date,
        recent_window_start_date,
        recent_window_end_date,
        bands,
        ambient_adjusted_baseline,
      } => {
        // The ΔT baseline's window is its own, and must cross the wire
        // as such rather than echoing the absolute one above.
        assert_eq!(
          ambient_adjusted_baseline,
          CoolingDeltaBaselineState::Established {
            delta_temperature_avg: 11.5,
            window_start_date: "2026-06-01".to_string(),
            window_end_date: "2026-06-07".to_string(),
            sample_minutes: 420,
          }
        );
        assert_eq!(baseline_window_start_date, "2026-01-01");
        assert_eq!(baseline_window_end_date, "2026-01-07");
        assert_eq!(recent_window_start_date, "2026-08-14");
        assert_eq!(recent_window_end_date, "2026-08-20");
        assert_eq!(bands.len(), 4);
        assert_eq!(bands[0].band, CoolingLoadBand::Idle);
        assert!(bands[0].comparable);
      }
      other => panic!("expected an established comparison, got {other:?}"),
    }
  }

  #[test]
  fn a_baseline_delta_carries_the_daily_series_with_formatted_dates() {
    let core = CoreCoolingBaselineDelta {
      baseline_state: CoreBaselineState::Established {
        idle_temperature_avg: 30.0,
        window_start_date: date(2026, 1, 1),
        window_end_date: date(2026, 1, 7),
        sample_minutes: 210,
      },
      recent: CoreRecentIdleSummary {
        window_start_date: date(2026, 8, 14),
        window_end_date: date(2026, 8, 20),
        idle_temperature_avg: Some(37.0),
        sample_minutes: 210,
      },
      delta: Some(7.0),
      observation: CoreCoolingDeltaObservation::SustainedMildRise,
      daily_deltas: vec![
        CoreDailyDelta {
          date: date(2026, 8, 18),
          delta: 5.5,
        },
        CoreDailyDelta {
          date: date(2026, 8, 19),
          delta: 6.5,
        },
        CoreDailyDelta {
          date: date(2026, 8, 20),
          delta: 7.0,
        },
      ],
      sustained_days: 3,
      // This case is about the absolute daily series, so the ambient
      // reading is the one a machine with no sensor reports.
      ambient_adjusted: CoreAmbientAdjustedBaselineDelta {
        baseline_state: CoreDeltaBaselineState::Establishing {
          qualifying_days: 0,
          required_days: 7,
        },
        recent: CoreBandDeltaWindowSummary::default(),
        delta: None,
        comparable: false,
      },
    };

    let wire: CoolingBaselineDelta = core.into();

    assert_eq!(wire.observation, CoolingDeltaObservation::SustainedMildRise);
    assert_eq!(wire.sustained_days, 3);
    assert_eq!(wire.delta, Some(7.0));
    assert_eq!(
      wire
        .daily_deltas
        .iter()
        .map(|d| d.date.clone())
        .collect::<Vec<_>>(),
      vec!["2026-08-18", "2026-08-19", "2026-08-20"]
    );
  }

  // ── ambient-adjusted wire shape (#2045) ──

  fn establishing_delta_baseline() -> CoreDeltaBaselineState {
    CoreDeltaBaselineState::Establishing {
      qualifying_days: 0,
      required_days: 7,
    }
  }

  fn established_delta_baseline() -> CoreDeltaBaselineState {
    CoreDeltaBaselineState::Established {
      source: "Living Room".to_string(),
      delta_temperature_avg: 12.0,
      window_start_date: date(2026, 6, 1),
      window_end_date: date(2026, 6, 7),
      sample_minutes: 210,
    }
  }

  fn core_baseline_delta(
    ambient_adjusted: CoreAmbientAdjustedBaselineDelta,
  ) -> CoreCoolingBaselineDelta {
    CoreCoolingBaselineDelta {
      baseline_state: CoreBaselineState::Established {
        idle_temperature_avg: 30.0,
        window_start_date: date(2026, 1, 1),
        window_end_date: date(2026, 1, 7),
        sample_minutes: 210,
      },
      recent: CoreRecentIdleSummary {
        window_start_date: date(2026, 8, 14),
        window_end_date: date(2026, 8, 20),
        idle_temperature_avg: Some(37.0),
        sample_minutes: 210,
      },
      delta: Some(7.0),
      observation: CoreCoolingDeltaObservation::SustainedMildRise,
      daily_deltas: Vec::new(),
      sustained_days: 3,
      ambient_adjusted,
    }
  }

  #[test]
  fn a_machine_without_ambient_data_sends_an_establishing_delta_baseline() {
    // The zero-ambient wire invariant: every pre-#2045 field keeps its
    // value, and the ambient reading reports progress rather than a
    // fabricated number the frontend might render as "0 K of drift".
    let json = serde_json::to_value(CoolingBaselineDelta::from(core_baseline_delta(
      CoreAmbientAdjustedBaselineDelta {
        baseline_state: establishing_delta_baseline(),
        recent: CoreBandDeltaWindowSummary::default(),
        delta: None,
        comparable: false,
      },
    )))
    .unwrap();

    let adjusted = &json["ambientAdjusted"];
    assert_eq!(adjusted["baseline"]["status"], "establishing");
    assert_eq!(adjusted["baseline"]["qualifyingDays"], 0);
    assert_eq!(adjusted["baseline"]["requiredDays"], 7);
    assert!(adjusted["delta"].is_null());
    assert_eq!(adjusted["comparable"], false);
    assert!(adjusted["recent"]["deltaAvg"].is_null());
    // ...and nothing above it moved.
    assert_eq!(json["delta"], 7.0);
    assert_eq!(json["observation"], "sustainedMildRise");
    assert_eq!(json["sustainedDays"], 3);
  }

  #[test]
  fn an_ambient_adjusted_baseline_delta_crosses_the_wire_in_camel_case() {
    let core = core_baseline_delta(CoreAmbientAdjustedBaselineDelta {
      baseline_state: established_delta_baseline(),
      recent: CoreBandDeltaWindowSummary {
        delta_avg: Some(12.5),
        sample_minutes: 180,
      },
      delta: Some(0.5),
      comparable: true,
    });

    let json = serde_json::to_value(CoolingBaselineDelta::from(core)).unwrap();

    let adjusted = &json["ambientAdjusted"];
    assert_eq!(adjusted["baseline"]["status"], "established");
    assert_eq!(adjusted["baseline"]["deltaTemperatureAvg"], 12.0);
    assert_eq!(adjusted["baseline"]["sampleMinutes"], 210);
    assert_eq!(adjusted["recent"]["deltaAvg"], 12.5);
    assert_eq!(adjusted["recent"]["sampleMinutes"], 180);
    assert_eq!(adjusted["delta"], 0.5);
    assert_eq!(adjusted["comparable"], true);
  }

  #[test]
  fn the_delta_baseline_window_crosses_the_wire_as_its_own_dates() {
    // The whole reason the window is on the wire (#2046 renders the
    // comparison's conditions): it must show the ΔT window, which is a
    // different range from the absolute baseline's.
    let core = core_baseline_delta(CoreAmbientAdjustedBaselineDelta {
      baseline_state: established_delta_baseline(),
      recent: CoreBandDeltaWindowSummary {
        delta_avg: Some(12.5),
        sample_minutes: 180,
      },
      delta: Some(0.5),
      comparable: true,
    });

    let json = serde_json::to_value(CoolingBaselineDelta::from(core)).unwrap();

    assert_eq!(
      json["ambientAdjusted"]["baseline"]["windowStartDate"],
      "2026-06-01"
    );
    assert_eq!(
      json["ambientAdjusted"]["baseline"]["windowEndDate"],
      "2026-06-07"
    );
    // The absolute baseline's own window is untouched beside it.
    assert_eq!(json["baseline"]["windowStartDate"], "2026-01-01");
    assert_eq!(json["baseline"]["windowEndDate"], "2026-01-07");
  }

  #[test]
  fn a_thin_recent_window_crosses_the_wire_present_with_a_null_delta() {
    // Distinct from an establishing baseline: the reference exists and
    // the evidence is reported, only the verdict is withheld.
    let core = core_baseline_delta(CoreAmbientAdjustedBaselineDelta {
      baseline_state: established_delta_baseline(),
      recent: CoreBandDeltaWindowSummary {
        delta_avg: Some(12.5),
        sample_minutes: 5,
      },
      delta: None,
      comparable: false,
    });

    let json = serde_json::to_value(CoolingBaselineDelta::from(core)).unwrap();

    assert_eq!(json["ambientAdjusted"]["baseline"]["status"], "established");
    assert!(json["ambientAdjusted"]["delta"].is_null());
    assert_eq!(json["ambientAdjusted"]["comparable"], false);
    assert_eq!(json["ambientAdjusted"]["recent"]["sampleMinutes"], 5);
  }

  #[test]
  fn a_band_comparison_entry_carries_its_ambient_adjusted_reading_in_camel_case() {
    let core = CoreBandComparison {
      band: CpuLoadBand::Idle,
      baseline: CoreBandWindowSummary {
        temperature_avg: Some(30.0),
        sample_minutes: 210,
      },
      recent: CoreBandWindowSummary {
        temperature_avg: Some(35.0),
        sample_minutes: 210,
      },
      comparable: true,
      ambient_adjusted: Some(CoreAmbientAdjustedBandComparison {
        baseline: CoreBandDeltaWindowSummary {
          delta_avg: Some(8.0),
          sample_minutes: 210,
        },
        recent: CoreBandDeltaWindowSummary {
          delta_avg: Some(8.25),
          sample_minutes: 200,
        },
        comparable: true,
      }),
    };

    let json = serde_json::to_value(CoolingBandComparisonEntry::from(core)).unwrap();

    assert_eq!(json["ambientAdjusted"]["baseline"]["deltaAvg"], 8.0);
    assert_eq!(json["ambientAdjusted"]["recent"]["deltaAvg"], 8.25);
    assert_eq!(json["ambientAdjusted"]["recent"]["sampleMinutes"], 200);
    // The absolute reading is untouched beside it.
    assert_eq!(json["baseline"]["temperatureAvg"], 30.0);
    assert_eq!(json["comparable"], true);
  }

  #[test]
  fn a_band_without_ambient_data_sends_a_null_ambient_adjusted_field() {
    let core = CoreBandComparison {
      band: CpuLoadBand::Idle,
      baseline: CoreBandWindowSummary {
        temperature_avg: Some(30.0),
        sample_minutes: 210,
      },
      recent: CoreBandWindowSummary {
        temperature_avg: Some(35.0),
        sample_minutes: 210,
      },
      comparable: true,
      ambient_adjusted: None,
    };

    let json = serde_json::to_value(CoolingBandComparisonEntry::from(core)).unwrap();

    assert!(json["ambientAdjusted"].is_null());
    assert_eq!(json["baseline"]["temperatureAvg"], 30.0);
    assert_eq!(json["recent"]["temperatureAvg"], 35.0);
    assert_eq!(json["comparable"], true);
  }

  // ── load-vs-temperature Explorer (#2023) ──

  fn hour(input: &str) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M").unwrap()
  }

  fn core_explorer_window(start: NaiveDate, end: NaiveDate) -> CoreExplorerWindow {
    CoreExplorerWindow {
      start_date: start,
      end_date: end,
      points: vec![CoreLoadTemperaturePoint {
        hour_start: hour("2026-08-20 13:00"),
        cpu_usage_avg: 45.5,
        cpu_temperature_avg: 62.25,
        sample_minutes: 60,
      }],
    }
  }

  #[test]
  fn an_explorer_point_carries_its_local_wall_clock_hour_as_a_string() {
    let wire: CoolingLoadTemperaturePoint = CoreLoadTemperaturePoint {
      hour_start: hour("2026-08-05 09:00"),
      cpu_usage_avg: 5.0,
      cpu_temperature_avg: 40.0,
      sample_minutes: 60,
    }
    .into();

    assert_eq!(wire.hour_start, "2026-08-05 09:00");
  }

  #[test]
  fn an_established_explorer_formats_both_window_ranges_and_every_band() {
    let core = CoreCoolingLoadTemperatureExplorer::Established {
      baseline: core_explorer_window(date(2026, 1, 1), date(2026, 1, 7)),
      recent: core_explorer_window(date(2026, 7, 24), date(2026, 8, 20)),
      band_deltas: vec![
        CoreBandMedianDelta {
          band: CpuLoadBand::Idle,
          baseline: CoreBandMedian {
            temperature_median: Some(30.0),
            point_count: 12,
            sample_minutes: 720,
          },
          recent: CoreBandMedian {
            temperature_median: Some(36.5),
            point_count: 20,
            sample_minutes: 1_200,
          },
          delta: Some(6.5),
          comparable: true,
        },
        CoreBandMedianDelta {
          band: CpuLoadBand::High,
          baseline: CoreBandMedian::default(),
          recent: CoreBandMedian::default(),
          delta: None,
          comparable: false,
        },
      ],
    };

    let wire: CoolingLoadTemperatureExplorer = core.into();

    match wire {
      CoolingLoadTemperatureExplorer::Established {
        baseline,
        recent,
        band_deltas,
      } => {
        assert_eq!(baseline.start_date, "2026-01-01");
        assert_eq!(baseline.end_date, "2026-01-07");
        assert_eq!(recent.start_date, "2026-07-24");
        assert_eq!(recent.points.len(), 1);
        assert_eq!(band_deltas[0].band, CoolingLoadBand::Idle);
        assert_eq!(band_deltas[0].delta, Some(6.5));
        assert!(band_deltas[0].comparable);
        assert_eq!(
          band_deltas[1].delta, None,
          "an uncomparable band must not carry a delta across the wire"
        );
      }
      other => panic!("expected an established explorer, got {other:?}"),
    }
  }

  #[test]
  fn an_established_explorer_serializes_its_fields_as_camel_case() {
    let wire = CoolingLoadTemperatureExplorer::Established {
      baseline: CoolingExplorerWindow {
        start_date: "2026-01-01".to_string(),
        end_date: "2026-01-07".to_string(),
        points: vec![CoolingLoadTemperaturePoint {
          hour_start: "2026-01-01 09:00".to_string(),
          cpu_usage_avg: 5.0,
          cpu_temperature_avg: 40.0,
          sample_minutes: 60,
        }],
      },
      recent: CoolingExplorerWindow {
        start_date: "2026-07-24".to_string(),
        end_date: "2026-08-20".to_string(),
        points: Vec::new(),
      },
      band_deltas: Vec::new(),
    };

    let json = serde_json::to_value(&wire).unwrap();

    assert_eq!(json["status"], "established");
    assert_eq!(json["baseline"]["startDate"], "2026-01-01");
    assert_eq!(json["baseline"]["points"][0]["cpuUsageAvg"], 5.0);
    assert_eq!(
      json["baseline"]["points"][0]["hourStart"],
      "2026-01-01 09:00"
    );
    assert!(
      json.get("band_deltas").is_none(),
      "must not also serialize the snake_case field name"
    );
    assert!(json.get("bandDeltas").is_some());
  }

  #[test]
  fn an_establishing_explorer_carries_its_progress_through() {
    let core = CoreCoolingLoadTemperatureExplorer::Establishing {
      qualifying_days: 3,
      required_days: 7,
    };

    let wire: CoolingLoadTemperatureExplorer = core.into();

    assert_eq!(
      wire,
      CoolingLoadTemperatureExplorer::Establishing {
        qualifying_days: 3,
        required_days: 7,
      }
    );
  }
  // --- Co-variate comparison (#2068) ---

  fn core_factor(
    baseline: f32,
    recent: f32,
    judgement: CoreFactorJudgement,
  ) -> CoreFactorComparison {
    CoreFactorComparison {
      baseline: Some(baseline),
      recent: Some(recent),
      change: Some(recent - baseline),
      judgement,
    }
  }

  fn core_fit(slope: f32, intercept: f32, paired_minutes: u32) -> CoreLeastSquaresFit {
    CoreLeastSquaresFit {
      slope,
      intercept,
      pearson_r: 0.875,
      paired_minutes,
    }
  }

  /// A comparable idle-band comparison whose ambient stayed within range
  /// while package power moved: 1.0 K/W baseline, 1.25 K/W recent, so the
  /// recent line sits 3.9 K higher at the baseline's 20 W median.
  fn core_established_comparison() -> CoreEstablishedCovariateComparison {
    CoreEstablishedCovariateComparison {
      band: CpuLoadBand::Idle,
      baseline_source: "Living Room".to_string(),
      baseline_window_start_date: date(2026, 8, 1),
      baseline_window_end_date: date(2026, 8, 7),
      recent_source: Some("Living Room".to_string()),
      recent_window_start_date: date(2026, 8, 26),
      recent_window_end_date: date(2026, 9, 1),
      baseline_paired_minutes: 700,
      recent_paired_minutes: 640,
      package_power: core_factor(20.0, 24.5, CoreFactorJudgement::Moved),
      ambient_temperature: core_factor(25.0, 25.5, CoreFactorJudgement::WithinRange),
      load_band_share: core_factor(0.6, 0.55, CoreFactorJudgement::WithinRange),
      fans: vec![CoreFanCovariateComparison {
        fan_source: "fan:cpu".to_string(),
        speed: core_factor(900.0, 880.0, CoreFactorJudgement::WithinRange),
        baseline_fit: Some(core_fit(-0.01, 14.0, 700)),
        recent_fit: None,
      }],
      baseline_fit: Some(core_fit(1.0, 5.0, 700)),
      recent_fit: Some(core_fit(1.25, 3.9, 640)),
      delta_at_baseline_median_power: Some(3.9),
      comparable: true,
      comparability: CoreCovariateComparability::Comparable,
    }
  }

  #[test]
  fn an_establishing_covariate_comparison_carries_its_progress_through() {
    let core = CoreCoolingCovariateComparison::Establishing {
      qualifying_days: 2,
      required_days: 7,
    };

    let wire: CoolingCovariateComparison = core.into();

    assert_eq!(
      wire,
      CoolingCovariateComparison::Establishing {
        qualifying_days: 2,
        required_days: 7,
      }
    );
  }

  #[test]
  fn every_factor_judgement_maps_one_to_one() {
    let pairs: [(CoreFactorJudgement, CoolingFactorJudgement); 4] = [
      (
        CoreFactorJudgement::WithinRange,
        CoolingFactorJudgement::WithinRange,
      ),
      (CoreFactorJudgement::Moved, CoolingFactorJudgement::Moved),
      (
        CoreFactorJudgement::NotComparable,
        CoolingFactorJudgement::NotComparable,
      ),
      (CoreFactorJudgement::Absent, CoolingFactorJudgement::Absent),
    ];

    for (core, wire) in pairs {
      assert_eq!(CoolingFactorJudgement::from(core), wire);
    }
  }

  #[test]
  fn every_comparability_reason_maps_one_to_one() {
    let pairs: [(CoreCovariateComparability, CoolingCovariateComparability); 3] = [
      (
        CoreCovariateComparability::Comparable,
        CoolingCovariateComparability::Comparable,
      ),
      (
        CoreCovariateComparability::TooFewPairedMinutes,
        CoolingCovariateComparability::TooFewPairedMinutes,
      ),
      (
        CoreCovariateComparability::DifferentAmbientSource,
        CoolingCovariateComparability::DifferentAmbientSource,
      ),
    ];

    for (core, wire) in pairs {
      assert_eq!(CoolingCovariateComparability::from(core), wire);
    }
  }

  #[test]
  fn factor_judgements_and_comparability_reasons_serialize_as_camel_case_tags() {
    assert_eq!(
      serde_json::to_value(CoolingFactorJudgement::WithinRange).unwrap(),
      "withinRange"
    );
    assert_eq!(
      serde_json::to_value(CoolingFactorJudgement::NotComparable).unwrap(),
      "notComparable"
    );
    assert_eq!(
      serde_json::to_value(CoolingCovariateComparability::TooFewPairedMinutes).unwrap(),
      "tooFewPairedMinutes"
    );
    assert_eq!(
      serde_json::to_value(CoolingCovariateComparability::DifferentAmbientSource)
        .unwrap(),
      "differentAmbientSource"
    );
  }

  #[test]
  fn an_absent_factor_crosses_the_wire_with_null_values_not_zeros() {
    let wire: CoolingFactorComparison = CoreFactorComparison {
      baseline: None,
      recent: None,
      change: None,
      judgement: CoreFactorJudgement::Absent,
    }
    .into();

    let json = serde_json::to_value(wire).unwrap();

    assert!(json["baseline"].is_null());
    assert!(json["recent"].is_null());
    assert!(json["change"].is_null());
    assert_eq!(json["judgement"], "absent");
  }

  #[test]
  fn a_missing_fit_crosses_the_wire_as_null_rather_than_a_flat_line() {
    let mut core = core_established_comparison();
    core.recent_fit = None;
    core.delta_at_baseline_median_power = None;

    let json = serde_json::to_value(CoolingCovariateComparison::from(
      CoreCoolingCovariateComparison::Established(Box::new(core)),
    ))
    .unwrap();

    assert!(json["recentFit"].is_null());
    assert!(json["deltaAtBaselineMedianPower"].is_null());
    assert_eq!(json["baselineFit"]["slope"], 1.0);
    assert_eq!(json["baselineFit"]["pairedMinutes"], 700);
    assert!(json["fans"][0]["recentFit"].is_null());
  }

  // Temperatures are not converted here: like every other Cooling
  // Insight DTO, the ambient medians stay in Celsius and the ΔT change
  // and fit slopes in kelvin, and the frontend applies the preferred unit
  // (a delta scaled by 9/5 without the offset). The command therefore
  // never reads the temperature-unit preference, and the same numbers
  // must come out whatever it is set to.
  #[test]
  fn ambient_medians_and_kelvin_differences_cross_the_wire_unconverted() {
    let wire: CoolingCovariateComparison = CoreCoolingCovariateComparison::Established(
      Box::new(core_established_comparison()),
    )
    .into();

    match wire {
      CoolingCovariateComparison::Established {
        ambient_temperature,
        baseline_fit,
        recent_fit,
        delta_at_baseline_median_power,
        ..
      } => {
        assert_eq!(ambient_temperature.baseline, Some(25.0));
        assert_eq!(ambient_temperature.recent, Some(25.5));
        assert_eq!(ambient_temperature.change, Some(0.5));
        assert_eq!(baseline_fit.map(|fit| fit.slope), Some(1.0));
        assert_eq!(recent_fit.map(|fit| fit.slope), Some(1.25));
        assert_eq!(delta_at_baseline_median_power, Some(3.9));
      }
      other => panic!("expected an established comparison, got {other:?}"),
    }
  }

  #[test]
  fn an_established_covariate_comparison_formats_its_dates_and_maps_every_part() {
    let wire: CoolingCovariateComparison = CoreCoolingCovariateComparison::Established(
      Box::new(core_established_comparison()),
    )
    .into();

    match wire {
      CoolingCovariateComparison::Established {
        band,
        baseline_source,
        baseline_window_start_date,
        baseline_window_end_date,
        recent_source,
        recent_window_start_date,
        recent_window_end_date,
        baseline_paired_minutes,
        recent_paired_minutes,
        package_power,
        load_band_share,
        fans,
        comparable,
        comparability,
        ..
      } => {
        assert_eq!(band, CoolingLoadBand::Idle);
        assert_eq!(baseline_source, "Living Room");
        assert_eq!(baseline_window_start_date, "2026-08-01");
        assert_eq!(baseline_window_end_date, "2026-08-07");
        assert_eq!(recent_source.as_deref(), Some("Living Room"));
        assert_eq!(recent_window_start_date, "2026-08-26");
        assert_eq!(recent_window_end_date, "2026-09-01");
        assert_eq!(baseline_paired_minutes, 700);
        assert_eq!(recent_paired_minutes, 640);
        assert_eq!(package_power.judgement, CoolingFactorJudgement::Moved);
        assert_eq!(package_power.change, Some(4.5));
        assert_eq!(
          load_band_share.judgement,
          CoolingFactorJudgement::WithinRange
        );
        assert_eq!(fans.len(), 1);
        assert_eq!(fans[0].fan_source, "fan:cpu");
        assert_eq!(fans[0].speed.baseline, Some(900.0));
        assert_eq!(fans[0].baseline_fit.map(|fit| fit.slope), Some(-0.01));
        assert!(comparable);
        assert_eq!(comparability, CoolingCovariateComparability::Comparable);
      }
      other => panic!("expected an established comparison, got {other:?}"),
    }
  }

  #[test]
  fn an_established_covariate_comparison_serializes_its_fields_as_camel_case() {
    let json = serde_json::to_value(CoolingCovariateComparison::from(
      CoreCoolingCovariateComparison::Established(
        Box::new(core_established_comparison()),
      ),
    ))
    .unwrap();

    assert_eq!(json["status"], "established");
    assert_eq!(json["band"], "idle");
    assert_eq!(json["baselineWindowStartDate"], "2026-08-01");
    assert_eq!(json["recentSource"], "Living Room");
    assert_eq!(json["packagePower"]["judgement"], "moved");
    assert_eq!(json["ambientTemperature"]["judgement"], "withinRange");
    assert_eq!(json["fans"][0]["fanSource"], "fan:cpu");
    assert_eq!(json["fans"][0]["baselineFit"]["pearsonR"], 0.875);
    assert_eq!(json["comparability"], "comparable");
    assert!(
      json.get("baseline_window_start_date").is_none()
        && json.get("delta_at_baseline_median_power").is_none(),
      "must not also serialize the snake_case field names"
    );
  }

  #[test]
  fn an_uncomparable_covariate_comparison_still_reports_both_windows() {
    let mut core = core_established_comparison();
    core.recent_source = Some("Bedroom".to_string());
    core.comparable = false;
    core.comparability = CoreCovariateComparability::DifferentAmbientSource;
    core.delta_at_baseline_median_power = None;
    core.package_power.judgement = CoreFactorJudgement::NotComparable;

    let json = serde_json::to_value(CoolingCovariateComparison::from(
      CoreCoolingCovariateComparison::Established(Box::new(core)),
    ))
    .unwrap();

    assert_eq!(json["comparable"], false);
    assert_eq!(json["comparability"], "differentAmbientSource");
    assert_eq!(json["recentSource"], "Bedroom");
    assert_eq!(json["packagePower"]["judgement"], "notComparable");
    assert_eq!(json["packagePower"]["recent"], 24.5);
    assert!(json["deltaAtBaselineMedianPower"].is_null());
    assert_eq!(
      json["recentFit"]["slope"], 1.25,
      "a window's own fit is reported even when the windows are not compared"
    );
  }

  #[test]
  fn a_wire_load_band_names_the_core_band_the_query_reads() {
    let pairs: [(&str, CpuLoadBand); 4] = [
      ("\"idle\"", CpuLoadBand::Idle),
      ("\"low\"", CpuLoadBand::Low),
      ("\"mid\"", CpuLoadBand::Mid),
      ("\"high\"", CpuLoadBand::High),
    ];

    for (json, core) in pairs {
      let wire: CoolingLoadBand = serde_json::from_str(json).unwrap();
      assert_eq!(CpuLoadBand::from(wire), core);
    }
  }
}
