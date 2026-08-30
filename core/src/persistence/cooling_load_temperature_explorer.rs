//! Cooling Insight's load-vs-temperature Explorer (#2023): the same CPU
//! load bands the comparison panel uses, but resolved to the individual
//! hourly (load, temperature) pairs behind them, for two windows at once.
//!
//! The baseline window is the calendar range that established the idle
//! cooling baseline (see [`crate::persistence::cooling_baseline`]), so
//! "baseline" here means exactly what it means everywhere else in Cooling
//! Insight rather than each view picking its own reference period. The
//! recent window is a caller-chosen trailing span, clamped here rather
//! than trusted from IPC.
//!
//! Every number the Explorer draws is decided at this boundary: the
//! per-band medians, the per-band deltas, and whether a band carries
//! enough evidence for its delta to mean anything
//! ([`crate::persistence::cooling_band_comparison::COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES`],
//! the same bar the band comparison applies). The frontend only draws
//! them.

use chrono::{Duration, NaiveDate, NaiveDateTime};

use crate::persistence::cooling_band_comparison::COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES;
use crate::persistence::cooling_baseline::BaselineState;
use crate::persistence::cooling_hourly_rollup::HourlyCoolingSummary;
use crate::persistence::cooling_rollup::CpuLoadBand;

/// Shortest recent window the Explorer will look at. Below a week the
/// scatter stops spanning a full usage cycle, and the per-band sample
/// counts fall under the comparability bar for most machines anyway.
pub const COOLING_EXPLORER_MIN_RECENT_DAYS: u32 = 7;

/// Longest recent window the Explorer will look at. Bounds both the row
/// count the scatter has to carry across IPC and the `NaiveDate`
/// arithmetic below, for any `u32` the command boundary lets through.
pub const COOLING_EXPLORER_MAX_RECENT_DAYS: u32 = 90;

/// Every load band, in ascending load order - the order the Explorer's
/// medians, deltas, and band dividers are all reported in.
const BANDS: [CpuLoadBand; 4] = [
  CpuLoadBand::Idle,
  CpuLoadBand::Low,
  CpuLoadBand::Mid,
  CpuLoadBand::High,
];

/// One hour's (load, temperature) pair, as scattered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadTemperaturePoint {
  pub hour_start: NaiveDateTime,
  pub cpu_usage_avg: f32,
  pub cpu_temperature_avg: f32,
  /// Archived minutes behind this point (see
  /// [`HourlyCoolingSummary::sample_minutes`]). Carried through so a
  /// point resting on three minutes is not presented as equal in weight
  /// to a full hour.
  pub sample_minutes: u32,
}

/// One band's temperature median within one window, with the evidence
/// behind it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BandMedian {
  pub temperature_median: Option<f32>,
  pub point_count: u32,
  pub sample_minutes: u32,
}

impl BandMedian {
  fn is_comparable(&self) -> bool {
    self.sample_minutes >= COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES
  }
}

/// One of the Explorer's two windows: its calendar range and its scatter
/// points.
///
/// The per-band medians deliberately live on [`BandMedianDelta`] instead
/// of here: every consumer draws a band's two medians together (as the
/// trend line and as the delta row), so splitting them across the two
/// windows would only make callers re-pair what this module already
/// paired.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplorerWindow {
  pub start_date: NaiveDate,
  pub end_date: NaiveDate,
  pub points: Vec<LoadTemperaturePoint>,
}

/// One band's two window medians and the delta between them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandMedianDelta {
  pub band: CpuLoadBand,
  pub baseline: BandMedian,
  pub recent: BandMedian,
  /// `recent` minus `baseline`, in degC. `None` whenever `comparable` is
  /// `false` - a delta is either evidence or it is absent, never a number
  /// the caller has to know not to trust.
  pub delta: Option<f32>,
  /// Whether both windows carry at least
  /// [`COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES`] behind this band.
  pub comparable: bool,
}

