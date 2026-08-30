//! Cooling hourly rollup (#2023).
//!
//! A second, finer projection of the same one-minute Hardware Archive rows
//! the daily rollup reads (see [`crate::persistence::cooling_rollup`]):
//! one row per local wall-clock hour carrying that hour's average CPU
//! usage and average CPU temperature. This is what the load-vs-temperature
//! Explorer scatters - the daily rollup deliberately stores no intra-day
//! usage series, so it cannot answer "how hot was the CPU at this load".
//!
//! Derived inside the daily rollup's own catch-up cycle from the rows it
//! already fetched, so this adds no worker and no extra query (see
//! `cooling_rollup::roll_up_day`), and it shares the daily rollup's
//! retention contract
//! ([`crate::persistence::cooling_rollup::COOLING_DAILY_SUMMARY_RETENTION_DAYS`]),
//! which bounds the table at ~9,600 narrow rows.
//!
//! Only a minute carrying *both* a CPU usage and a CPU temperature reading
//! contributes: a scatter point is a pair, and averaging the two sides over
//! different sets of minutes would plot a coordinate neither reading
//! actually observed. An hour with no such minute produces no row at all
//! rather than a zeroed one (DP-02), which is why a persisted row always
//! carries both averages even though the columns are nullable.
//!
//! "Carrying a temperature reading" means the same thing here as in
//! `summarize_day`: all three of avg/max/min present. Only the averages
//! are stored, but holding the two folds to one condition is what lets
//! `cooling_rollup::rollup_catch_up_cursor` infer from the daily table
//! whether this rollup has fallen behind - see [`summarize_hours`].

use std::collections::BTreeMap;

use chrono::{NaiveDateTime, TimeZone, Timelike};

use crate::persistence::cooling_rollup::ArchiveMinuteSample;

/// Storage format of `cooling_hourly_summary.hour_start`: a local
/// wall-clock hour.
///
/// Deliberately a `"%Y-%m-%d"`-prefixed extension of
/// `cooling_daily_summary.date`'s format, so the same two properties hold:
/// it sorts lexicographically exactly as it does chronologically, and a
/// plain `hour_start < "2025-07-25"` date-string comparison is a correct
/// cutoff (every hour of a day sorts after that day's bare date string).
/// Both the range query and the retention delete depend on that.
const HOUR_START_FORMAT: &str = "%Y-%m-%d %H:00";

/// The format [`parse_hour_start`] reads. `%M` rather than a literal `00`
/// so a value is decoded by the same rules chrono would use to produce it.
const HOUR_START_PARSE_FORMAT: &str = "%Y-%m-%d %H:%M";

pub fn format_hour_start(hour_start: NaiveDateTime) -> String {
  hour_start.format(HOUR_START_FORMAT).to_string()
}

/// Decode a stored `hour_start`. `None` for a value that is not in
/// [`HOUR_START_FORMAT`] - a hand-edited database must degrade to "this
/// row is unreadable" rather than panic a query.
pub fn parse_hour_start(raw: &str) -> Option<NaiveDateTime> {
  NaiveDateTime::parse_from_str(raw, HOUR_START_PARSE_FORMAT).ok()
}

/// One `cooling_hourly_summary` row: a single local wall-clock hour's
/// paired CPU load and CPU temperature.
///
/// `sample_minutes` counts the archived minutes that carried both
/// readings, and both averages are taken over exactly those minutes. A
/// row only exists when `sample_minutes > 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HourlyCoolingSummary {
  pub hour_start: NaiveDateTime,
  pub cpu_usage_avg: Option<f32>,
  pub cpu_temperature_avg: Option<f32>,
  pub sample_minutes: u32,
}

