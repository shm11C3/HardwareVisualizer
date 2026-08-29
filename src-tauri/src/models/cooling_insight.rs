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
  BandComparison as CoreBandComparison, BandWindowSummary as CoreBandWindowSummary,
  CoolingBandComparison as CoreCoolingBandComparison,
};
use hardviz_core::persistence::cooling_baseline::{
  BaselineState as CoreBaselineState, RecentIdleSummary as CoreRecentIdleSummary,
};
use hardviz_core::persistence::cooling_baseline_delta::{
  CoolingBaselineDelta as CoreCoolingBaselineDelta,
  CoolingDeltaObservation as CoreCoolingDeltaObservation, DailyDelta as CoreDailyDelta,
};
use hardviz_core::persistence::cooling_rollup::{
  BandSummary as CoreBandSummary, CpuLoadBand as CoreCpuLoadBand,
  DailyCoolingSummary as CoreDailyCoolingSummary,
};
use serde::Serialize;
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
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
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

/// One CPU-load band's baseline-vs-recent comparison.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoolingBandComparisonEntry {
  pub band: CoolingLoadBand,
  pub baseline: CoolingBandWindowSummary,
  pub recent: CoolingBandWindowSummary,
  pub comparable: bool,
}

impl From<CoreBandComparison> for CoolingBandComparisonEntry {
  fn from(value: CoreBandComparison) -> Self {
    Self {
      band: value.band.into(),
      baseline: value.baseline.into(),
      recent: value.recent.into(),
      comparable: value.comparable,
    }
  }
}

/// Cooling Insight's load-band comparison, gated by the same baseline
/// lifecycle as [`CoolingBaselineState`]: no comparison exists yet while
/// the baseline is still establishing.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "status", rename_all = "camelCase")]
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
      } => Self::Established {
        baseline_window_start_date: format_date(baseline_window_start_date),
        baseline_window_end_date: format_date(baseline_window_end_date),
        recent_window_start_date: format_date(recent_window_start_date),
        recent_window_end_date: format_date(recent_window_end_date),
        bands: bands.into_iter().map(Into::into).collect(),
      },
    }
  }
}

/// Lifecycle of the idle cooling baseline (see
/// `hardviz_core::persistence::cooling_baseline::BaselineState`).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "status", rename_all = "camelCase")]
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
    };

    let wire: CoolingDailyTrendPoint = core.into();

    assert_eq!(wire.date, "2026-08-05");
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
      bands: [
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
        },
        CoreBandComparison {
          band: CpuLoadBand::Low,
          baseline: CoreBandWindowSummary::default(),
          recent: CoreBandWindowSummary::default(),
          comparable: false,
        },
        CoreBandComparison {
          band: CpuLoadBand::Mid,
          baseline: CoreBandWindowSummary::default(),
          recent: CoreBandWindowSummary::default(),
          comparable: false,
        },
        CoreBandComparison {
          band: CpuLoadBand::High,
          baseline: CoreBandWindowSummary::default(),
          recent: CoreBandWindowSummary::default(),
          comparable: false,
        },
      ],
    };

    let wire: CoolingBandComparison = core.into();

    match wire {
      CoolingBandComparison::Established {
        baseline_window_start_date,
        baseline_window_end_date,
        recent_window_start_date,
        recent_window_end_date,
        bands,
      } => {
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
}