/// The Explorer's result, gated by the same baseline lifecycle as every
/// other Cooling Insight query: without an established baseline there is
/// no baseline window to scatter against, so there is no partial answer to
/// give.
#[derive(Debug, Clone, PartialEq)]
pub enum CoolingLoadTemperatureExplorer {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    baseline: ExplorerWindow,
    recent: ExplorerWindow,
    /// One entry per [`CpuLoadBand`], in [`BANDS`] order. A `Vec` rather
    /// than `[BandMedianDelta; 4]` so the variant stays small enough to
    /// pass by value: this enum is returned by value from the query and
    /// converted straight to its wire DTO (which is `Vec`-shaped anyway),
    /// so boxing a fixed array would only add indirection.
    band_deltas: Vec<BandMedianDelta>,
  },
}

/// Number of trailing days the recent window actually covers for a
/// requested `recent_days`, clamped to
/// `COOLING_EXPLORER_MIN_RECENT_DAYS..=COOLING_EXPLORER_MAX_RECENT_DAYS`.
///
/// Clamped in Core rather than validated at the command boundary: the
/// window length is a property of what the rollup can answer for, not of
/// how the request arrived.
pub fn clamp_recent_days(recent_days: u32) -> u32 {
  recent_days.clamp(
    COOLING_EXPLORER_MIN_RECENT_DAYS,
    COOLING_EXPLORER_MAX_RECENT_DAYS,
  )
}

/// Derive the Explorer from the hourly rollup rows covering both windows
/// and the current baseline lifecycle state.
///
/// `window_end_date` is the most recent completed local day (yesterday),
/// matching every other Cooling Insight derivation. `hours` may hold rows
/// outside either window; they are filtered here.
pub fn derive_load_temperature_explorer(
  hours: &[HourlyCoolingSummary],
  baseline_state: BaselineState,
  window_end_date: NaiveDate,
  recent_days: u32,
) -> CoolingLoadTemperatureExplorer {
  let (baseline_start, baseline_end) = match baseline_state {
    BaselineState::Establishing {
      qualifying_days,
      required_days,
    } => {
      return CoolingLoadTemperatureExplorer::Establishing {
        qualifying_days,
        required_days,
      };
    }
    BaselineState::Established {
      window_start_date,
      window_end_date,
      ..
    } => (window_start_date, window_end_date),
  };

  let recent_start =
    window_end_date - Duration::days(clamp_recent_days(recent_days) as i64 - 1);

  let baseline = build_window(hours, baseline_start, baseline_end);
  let recent = build_window(hours, recent_start, window_end_date);

  let band_deltas = BANDS
    .iter()
    .map(|&band| {
      let baseline_median = band_median(&baseline.points, band);
      let recent_median = band_median(&recent.points, band);
      let comparable = baseline_median.is_comparable() && recent_median.is_comparable();
      BandMedianDelta {
        band,
        baseline: baseline_median,
        recent: recent_median,
        delta: comparable
          .then(|| {
            Some(recent_median.temperature_median? - baseline_median.temperature_median?)
          })
          .flatten(),
        comparable,
      }
    })
    .collect();

  CoolingLoadTemperatureExplorer::Established {
    baseline,
    recent,
    band_deltas,
  }
}

/// Collect one window's scatter points.
///
/// A row is only a scatter point when it carries both readings: the
/// rollup already guarantees that for every persisted row, but the
/// columns are nullable, so a row that somehow lacks either is dropped
/// rather than plotted at an invented coordinate.
fn build_window(
  hours: &[HourlyCoolingSummary],
  start_date: NaiveDate,
  end_date: NaiveDate,
) -> ExplorerWindow {
  let points: Vec<LoadTemperaturePoint> = hours
    .iter()
    .filter(|hour| {
      let date = hour.hour_start.date();
      date >= start_date && date <= end_date
    })
    .filter_map(|hour| {
      Some(LoadTemperaturePoint {
        hour_start: hour.hour_start,
        cpu_usage_avg: hour.cpu_usage_avg?,
        cpu_temperature_avg: hour.cpu_temperature_avg?,
        sample_minutes: hour.sample_minutes,
      })
    })
    .collect();

  ExplorerWindow {
    start_date,
    end_date,
    points,
  }
}