/// Fold archived one-minute rows into per-local-hour summaries, ordered by
/// hour ascending.
///
/// `zone` resolves each row's UTC timestamp to a local wall-clock hour,
/// matching the local-calendar framing the daily rollup already uses.
/// Hours with no paired minute are absent from the result, never zeroed.
pub fn summarize_hours<Tz: TimeZone>(
  minutes: &[ArchiveMinuteSample],
  zone: &Tz,
) -> Vec<HourlyCoolingSummary> {
  let mut hours: BTreeMap<NaiveDateTime, HourAccumulator> = BTreeMap::new();

  for minute in minutes {
    // Exactly `summarize_day`'s band condition, field for field: a
    // minute counts only when its CPU usage *and* all three CPU
    // temperature readings are present.
    //
    // Only the two averages are stored, so requiring max/min looks
    // stricter than this projection needs - but the equality with the
    // daily fold is load-bearing. `rollup_catch_up_cursor` decides
    // whether the hourly rollup is behind from the daily table's band
    // sample counts, which is sound only while "the day accrued band
    // samples" and "the day produced hourly rows" are the same
    // condition. A looser rule here would let an avg-only minute create
    // an hourly row on a day the daily fold recorded no band samples,
    // silently breaking that equivalence.
    //
    // No real data is lost: the collector writes the three temperature
    // fields together, so a minute never carries avg without max/min.
    let (
      Some(cpu_usage_avg),
      Some(cpu_temperature_avg),
      Some(_temperature_max),
      Some(_temperature_min),
    ) = (
      minute.cpu_usage_avg,
      minute.cpu_temperature_avg,
      minute.cpu_temperature_max,
      minute.cpu_temperature_min,
    )
    else {
      continue;
    };

    let Some(hour_start) = local_hour_start(minute, zone) else {
      continue;
    };
    hours
      .entry(hour_start)
      .or_default()
      .push(cpu_usage_avg, cpu_temperature_avg);
  }

  hours
    .into_iter()
    .map(|(hour_start, accumulator)| accumulator.finish(hour_start))
    .collect()
}

/// The local wall-clock hour `minute` falls in. `None` only if the
/// truncation to the hour is not a representable local time, which cannot
/// happen for an hour chrono just produced - handled rather than
/// `expect`ed so a persistence worker never panics on a clock edge case.
fn local_hour_start<Tz: TimeZone>(
  minute: &ArchiveMinuteSample,
  zone: &Tz,
) -> Option<NaiveDateTime> {
  let local = minute.timestamp.with_timezone(zone).naive_local();
  local.date().and_hms_opt(local.time().hour(), 0, 0)
}

/// Accumulates one hour's paired readings. Both averages are means of the
/// per-minute averages, consistent with how the daily rollup folds the
/// same rows (each archived row is itself already a one-minute average).
#[derive(Default)]
struct HourAccumulator {
  usage_sum: f64,
  temperature_sum: f64,
  count: u32,
}

impl HourAccumulator {
  fn push(&mut self, cpu_usage_avg: f32, cpu_temperature_avg: f32) {
    self.usage_sum += cpu_usage_avg as f64;
    self.temperature_sum += cpu_temperature_avg as f64;
    self.count += 1;
  }

