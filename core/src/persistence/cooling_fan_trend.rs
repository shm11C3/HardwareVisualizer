//! Cooling Insight long-range fan trend: per-day fan-speed summaries from
//! the fan daily rollup (`cooling_fan_daily_summary`), for the 90-day and
//! 1-year Cooling Insight windows (#2022).
//!
//! A sibling of [`crate::persistence::cooling_trend`] rather than an
//! extension of it: the CPU trend answers one row per day, while the fan
//! trend answers one series per fan, because how many fans a machine
//! exposes is configuration-dependent. Joining them would force the
//! CPU-side query to carry a shape it has no use for.
//!
//! Periods of 30 days and below read `FAN_ARCHIVE` directly through
//! `archive_queries::select_fan_archive_series`, matching how the other
//! lanes route those windows.
//!
//! A fan-day the rollup has no row for is simply absent: the caller renders
//! that as a gap, never as 0 RPM.

use chrono::{Duration, NaiveDate};

use crate::persistence::cooling_fan_rollup::FanDailySummary;
use crate::persistence::cooling_trend::trend_window_start_date;

/// One fan's daily series over the requested window, oldest day first.
#[derive(Debug, Clone, PartialEq)]
pub struct FanTrendSeries {
  /// The fan's stable channel-derived identifier, as archived.
  pub source: String,
  pub days: Vec<FanDailySummary>,
}

/// Group summarized fan-days into one series per fan, keeping the input's
/// day order within each series and ordering the series by source so the
/// lane's colors and legend stay stable across refreshes.
pub fn group_fan_days_by_source(days: &[FanDailySummary]) -> Vec<FanTrendSeries> {
  let mut series: Vec<FanTrendSeries> = Vec::new();

  for day in days {
    match series.iter_mut().find(|entry| entry.source == day.source) {
      Some(entry) => entry.days.push(day.clone()),
      None => series.push(FanTrendSeries {
        source: day.source.clone(),
        days: vec![day.clone()],
      }),
    }
  }

  series.sort_by(|a, b| a.source.cmp(&b.source));
  series
}

/// The subset of `days` whose date falls within `[start, end]` inclusive,
/// preserving input order. Days the rollup never produced a row for stay
/// absent - this never synthesizes an entry to close a gap.
pub fn fan_days_in_window(
  days: &[FanDailySummary],
  start: NaiveDate,
  end: NaiveDate,
) -> Vec<FanDailySummary> {
  days
    .iter()
    .filter(|day| day.date >= start && day.date <= end)
    .cloned()
    .collect()
}

/// The trailing `days`-day fan trend, ending yesterday (the most recent
/// local day the rollup can have summarized).
pub async fn load_cooling_fan_trend(
  days: u32,
) -> Result<Vec<FanTrendSeries>, sqlx::Error> {
  use crate::infrastructure::database;

  let all_days =
    database::cooling_fan_daily_summary::select_all_fan_daily_summaries().await?;
  let yesterday = chrono::Local::now().date_naive() - Duration::days(1);
  let start = trend_window_start_date(days, yesterday);

  Ok(group_fan_days_by_source(&fan_days_in_window(
    &all_days, start, yesterday,
  )))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn day(d: NaiveDate, source: &str, avg: f32) -> FanDailySummary {
    FanDailySummary {
      date: d,
      source: source.to_string(),
      rpm_avg: avg,
      rpm_max: avg as u32,
      rpm_min: avg as u32,
      sample_minutes: 600,
    }
  }

  #[test]
  fn each_fan_becomes_its_own_series_ordered_by_source() {
    let series = group_fan_days_by_source(&[
      day(date(2026, 8, 10), "Fan 2", 1500.0),
      day(date(2026, 8, 10), "Fan 1", 900.0),
      day(date(2026, 8, 11), "Fan 1", 1000.0),
    ]);

    assert_eq!(
      series
        .iter()
        .map(|entry| entry.source.as_str())
        .collect::<Vec<_>>(),
      vec!["Fan 1", "Fan 2"]
    );
    assert_eq!(
      series[0]
        .days
        .iter()
        .map(|entry| entry.date)
        .collect::<Vec<_>>(),
      vec![date(2026, 8, 10), date(2026, 8, 11)]
    );
    assert_eq!(series[1].days.len(), 1);
  }

  #[test]
  fn a_fan_that_only_ran_part_of_the_window_keeps_the_days_it_has() {
    // The absent days stay absent instead of being filled with 0 RPM;
    // the lane draws the gap.
    let series = group_fan_days_by_source(&[
      day(date(2026, 8, 10), "Fan 1", 900.0),
      day(date(2026, 8, 12), "Fan 1", 950.0),
    ]);

    assert_eq!(
      series[0]
        .days
        .iter()
        .map(|entry| entry.date)
        .collect::<Vec<_>>(),
      vec![date(2026, 8, 10), date(2026, 8, 12)]
    );
  }

  #[test]
  fn a_machine_with_no_fan_rows_yields_no_series() {
    assert_eq!(group_fan_days_by_source(&[]), Vec::new());
  }

  #[test]
  fn days_outside_the_window_are_excluded() {
    let start = date(2026, 8, 1);
    let end = date(2026, 8, 10);
    let days = vec![
      day(start - Duration::days(1), "Fan 1", 900.0),
      day(start, "Fan 1", 900.0),
      day(end, "Fan 1", 900.0),
      day(end + Duration::days(1), "Fan 1", 900.0),
    ];

    assert_eq!(
      fan_days_in_window(&days, start, end)
        .iter()
        .map(|entry| entry.date)
        .collect::<Vec<_>>(),
      vec![start, end]
    );
  }

  #[test]
  fn an_empty_rollup_yields_an_empty_window() {
    assert_eq!(
      fan_days_in_window(&[], date(2026, 8, 1), date(2026, 8, 10)),
      Vec::new()
    );
  }
}
