//! Cooling Insight long-range trend: per-day CPU temperature summaries
//! from the daily rollup (`cooling_daily_summary`), for the 90-day and
//! 1-year Cooling Insight windows (#2017).
//!
//! Periods of 30 days and below keep using the existing bucketed
//! `archive_queries` (see `get_data_archive_series`) - this module exists
//! only for the longer windows the one-minute Hardware Archive cannot
//! reach without extending `hardwareArchive.retentionDays`.
//!
//! A day the rollup has no row for is simply absent from the returned
//! list: the caller renders that as a gap, never as a zeroed day (see
//! `crate::persistence::cooling_rollup::summarize_day`, which is why a
//! dayless row can never exist in the table to begin with).

use chrono::{Duration, NaiveDate};

use crate::persistence::cooling_rollup::{
  COOLING_DAILY_SUMMARY_RETENTION_DAYS, DailyCoolingSummary,
};

/// Upper bound for a requested trend window, matching the rollup's own
/// retention: no window can show more days than the table can hold.
/// Clamping here also keeps the `NaiveDate` arithmetic below safely
/// inside chrono's supported range for any `u32` the IPC boundary lets
/// through, instead of panicking on an oversized request.
pub const COOLING_TREND_MAX_DAYS: u32 = COOLING_DAILY_SUMMARY_RETENTION_DAYS;

/// First local day of the trailing `days`-day window ending at (and
/// including) `window_end_date`. `days` is clamped to
/// `1..=`[`COOLING_TREND_MAX_DAYS`]: `days == 0` degenerates to a
/// single-day window at `window_end_date`, matching `days == 1`, rather
/// than producing a start date after the end date, and an oversized
/// request degenerates to the widest window the rollup can back.
pub fn trend_window_start_date(days: u32, window_end_date: NaiveDate) -> NaiveDate {
  let days = days.clamp(1, COOLING_TREND_MAX_DAYS);
  window_end_date - Duration::days((days - 1) as i64)
}

/// The subset of `days` whose date falls within `[start, end]`
/// (inclusive), preserving input order. Days the rollup never produced a
/// row for are simply not present in `days` and stay absent from the
/// result - this never fills a gap with a synthesized entry.
pub fn days_in_window(
  days: &[DailyCoolingSummary],
  start: NaiveDate,
  end: NaiveDate,
) -> Vec<DailyCoolingSummary> {
  days
    .iter()
    .filter(|day| day.date >= start && day.date <= end)
    .cloned()
    .collect()
}

/// [`days_in_window`] over the trailing `days`-day window ending
/// yesterday (the most recent local day the rollup can have summarized).
pub async fn load_cooling_trend(
  days: u32,
) -> Result<Vec<DailyCoolingSummary>, sqlx::Error> {
  use crate::infrastructure::database;

  let all_days =
    database::cooling_daily_summary::select_all_daily_cooling_summaries().await?;
  let yesterday = chrono::Local::now().date_naive() - Duration::days(1);
  let start = trend_window_start_date(days, yesterday);

  Ok(days_in_window(&all_days, start, yesterday))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::persistence::cooling_rollup::{
    AmbientDeltaSummary, BandSummary, PowerSummary,
  };

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn empty_band() -> BandSummary {
    BandSummary::default()
  }

  fn day(date: NaiveDate, coverage_minutes: u32) -> DailyCoolingSummary {
    DailyCoolingSummary {
      date,
      coverage_minutes,
      idle: empty_band(),
      low: empty_band(),
      mid: empty_band(),
      high: empty_band(),
      power: PowerSummary::default(),
      ambient: AmbientDeltaSummary::default(),
    }
  }

  // ── trend_window_start_date ──

  #[test]
  fn a_ninety_day_window_starts_eighty_nine_days_before_the_end() {
    let end = date(2026, 8, 29);
    assert_eq!(trend_window_start_date(90, end), end - Duration::days(89));
  }

  #[test]
  fn a_one_year_window_starts_three_hundred_sixty_four_days_before_the_end() {
    let end = date(2026, 8, 29);
    assert_eq!(trend_window_start_date(365, end), end - Duration::days(364));
  }

  #[test]
  fn a_one_day_window_starts_and_ends_on_the_same_day() {
    let end = date(2026, 8, 29);
    assert_eq!(trend_window_start_date(1, end), end);
  }

  #[test]
  fn an_oversized_request_clamps_to_the_retention_window_instead_of_panicking() {
    let end = date(2026, 8, 29);
    // u32::MAX days would push the NaiveDate arithmetic outside chrono's
    // supported range; the clamp degrades it to the widest window the
    // rollup can back.
    assert_eq!(
      trend_window_start_date(u32::MAX, end),
      trend_window_start_date(COOLING_TREND_MAX_DAYS, end)
    );
  }

  #[test]
  fn a_zero_day_window_does_not_start_after_the_end() {
    let end = date(2026, 8, 29);
    assert_eq!(trend_window_start_date(0, end), end);
  }

  // ── days_in_window ──

  #[test]
  fn days_outside_the_window_are_excluded() {
    let start = date(2026, 8, 1);
    let end = date(2026, 8, 10);
    let days = vec![
      day(start - Duration::days(1), 100),
      day(start, 100),
      day(end, 100),
      day(end + Duration::days(1), 100),
    ];

    let result = days_in_window(&days, start, end);

    assert_eq!(
      result.iter().map(|d| d.date).collect::<Vec<_>>(),
      vec![start, end]
    );
  }

  #[test]
  fn a_gap_in_the_rollup_stays_absent_rather_than_being_filled() {
    let start = date(2026, 8, 1);
    let end = date(2026, 8, 5);
    // Day 3 was never rolled up (e.g. the app was not running).
    let days = vec![day(date(2026, 8, 1), 100), day(date(2026, 8, 5), 100)];

    let result = days_in_window(&days, start, end);

    assert_eq!(result.len(), 2, "only the two present days should appear");
    assert_eq!(
      result.iter().map(|d| d.date).collect::<Vec<_>>(),
      vec![date(2026, 8, 1), date(2026, 8, 5)]
    );
  }

  #[test]
  fn an_empty_rollup_yields_an_empty_trend() {
    assert_eq!(
      days_in_window(&[], date(2026, 8, 1), date(2026, 8, 10)),
      Vec::new()
    );
  }
}