/// Median temperature across the points that fall in `band`, plus the
/// evidence behind it.
///
/// The median (not the mean) because the scatter's whole point is that a
/// band mixes very different workloads: one hour of a stress test should
/// move the trend line no further than one hour of anything else.
fn band_median(points: &[LoadTemperaturePoint], band: CpuLoadBand) -> BandMedian {
  let mut temperatures: Vec<f32> = Vec::new();
  let mut sample_minutes: u64 = 0;

  for point in points
    .iter()
    .filter(|point| CpuLoadBand::classify(point.cpu_usage_avg) == band)
  {
    temperatures.push(point.cpu_temperature_avg);
    sample_minutes += point.sample_minutes as u64;
  }

  BandMedian {
    point_count: temperatures.len() as u32,
    temperature_median: median(&mut temperatures),
    sample_minutes: sample_minutes.min(u32::MAX as u64) as u32,
  }
}

/// Median of `values`, averaging the two middle values for an even count.
/// `None` for an empty slice - a band nothing landed in has no median,
/// never zero. Sorts `values` in place; NaN is not expected from the
/// rollup (it only stores means of recorded readings) and would sort to
/// one end under `total_cmp` rather than corrupting the ordering.
fn median(values: &mut [f32]) -> Option<f32> {
  if values.is_empty() {
    return None;
  }
  values.sort_by(|a, b| a.total_cmp(b));

  let middle = values.len() / 2;
  Some(if values.len() % 2 == 1 {
    values[middle]
  } else {
    ((values[middle - 1] as f64 + values[middle] as f64) / 2.0) as f32
  })
}

/// [`derive_load_temperature_explorer`] against an explicit pool.
///
/// Resolves the baseline lifecycle through the shared resolver (so the
/// pinned baseline wins once one exists, exactly as the band comparison
/// does), then reads only the hourly rows the two resulting windows
/// cover.
pub(crate) async fn load_cooling_load_temperature_explorer_from_pool(
  pool: &sqlx::SqlitePool,
  today: NaiveDate,
  recent_days: u32,
) -> Result<CoolingLoadTemperatureExplorer, sqlx::Error> {
  use crate::infrastructure::database;
  use crate::persistence::cooling_baseline::resolve_baseline_state_from_pool;

  let idle_samples =
    database::cooling_daily_summary::select_daily_idle_samples_from_pool(pool).await?;
  let baseline_state = resolve_baseline_state_from_pool(pool, &idle_samples).await?;
  let yesterday = today - Duration::days(1);

  let BaselineState::Established {
    window_start_date, ..
  } = baseline_state
  else {
    // No baseline window exists yet, so there is nothing to read hours
    // for; the derivation returns `Establishing` from the same state.
    return Ok(derive_load_temperature_explorer(
      &[],
      baseline_state,
      yesterday,
      recent_days,
    ));
  };

  // One read spanning both windows. They can be a year apart, but the
  // rows between them are narrow and bounded by the rollup's retention,
  // and a single range keeps this to one query.
  let hours = database::cooling_hourly_summary::select_hours_in_date_range_from_pool(
    pool,
    window_start_date
      .min(yesterday - Duration::days(clamp_recent_days(recent_days) as i64 - 1)),
    yesterday,
  )
  .await?;

  Ok(derive_load_temperature_explorer(
    &hours,
    baseline_state,
    yesterday,
    recent_days,
  ))
}

