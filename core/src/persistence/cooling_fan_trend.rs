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

/// The long-range fan trend, plus the one fact the caller cannot derive
/// from the series alone.
///
/// An empty `series` has two very different causes, and only one of them
/// licenses telling the user the machine has no readable fan:
/// - the machine really has none, or
/// - the rollup has not summarized one yet. It only ever summarizes
///   *completed* days, so a machine that started recording fans today -
///   or one whose fan tables the migration only just created - has a full
///   CPU trend beside an empty fan trend for up to a day.
///
/// `archive_has_readings` separates the two: the one-minute fan archive
/// holds a reading the moment collection starts, long before the rollup
/// can act on it.
#[derive(Debug, Clone, PartialEq)]
pub struct CoolingFanTrend {
  pub series: Vec<FanTrendSeries>,
  pub archive_has_readings: bool,
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
///
/// The archive probe rides along on the same call rather than being a
/// second command: the caller needs both answers to decide one thing (see
/// [`CoolingFanTrend`]), and splitting them would let a view render from
/// one without the other.
pub async fn load_cooling_fan_trend(days: u32) -> Result<CoolingFanTrend, sqlx::Error> {
  use crate::infrastructure::database;

  let all_days =
    database::cooling_fan_daily_summary::select_all_fan_daily_summaries().await?;
  let yesterday = chrono::Local::now().date_naive() - Duration::days(1);
  let start = trend_window_start_date(days, yesterday);

  Ok(CoolingFanTrend {
    series: group_fan_days_by_source(&fan_days_in_window(&all_days, start, yesterday)),
    archive_has_readings: database::fan_archive::has_any_reading().await?,
  })
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

  // ── the post-upgrade / first-day distinction ──

  #[tokio::test]
  async fn an_empty_trend_still_reports_the_archive_holding_readings() {
    // The state right after the migration lands, and again on the first
    // day of collection: the rollup has nothing (it only summarizes
    // completed days) while the archive already does. Reporting this as a
    // plain empty trend is what made the view claim "not supported".
    use crate::infrastructure::database::cooling_fan_daily_summary::tests::setup_cooling_fan_daily_summary;
    use crate::infrastructure::database::fan_archive::tests::setup_fan_archive;
    use sqlx::SqlitePool;

    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    setup_cooling_fan_daily_summary(&pool).await;
    crate::infrastructure::database::fan_archive::insert_from_pool(
      &pool,
      vec![crate::persistence::archive_data::FanArchiveRow {
        source: "Fan 1".to_string(),
        rpm: 900,
      }],
      chrono::Utc::now(),
    )
    .await
    .unwrap();

    let summarized =
      crate::infrastructure::database::cooling_fan_daily_summary::select_all_fan_daily_summaries_from_pool(&pool)
        .await
        .unwrap();
    let archive_has_readings =
      crate::infrastructure::database::fan_archive::has_any_reading_from_pool(&pool)
        .await
        .unwrap();

    assert_eq!(
      group_fan_days_by_source(&summarized),
      Vec::new(),
      "the rollup has not summarized a completed day yet"
    );
    assert!(
      archive_has_readings,
      "but the archive already proves the fan is readable"
    );
  }

  #[tokio::test]
  async fn a_machine_without_a_readable_fan_reports_neither_side() {
    use crate::infrastructure::database::fan_archive::tests::setup_fan_archive;
    use sqlx::SqlitePool;

    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;

    assert!(
      !crate::infrastructure::database::fan_archive::has_any_reading_from_pool(&pool)
        .await
        .unwrap(),
      "with neither side holding anything, absent is the honest answer"
    );
  }
}