  fn finish(self, hour_start: NaiveDateTime) -> HourlyCoolingSummary {
    let count = self.count as f64;
    HourlyCoolingSummary {
      hour_start,
      cpu_usage_avg: (self.count > 0).then(|| (self.usage_sum / count) as f32),
      cpu_temperature_avg: (self.count > 0)
        .then(|| (self.temperature_sum / count) as f32),
      sample_minutes: self.count,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::{DateTime, Utc};

  fn utc(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
      .unwrap()
      .with_timezone(&Utc)
  }

  fn jst() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(9 * 3600).unwrap()
  }

  fn naive(input: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M").unwrap()
  }

  fn minute(
    timestamp: &str,
    cpu_usage_avg: Option<f32>,
    cpu_temperature_avg: Option<f32>,
  ) -> ArchiveMinuteSample {
    ArchiveMinuteSample {
      timestamp: utc(timestamp),
      cpu_usage_avg,
      cpu_temperature_avg,
      cpu_temperature_max: cpu_temperature_avg,
      cpu_temperature_min: cpu_temperature_avg,
    }
  }

  // ── hour_start format ──

  #[test]
  fn an_hour_start_formats_as_a_zero_padded_local_wall_clock_hour() {
    assert_eq!(
      format_hour_start(naive("2026-08-15 09:00")),
      "2026-08-15 09:00"
    );
    assert_eq!(
      format_hour_start(naive("2026-08-15 23:00")),
      "2026-08-15 23:00"
    );
  }

  #[test]
  fn a_formatted_hour_start_round_trips_through_parse() {
    let hour = naive("2026-08-15 13:00");
    assert_eq!(parse_hour_start(&format_hour_start(hour)), Some(hour));
  }

  #[test]
  fn an_unreadable_hour_start_parses_to_none_rather_than_panicking() {
    assert_eq!(parse_hour_start("not a timestamp"), None);
    assert_eq!(parse_hour_start("2026-08-15"), None);
  }

  #[test]
  fn an_hour_start_sorts_lexicographically_as_it_does_chronologically() {
    // The bounded range query and the retention delete both rely on this.
    let mut keys = [
      format_hour_start(naive("2026-08-15 09:00")),
      format_hour_start(naive("2026-08-14 23:00")),
      format_hour_start(naive("2026-08-15 10:00")),
    ];
    keys.sort();

    assert_eq!(
      keys,
      [
        "2026-08-14 23:00".to_string(),
        "2026-08-15 09:00".to_string(),
        "2026-08-15 10:00".to_string(),
      ]
    );
    assert!(
      format_hour_start(naive("2026-08-15 00:00")).as_str() > "2026-08-15",
      "every hour of a day must sort after that day's bare date string, or a date-string retention cutoff would delete the whole day"
    );
  }

  // ── summarize_hours ──

  #[test]
  fn no_minutes_produce_no_hours() {
    assert_eq!(summarize_hours(&[], &Utc), Vec::new());
  }

  #[test]
  fn minutes_in_the_same_hour_average_into_one_point() {
    let minutes = [
      minute("2026-08-15T09:00:00Z", Some(10.0), Some(40.0)),
      minute("2026-08-15T09:30:00Z", Some(30.0), Some(50.0)),
      minute("2026-08-15T09:59:00Z", Some(20.0), Some(60.0)),
    ];

    let hours = summarize_hours(&minutes, &Utc);

    assert_eq!(hours.len(), 1);
    assert_eq!(hours[0].hour_start, naive("2026-08-15 09:00"));
    assert_eq!(hours[0].cpu_usage_avg, Some(20.0));
    assert_eq!(hours[0].cpu_temperature_avg, Some(50.0));
    assert_eq!(hours[0].sample_minutes, 3);
  }

  #[test]
  fn hours_are_returned_in_ascending_order_regardless_of_input_order() {
    let minutes = [
      minute("2026-08-15T11:00:00Z", Some(10.0), Some(40.0)),
      minute("2026-08-15T09:00:00Z", Some(10.0), Some(40.0)),
      minute("2026-08-15T10:00:00Z", Some(10.0), Some(40.0)),
    ];

    let hours = summarize_hours(&minutes, &Utc);

    assert_eq!(
      hours.iter().map(|h| h.hour_start).collect::<Vec<_>>(),
      vec![
        naive("2026-08-15 09:00"),
        naive("2026-08-15 10:00"),
        naive("2026-08-15 11:00"),
      ]
    );
  }

  #[test]
  fn an_hour_boundary_splits_two_points() {
    let minutes = [
      minute("2026-08-15T09:59:59Z", Some(10.0), Some(40.0)),
      minute("2026-08-15T10:00:00Z", Some(80.0), Some(70.0)),
    ];

    let hours = summarize_hours(&minutes, &Utc);

    assert_eq!(hours.len(), 2);
    assert_eq!(hours[0].hour_start, naive("2026-08-15 09:00"));
    assert_eq!(hours[0].cpu_usage_avg, Some(10.0));
    assert_eq!(hours[1].hour_start, naive("2026-08-15 10:00"));
    assert_eq!(hours[1].cpu_usage_avg, Some(80.0));
  }

  #[test]
  fn a_minute_without_a_temperature_reading_contributes_nothing() {
    let minutes = [
      minute("2026-08-15T09:00:00Z", Some(10.0), Some(40.0)),
      // Usage was recorded, temperature was not: this cannot form a
      // (load, temperature) pair, so it must not shift the load average.
      minute("2026-08-15T09:30:00Z", Some(90.0), None),
    ];

    let hours = summarize_hours(&minutes, &Utc);

    assert_eq!(hours.len(), 1);
    assert_eq!(hours[0].cpu_usage_avg, Some(10.0));
    assert_eq!(hours[0].sample_minutes, 1);
  }

  #[test]
  fn a_minute_without_a_usage_reading_contributes_nothing() {
    let minutes = [
      minute("2026-08-15T09:00:00Z", Some(10.0), Some(40.0)),
      minute("2026-08-15T09:30:00Z", None, Some(90.0)),
    ];

    let hours = summarize_hours(&minutes, &Utc);

    assert_eq!(hours.len(), 1);
    assert_eq!(hours[0].cpu_temperature_avg, Some(40.0));
    assert_eq!(hours[0].sample_minutes, 1);
  }

  #[test]
  fn a_minute_with_an_average_but_no_temperature_extremes_contributes_nothing() {
    // Regression: this fold used to accept a minute on `avg` alone while
    // `summarize_day` required avg/max/min, so such a minute produced an
    // hourly row on a day whose daily bands stayed empty. That broke the
    // equivalence `rollup_catch_up_cursor` relies on, letting the hourly
    // rollup be skipped. Both folds must reject it identically.
    let minutes = [ArchiveMinuteSample {
      timestamp: utc("2026-08-15T09:00:00Z"),
      cpu_usage_avg: Some(5.0),
      cpu_temperature_avg: Some(40.0),
      cpu_temperature_max: None,
      cpu_temperature_min: None,
    }];

    assert_eq!(summarize_hours(&minutes, &Utc), Vec::new());

    let day = crate::persistence::cooling_rollup::summarize_day(
      utc("2026-08-15T09:00:00Z").date_naive(),
      &minutes,
    )
    .expect("the day was recorded, so it still has a summary row");
    let band_sample_minutes: u32 = [day.idle, day.low, day.mid, day.high]
      .iter()
      .map(|band| band.sample_minutes)
      .sum();
    assert_eq!(
      band_sample_minutes, 0,
      "the daily fold rejects this minute, so the hourly fold must too"
    );
  }

  #[test]
  fn an_hour_with_no_paired_minute_stays_absent_rather_than_becoming_zero() {
    let minutes = [
      minute("2026-08-15T09:00:00Z", Some(10.0), None),
      minute("2026-08-15T09:30:00Z", None, Some(40.0)),
    ];

    assert_eq!(
      summarize_hours(&minutes, &Utc),
      Vec::new(),
      "an hour the machine recorded but never paired must not become a 0%/0degC point"
    );
  }

  #[test]
  fn hours_are_bucketed_by_local_wall_clock_time_not_utc() {
    // 15:00Z is JST midnight, so the two rows below fall in *different*
    // local days as well as different local hours.
    let minutes = [
      minute("2026-08-14T14:59:00Z", Some(10.0), Some(40.0)),
      minute("2026-08-14T15:00:00Z", Some(80.0), Some(70.0)),
    ];

    let hours = summarize_hours(&minutes, &jst());

    assert_eq!(
      hours.iter().map(|h| h.hour_start).collect::<Vec<_>>(),
      vec![naive("2026-08-14 23:00"), naive("2026-08-15 00:00")]
    );
  }
}