/// [`load_cooling_load_temperature_explorer_from_pool`] against Core's
/// process-wide pool.
pub async fn load_cooling_load_temperature_explorer(
  recent_days: u32,
) -> Result<CoolingLoadTemperatureExplorer, sqlx::Error> {
  let pool = crate::infrastructure::database::db::get_pool().await?;
  load_cooling_load_temperature_explorer_from_pool(
    &pool,
    chrono::Local::now().date_naive(),
    recent_days,
  )
  .await
}

#[cfg(test)]
mod tests {
  use super::*;

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn hour_at(date: NaiveDate, hour: u32) -> NaiveDateTime {
    date.and_hms_opt(hour, 0, 0).unwrap()
  }

  fn summary(
    date: NaiveDate,
    hour: u32,
    usage: f32,
    temperature: f32,
  ) -> HourlyCoolingSummary {
    HourlyCoolingSummary {
      hour_start: hour_at(date, hour),
      cpu_usage_avg: Some(usage),
      cpu_temperature_avg: Some(temperature),
      sample_minutes: 60,
    }
  }

  fn established(start: NaiveDate, end: NaiveDate) -> BaselineState {
    BaselineState::Established {
      idle_temperature_avg: 35.0,
      window_start_date: start,
      window_end_date: end,
      sample_minutes: 210,
    }
  }

  fn established_result(
    result: CoolingLoadTemperatureExplorer,
  ) -> (ExplorerWindow, ExplorerWindow, Vec<BandMedianDelta>) {
    match result {
      CoolingLoadTemperatureExplorer::Established {
        baseline,
        recent,
        band_deltas,
      } => (baseline, recent, band_deltas),
      other => panic!("expected an established explorer, got {other:?}"),
    }
  }

  // ── clamp_recent_days ──

  #[test]
  fn a_recent_window_inside_the_range_is_used_as_requested() {
    assert_eq!(clamp_recent_days(28), 28);
    assert_eq!(clamp_recent_days(COOLING_EXPLORER_MIN_RECENT_DAYS), 7);
    assert_eq!(clamp_recent_days(COOLING_EXPLORER_MAX_RECENT_DAYS), 90);
  }

  #[test]
  fn an_undersized_recent_window_clamps_up_to_the_minimum() {
    assert_eq!(clamp_recent_days(0), COOLING_EXPLORER_MIN_RECENT_DAYS);
    assert_eq!(clamp_recent_days(1), COOLING_EXPLORER_MIN_RECENT_DAYS);
  }

  #[test]
  fn an_oversized_recent_window_clamps_down_instead_of_overflowing_the_calendar() {
    assert_eq!(
      clamp_recent_days(u32::MAX),
      COOLING_EXPLORER_MAX_RECENT_DAYS
    );
  }

  #[test]
  fn the_recent_window_start_follows_the_clamped_length() {
    let end = date(2026, 8, 20);
    let (_, recent, _) = established_result(derive_load_temperature_explorer(
      &[],
      established(date(2026, 1, 1), date(2026, 1, 7)),
      end,
      u32::MAX,
    ));

    assert_eq!(recent.end_date, end);
    assert_eq!(
      recent.start_date,
      end - Duration::days(COOLING_EXPLORER_MAX_RECENT_DAYS as i64 - 1)
    );
  }

  // ── lifecycle ──

  #[test]
  fn an_unestablished_baseline_reports_establishing_rather_than_a_partial_scatter() {
    let result = derive_load_temperature_explorer(
      &[summary(date(2026, 8, 19), 9, 5.0, 40.0)],
      BaselineState::Establishing {
        qualifying_days: 2,
        required_days: 7,
      },
      date(2026, 8, 20),
      28,
    );

    assert_eq!(
      result,
      CoolingLoadTemperatureExplorer::Establishing {
        qualifying_days: 2,
        required_days: 7,
      }
    );
  }

  // ── window partitioning ──

