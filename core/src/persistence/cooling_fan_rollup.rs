//! Cooling fan daily rollup (#2022).
//!
//! Folds the one-minute `FAN_ARCHIVE` rows of one completed local day into
//! one `cooling_fan_daily_summary` row per fan, so Cooling Insight's fan
//! lane can reach the 90-day and 1-year windows the one-minute archive
//! cannot hold. It runs inside the existing single-pass catch-up in
//! [`crate::persistence::cooling_rollup`] rather than as a second worker:
//! both projections answer different questions about the same completed
//! day, and the day's boundary has already been resolved there.
//!
//! Row-per-fan, like the archive it derives from: how many fans a machine
//! exposes is configuration-dependent. A fan with no archived reading that
//! day is absent from the result rather than present with zeroes, and a
//! machine with no fan source produces no rows at all.
//!
//! There is deliberately no hourly fan projection. The Explorer compares
//! CPU load against CPU temperature and has no fan axis, so an hourly fan
//! table would be collection cost with no visible value.

use chrono::NaiveDate;
use std::collections::BTreeMap;

/// One archived fan reading, as the rollup reads it back.
///
/// Every `FAN_ARCHIVE` row carries a real observation - the archive writer
/// only ever writes readings it could actually take - so `rpm` is not
/// optional here. An unreadable fan is represented by the absence of rows,
/// never by a zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanArchiveMinuteSample {
  pub source: String,
  pub rpm: u32,
}

/// One `cooling_fan_daily_summary` row: one fan's profile for one
/// completed local day.
///
/// Not an `Option`-shaped summary like
/// [`crate::persistence::cooling_rollup::PowerSummary`]: a fan with no
/// contributing minute has no row at all, so every row that exists carries
/// real numbers. `sample_minutes` is therefore always at least 1.
#[derive(Debug, Clone, PartialEq)]
pub struct FanDailySummary {
  pub date: NaiveDate,
  /// The fan's stable channel-derived identifier, as archived.
  pub source: String,
  pub rpm_avg: f32,
  pub rpm_max: u32,
  pub rpm_min: u32,
  pub sample_minutes: u32,
}

/// Fold one local day's archived fan readings into one summary per fan,
/// ordered by source so the write order is deterministic.
///
/// `rpm_avg` averages the per-minute averages, consistent with how the CPU
/// rollup folds `DATA_ARCHIVE`; `rpm_max`/`rpm_min` are the extremes across
/// those per-minute values. A day with no archived fan reading produces an
/// empty result rather than zeroed rows for the fans the machine used to
/// have.
pub fn summarize_fan_day(
  date: NaiveDate,
  minutes: &[FanArchiveMinuteSample],
) -> Vec<FanDailySummary> {
  let mut accumulators: BTreeMap<&str, FanAccumulator> = BTreeMap::new();

  for minute in minutes {
    accumulators
      .entry(minute.source.as_str())
      .or_default()
      .push(minute.rpm);
  }

  accumulators
    .into_iter()
    .filter_map(|(source, accumulator)| accumulator.finish(date, source))
    .collect()
}

#[derive(Default)]
struct FanAccumulator {
  sum: u64,
  count: u32,
  max: Option<u32>,
  min: Option<u32>,
}

impl FanAccumulator {
  fn push(&mut self, rpm: u32) {
    self.sum += rpm as u64;
    self.count += 1;
    self.max = Some(self.max.map_or(rpm, |current| current.max(rpm)));
    self.min = Some(self.min.map_or(rpm, |current| current.min(rpm)));
  }

  fn finish(self, date: NaiveDate, source: &str) -> Option<FanDailySummary> {
    let (max, min) = (self.max?, self.min?);
    Some(FanDailySummary {
      date,
      source: source.to_string(),
      rpm_avg: (self.sum as f64 / self.count as f64) as f32,
      rpm_max: max,
      rpm_min: min,
      sample_minutes: self.count,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()
  }

  fn sample(source: &str, rpm: u32) -> FanArchiveMinuteSample {
    FanArchiveMinuteSample {
      source: source.to_string(),
      rpm,
    }
  }

  #[test]
  fn each_fan_gets_its_own_row() {
    let summaries = summarize_fan_day(
      date(),
      &[
        sample("Fan 1", 900),
        sample("Fan 2", 1500),
        sample("Fan 1", 1100),
      ],
    );

    assert_eq!(
      summaries
        .iter()
        .map(|summary| summary.source.as_str())
        .collect::<Vec<_>>(),
      vec!["Fan 1", "Fan 2"]
    );
    assert_eq!(summaries[0].rpm_avg, 1000.0);
    assert_eq!(summaries[0].rpm_max, 1100);
    assert_eq!(summaries[0].rpm_min, 900);
    assert_eq!(summaries[0].sample_minutes, 2);
  }

  #[test]
  fn a_day_of_inactive_fan_readings_summarizes_as_a_real_zero() {
    // 0 RPM is an observation the fan is not reporting rotation, so the
    // day must summarize to zero rather than dropping the fan entirely.
    let summaries = summarize_fan_day(date(), &[sample("Fan 3", 0), sample("Fan 3", 0)]);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].rpm_avg, 0.0);
    assert_eq!(summaries[0].rpm_max, 0);
    assert_eq!(summaries[0].rpm_min, 0);
    assert_eq!(summaries[0].sample_minutes, 2);
  }

  #[test]
  fn a_day_without_any_archived_fan_reading_produces_no_rows() {
    assert_eq!(summarize_fan_day(date(), &[]), Vec::new());
  }

  #[test]
  fn a_fan_that_only_reported_part_of_the_day_counts_only_its_own_minutes() {
    // The lane must not imply a fan was recorded all day because another
    // fan was: `sample_minutes` is per fan, never the day's coverage.
    let summaries = summarize_fan_day(
      date(),
      &[
        sample("Fan 1", 800),
        sample("Fan 1", 800),
        sample("Fan 1", 800),
        sample("Fan 2", 1200),
      ],
    );

    assert_eq!(summaries[0].sample_minutes, 3);
    assert_eq!(summaries[1].sample_minutes, 1);
  }

  #[test]
  fn the_date_is_carried_onto_every_row() {
    let summaries = summarize_fan_day(date(), &[sample("Fan 1", 900)]);

    assert_eq!(summaries[0].date, date());
  }
}