  #[test]
  fn each_hour_lands_in_the_windows_whose_dates_contain_it() {
    let baseline_start = date(2026, 8, 1);
    let baseline_end = date(2026, 8, 3);
    let recent_end = date(2026, 8, 20);
    let hours = [
      // Before the baseline window.
      summary(date(2026, 7, 31), 12, 5.0, 30.0),
      // Inside the baseline window, at both edges.
      summary(baseline_start, 0, 5.0, 31.0),
      summary(baseline_end, 23, 5.0, 32.0),
      // Between the two windows (a 7-day recent window starts 08-14).
      summary(date(2026, 8, 10), 12, 5.0, 33.0),
      // Inside the recent window, at both edges.
      summary(date(2026, 8, 14), 0, 5.0, 34.0),
      summary(recent_end, 23, 5.0, 35.0),
    ];

    let (baseline, recent, _) = established_result(derive_load_temperature_explorer(
      &hours,
      established(baseline_start, baseline_end),
      recent_end,
      7,
    ));

    assert_eq!(baseline.start_date, baseline_start);
    assert_eq!(baseline.end_date, baseline_end);
    assert_eq!(
      baseline
        .points
        .iter()
        .map(|p| p.cpu_temperature_avg)
        .collect::<Vec<_>>(),
      vec![31.0, 32.0]
    );
    assert_eq!(recent.start_date, date(2026, 8, 14));
    assert_eq!(
      recent
        .points
        .iter()
        .map(|p| p.cpu_temperature_avg)
        .collect::<Vec<_>>(),
      vec![34.0, 35.0]
    );
  }

  #[test]
  fn an_hour_missing_either_reading_is_not_scattered_at_an_invented_coordinate() {
    let day = date(2026, 8, 20);
    let hours = [
      summary(day, 9, 5.0, 40.0),
      HourlyCoolingSummary {
        hour_start: hour_at(day, 10),
        cpu_usage_avg: Some(80.0),
        cpu_temperature_avg: None,
        sample_minutes: 60,
      },
      HourlyCoolingSummary {
        hour_start: hour_at(day, 11),
        cpu_usage_avg: None,
        cpu_temperature_avg: Some(90.0),
        sample_minutes: 60,
      },
    ];

    let (_, recent, _) = established_result(derive_load_temperature_explorer(
      &hours,
      established(date(2026, 1, 1), date(2026, 1, 7)),
      day,
      7,
    ));

    assert_eq!(recent.points.len(), 1);
    assert_eq!(recent.points[0].cpu_usage_avg, 5.0);
    assert_eq!(recent.points[0].cpu_temperature_avg, 40.0);
  }

  // ── medians ──

  #[test]
  fn a_band_median_of_an_odd_number_of_points_is_the_middle_value() {
    let day = date(2026, 8, 20);
    // Deliberately out of temperature order: the median must sort, not
    // take whatever arrived in the middle.
    let hours = [
      summary(day, 9, 5.0, 50.0),
      summary(day, 10, 5.0, 30.0),
      summary(day, 11, 5.0, 40.0),
    ];

    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &hours,
      established(date(2026, 1, 1), date(2026, 1, 7)),
      day,
      7,
    ));

    let idle = deltas[0].recent;
    assert_eq!(idle.temperature_median, Some(40.0));
    assert_eq!(idle.point_count, 3);
    assert_eq!(idle.sample_minutes, 180);
  }

  #[test]
  fn a_band_median_of_an_even_number_of_points_averages_the_two_middle_values() {
    let day = date(2026, 8, 20);
    let hours = [
      summary(day, 9, 5.0, 30.0),
      summary(day, 10, 5.0, 40.0),
      summary(day, 11, 5.0, 44.0),
      summary(day, 12, 5.0, 60.0),
    ];

    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &hours,
      established(date(2026, 1, 1), date(2026, 1, 7)),
      day,
      7,
    ));

    assert_eq!(deltas[0].recent.temperature_median, Some(42.0));
  }

  #[test]
  fn a_median_is_unmoved_by_a_single_extreme_hour() {
    let day = date(2026, 8, 20);
    let hours = [
      summary(day, 9, 5.0, 30.0),
      summary(day, 10, 5.0, 31.0),
      // A one-off stress run must not drag the band's trend the way a
      // mean would.
      summary(day, 11, 5.0, 95.0),
    ];

    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &hours,
      established(date(2026, 1, 1), date(2026, 1, 7)),
      day,
      7,
    ));

    assert_eq!(deltas[0].recent.temperature_median, Some(31.0));
  }

  #[test]
  fn each_point_is_binned_by_the_same_load_bands_the_rest_of_cooling_insight_uses() {
    let day = date(2026, 8, 20);
    let hours = [
      summary(day, 0, 9.99, 30.0),
      summary(day, 1, 10.0, 40.0),
      summary(day, 2, 29.99, 41.0),
      summary(day, 3, 30.0, 50.0),
      summary(day, 4, 59.99, 51.0),
      summary(day, 5, 60.0, 70.0),
    ];

    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &hours,
      established(date(2026, 1, 1), date(2026, 1, 7)),
      day,
      7,
    ));

    let [idle, low, mid, high] = [0, 1, 2, 3].map(|index| deltas[index].recent);
    assert_eq!(idle.temperature_median, Some(30.0));
    assert_eq!(low.temperature_median, Some(40.5));
    assert_eq!(mid.temperature_median, Some(50.5));
    assert_eq!(high.temperature_median, Some(70.0));
  }

  #[test]
  fn a_band_no_hour_landed_in_has_no_median_rather_than_zero() {
    let day = date(2026, 8, 20);
    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &[summary(day, 9, 5.0, 30.0)],
      established(date(2026, 1, 1), date(2026, 1, 7)),
      day,
      7,
    ));

    for band_median in deltas[1..].iter().map(|delta| delta.recent) {
      assert_eq!(band_median.temperature_median, None);
      assert_eq!(band_median.point_count, 0);
      assert_eq!(band_median.sample_minutes, 0);
    }
  }

  // ── per-band deltas ──

  fn window_hours(
    start: NaiveDate,
    temperature: f32,
    sample_minutes: u32,
  ) -> HourlyCoolingSummary {
    HourlyCoolingSummary {
      hour_start: hour_at(start, 12),
      cpu_usage_avg: Some(5.0),
      cpu_temperature_avg: Some(temperature),
      sample_minutes,
    }
  }

  #[test]
  fn a_band_with_enough_evidence_on_both_sides_reports_its_median_delta() {
    let baseline_day = date(2026, 8, 1);
    let recent_day = date(2026, 8, 20);
    let hours = [
      window_hours(
        baseline_day,
        30.0,
        COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES,
      ),
      window_hours(
        recent_day,
        36.5,
        COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES,
      ),
    ];

    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &hours,
      established(baseline_day, baseline_day),
      recent_day,
      7,
    ));

    let idle = deltas[0];
    assert_eq!(idle.band, CpuLoadBand::Idle);
    assert!(
      idle.comparable,
      "exactly the minimum on both sides is enough"
    );
    assert_eq!(idle.baseline.temperature_median, Some(30.0));
    assert_eq!(idle.recent.temperature_median, Some(36.5));
    assert_eq!(idle.delta, Some(6.5));
  }

  #[test]
  fn a_band_one_minute_short_on_either_side_reports_no_delta() {
    let baseline_day = date(2026, 8, 1);
    let recent_day = date(2026, 8, 20);
    let short = COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES - 1;
    let hours = [
      window_hours(baseline_day, 30.0, short),
      window_hours(
        recent_day,
        36.5,
        COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES,
      ),
    ];

    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &hours,
      established(baseline_day, baseline_day),
      recent_day,
      7,
    ));

    let idle = deltas[0];
    assert!(!idle.comparable);
    assert_eq!(
      idle.delta, None,
      "an unusable delta must be absent, not a number the caller has to know to ignore"
    );
    // The medians themselves are still carried through: the scatter
    // renders them, only the delta claim is withheld.
    assert_eq!(idle.baseline.temperature_median, Some(30.0));
    assert_eq!(idle.recent.temperature_median, Some(36.5));
  }

  #[test]
  fn a_band_present_only_in_the_recent_window_is_not_comparable() {
    let baseline_day = date(2026, 8, 1);
    let recent_day = date(2026, 8, 20);
    let hours = [
      window_hours(baseline_day, 30.0, 60),
      HourlyCoolingSummary {
        hour_start: hour_at(recent_day, 12),
        cpu_usage_avg: Some(80.0),
        cpu_temperature_avg: Some(75.0),
        sample_minutes: 60,
      },
    ];

    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &hours,
      established(baseline_day, baseline_day),
      recent_day,
      7,
    ));

    let high = deltas[3];
    assert_eq!(high.band, CpuLoadBand::High);
    assert!(
      !high.comparable,
      "the baseline side has no high-band evidence"
    );
    assert_eq!(high.delta, None);
    assert_eq!(high.recent.temperature_median, Some(75.0));
  }

  #[test]
  fn every_band_is_reported_in_ascending_load_order() {
    let (_, _, deltas) = established_result(derive_load_temperature_explorer(
      &[],
      established(date(2026, 8, 1), date(2026, 8, 1)),
      date(2026, 8, 20),
      7,
    ));

    assert_eq!(
      deltas.iter().map(|d| d.band).collect::<Vec<_>>(),
      vec![
        CpuLoadBand::Idle,
        CpuLoadBand::Low,
        CpuLoadBand::Mid,
        CpuLoadBand::High,
      ]
    );
  }

  // ── pinned baseline (DB-backed) ──

  mod pinned_baseline {
    use super::*;
    use crate::persistence::cooling_hourly_rollup::format_hour_start;
    use sqlx::SqlitePool;

    async fn setup_tables(pool: &SqlitePool) {
      sqlx::query(
        "CREATE TABLE cooling_daily_summary (
          date TEXT PRIMARY KEY,
          idle_cpu_temperature_avg REAL,
          idle_cpu_temperature_max REAL,
          idle_cpu_temperature_min REAL,
          idle_sample_minutes INTEGER NOT NULL DEFAULT 0,
          low_cpu_temperature_avg REAL,
          low_cpu_temperature_max REAL,
          low_cpu_temperature_min REAL,
          low_sample_minutes INTEGER NOT NULL DEFAULT 0,
          mid_cpu_temperature_avg REAL,
          mid_cpu_temperature_max REAL,
          mid_cpu_temperature_min REAL,
          mid_sample_minutes INTEGER NOT NULL DEFAULT 0,
          high_cpu_temperature_avg REAL,
          high_cpu_temperature_max REAL,
          high_cpu_temperature_min REAL,
          high_sample_minutes INTEGER NOT NULL DEFAULT 0,
          coverage_minutes INTEGER NOT NULL
        )",
      )
      .execute(pool)
      .await
      .unwrap();
      sqlx::query(
        "CREATE TABLE cooling_baseline (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          window_start_date TEXT NOT NULL,
          window_end_date TEXT NOT NULL,
          idle_temperature_avg REAL NOT NULL,
          sample_minutes INTEGER NOT NULL,
          established_at TEXT NOT NULL
        )",
      )
      .execute(pool)
      .await
      .unwrap();
      sqlx::query(
        "CREATE TABLE cooling_hourly_summary (
          hour_start TEXT PRIMARY KEY,
          cpu_usage_avg REAL,
          cpu_temperature_avg REAL,
          sample_minutes INTEGER NOT NULL
        )",
      )
      .execute(pool)
      .await
      .unwrap();
    }

    async fn insert_establishing_days(pool: &SqlitePool, start: NaiveDate) {
      use crate::persistence::cooling_baseline::{
        COOLING_BASELINE_QUALIFYING_IDLE_MINUTES,
        COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      };
      for offset in 0..COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS {
        sqlx::query(
          "INSERT INTO cooling_daily_summary
             (date, idle_cpu_temperature_avg, idle_sample_minutes, coverage_minutes)
           VALUES ($1, 30.0, $2, 1440)",
        )
        .bind(
          (start + Duration::days(offset as i64))
            .format("%Y-%m-%d")
            .to_string(),
        )
        .bind(COOLING_BASELINE_QUALIFYING_IDLE_MINUTES as i64)
        .execute(pool)
        .await
        .unwrap();
      }
    }

    async fn insert_hour(pool: &SqlitePool, hour_start: NaiveDateTime, temperature: f32) {
      sqlx::query(
        "INSERT INTO cooling_hourly_summary
           (hour_start, cpu_usage_avg, cpu_temperature_avg, sample_minutes)
         VALUES ($1, 5.0, $2, 60)",
      )
      .bind(format_hour_start(hour_start))
      .bind(temperature)
      .execute(pool)
      .await
      .unwrap();
    }

    #[tokio::test]
    async fn an_establishing_baseline_short_circuits_before_reading_any_hours() {
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      setup_tables(&pool).await;

      let result =
        load_cooling_load_temperature_explorer_from_pool(&pool, date(2026, 8, 20), 28)
          .await
          .unwrap();

      assert!(matches!(
        result,
        CoolingLoadTemperatureExplorer::Establishing { .. }
      ));
    }

    #[tokio::test]
    async fn the_baseline_window_does_not_drift_when_its_source_rows_are_deleted() {
      // Same regression as the band comparison's loader: the Explorer's
      // baseline window must come from the pinned row, not from a
      // re-derivation over whatever daily rows still exist.
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      setup_tables(&pool).await;
      let start = date(2026, 8, 1);
      insert_establishing_days(&pool, start).await;
      insert_hour(&pool, hour_at(start, 12), 30.0).await;

      let established =
        load_cooling_load_temperature_explorer_from_pool(&pool, date(2026, 8, 20), 28)
          .await
          .unwrap();
      let (baseline, _, _) = established_result(established);
      assert_eq!(baseline.start_date, start);
      assert_eq!(baseline.points.len(), 1);

      sqlx::query("DELETE FROM cooling_daily_summary")
        .execute(&pool)
        .await
        .unwrap();
      insert_establishing_days(&pool, date(2027, 6, 1)).await;

      let after_cleanup =
        load_cooling_load_temperature_explorer_from_pool(&pool, date(2027, 6, 20), 28)
          .await
          .unwrap();
      let (baseline, _, _) = established_result(after_cleanup);

      assert_eq!(
        baseline.start_date, start,
        "the pinned baseline window must not drift when its source rows are deleted"
      );
    }

    #[tokio::test]
    async fn the_bounded_read_still_covers_a_baseline_window_far_before_the_recent_one() {
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      setup_tables(&pool).await;
      let baseline_start = date(2026, 1, 1);
      let today = date(2026, 8, 20);
      insert_establishing_days(&pool, baseline_start).await;
      insert_hour(&pool, hour_at(baseline_start, 12), 30.0).await;
      insert_hour(&pool, hour_at(date(2026, 8, 18), 12), 44.0).await;

      let result = load_cooling_load_temperature_explorer_from_pool(&pool, today, 7)
        .await
        .unwrap();
      let (baseline, recent, _) = established_result(result);

      assert_eq!(
        baseline.points.len(),
        1,
        "the far-back baseline hour must still be read"
      );
      assert_eq!(recent.points.len(), 1);
    }
  }
}
