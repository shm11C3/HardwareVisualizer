//! Cooling Thermal Delta daily rollup (#2045, made row-per-source by
//! #2062).
//!
//! Folds one completed local day's paired hardware/ambient minutes into
//! one `cooling_thermal_delta_daily_summary` row per ambient Sensor Source
//! Label: `ΔT = CPU package temperature − ambient temperature`, per
//! CPU-load band, plus how many archived minutes that source paired with
//! at all. It runs inside the existing single-pass catch-up in
//! [`crate::persistence::cooling_rollup`], the same way the fan rollup
//! does, because it answers a different question about the same completed
//! day whose boundary has already been resolved there.
//!
//! Row-per-source, like `AMBIENT_ARCHIVE` and like
//! [`crate::persistence::cooling_fan_rollup`], and for a reason the fan
//! rollup does not have: which sensor a ΔT was measured against *is* the
//! measurement. Three sensors in one room were observed reading about
//! 2 K apart - close to half the 5 K rise Cooling Insight reports as a
//! mild sustained rise - so averaging two placements into one per-day
//! number produces a ΔT no sensor observed, and pinning a baseline from it
//! bakes that mixture in for good. Keeping the source on the row lets the
//! baseline record which placement it was established from and refuse to
//! be compared against any other (see
//! [`crate::persistence::cooling_delta_baseline`]).
//!
//! The pairing that makes ΔT meaningful happens *before* this fold, at
//! the read boundary: each [`ThermalDeltaMinuteSample`] is one archived
//! minute joined to one source's reading for that same minute (see
//! `database::cooling_thermal_delta_daily_summary`), so the fold can only
//! ever subtract two readings describing the same minute and the same
//! sensor. Independently aggregated CPU and ambient summaries must never
//! be subtracted - the two archives do not share a sample set.
//!
//! A source with no paired minute that day has no row rather than a
//! zeroed one, and a machine with no ambient sensor produces no rows at
//! all (DP-02).

use chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeMap;

use crate::persistence::cooling_rollup::{BandSummary, CpuLoadBand, ReadingAccumulator};

/// One archived minute paired with one ambient source's reading for that
/// same minute, as the rollup reads it back.
///
/// `ambient_temperature` is not optional: a sample exists only where the
/// join found a reading, so an unpaired minute is represented by absence.
/// The CPU fields stay optional because the pairing is defined on the
/// minute, not on the CPU sensors - a minute whose ambient reading paired
/// with an archive row that had no usable CPU temperature still counts as
/// coverage for that source (see [`ThermalDeltaDailySummary::coverage_minutes`]).
///
/// `timestamp` and `cpu_power_avg` are not read by this fold. They ride
/// on the sample so the co-variate rollup
/// ([`crate::persistence::cooling_covariate_rollup`], #2068) can pair the
/// very same minutes with the fan archive and with package power: one
/// read, one sample set, so the ΔT it fits against can never disagree
/// with the ΔT summarized here.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalDeltaMinuteSample {
  /// The archived minute's own instant, as `DATA_ARCHIVE` stamped it.
  pub timestamp: DateTime<Utc>,
  pub source: String,
  pub ambient_temperature: f32,
  pub cpu_usage_avg: Option<f32>,
  pub cpu_temperature_avg: Option<f32>,
  pub cpu_temperature_max: Option<f32>,
  pub cpu_temperature_min: Option<f32>,
  /// The minute's CPU package power in watts, where the archive carried
  /// one; `None` is a minute without a power reading, never 0 W.
  pub cpu_power_avg: Option<f32>,
}

/// One `cooling_thermal_delta_daily_summary` row: one ambient source's
/// ΔT profile for one completed local day.
///
/// Each band's [`BandSummary`] holds ΔT in kelvin-equivalent degrees,
/// folded only over minutes that carried *both* a classifiable CPU
/// reading and this source's ambient reading, so a band's
/// `sample_minutes` is always a subset of the matching absolute
/// temperature band's on `cooling_daily_summary`.
///
/// `coverage_minutes` is counted outside that nesting: every archived
/// minute of the day this source paired with, whether or not the CPU side
/// could be classified. That makes it an honest measure of ambient
/// availability on a machine with no CPU temperature sensor, which is
/// exactly what the backfill cursor needs (a row exists iff
/// `coverage_minutes >= 1`, so such a machine's days still count as
/// summarized rather than being re-rolled forever).
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalDeltaDailySummary {
  pub date: NaiveDate,
  /// The ambient Sensor Source Label, as archived.
  pub source: String,
  pub coverage_minutes: u32,
  pub idle: BandSummary,
  pub low: BandSummary,
  pub mid: BandSummary,
  pub high: BandSummary,
}

impl ThermalDeltaDailySummary {
  /// This day's ΔT summary for `band`.
  pub fn band(&self, band: CpuLoadBand) -> &BandSummary {
    match band {
      CpuLoadBand::Idle => &self.idle,
      CpuLoadBand::Low => &self.low,
      CpuLoadBand::Mid => &self.mid,
      CpuLoadBand::High => &self.high,
    }
  }
}

/// Minutes in a local calendar day; the cap for
/// [`ThermalDeltaDailySummary::coverage_minutes`], for the same
/// shutdown-flush reason `cooling_rollup::MINUTES_PER_DAY` exists.
const MINUTES_PER_DAY: u32 = 24 * 60;

/// Fold one local day's paired minutes into one summary per ambient
/// source, ordered by source so the write order is deterministic.
///
/// A minute contributes to a ΔT band only when it carries a usable CPU
/// usage *and* temperature triple, matching `cooling_rollup::summarize_day`'s
/// own band gate; otherwise it contributes coverage and nothing else. The
/// minute's single ambient value shifts all three of avg/max/min
/// together, so the ΔT extremes are the CPU extremes offset by it -
/// nothing is interpolated across minutes.
pub fn summarize_thermal_delta_day(
  date: NaiveDate,
  minutes: &[ThermalDeltaMinuteSample],
) -> Vec<ThermalDeltaDailySummary> {
  let mut accumulators: BTreeMap<&str, SourceAccumulator> = BTreeMap::new();

  for minute in minutes {
    accumulators
      .entry(minute.source.as_str())
      .or_default()
      .push(minute);
  }

  accumulators
    .into_iter()
    .map(|(source, accumulator)| accumulator.finish(date, source))
    .collect()
}

#[derive(Default)]
struct SourceAccumulator {
  coverage_minutes: u32,
  idle: ReadingAccumulator,
  low: ReadingAccumulator,
  mid: ReadingAccumulator,
  high: ReadingAccumulator,
}

impl SourceAccumulator {
  fn push(&mut self, minute: &ThermalDeltaMinuteSample) {
    self.coverage_minutes += 1;

    let (Some(cpu_usage_avg), Some(avg), Some(max), Some(min)) = (
      minute.cpu_usage_avg,
      minute.cpu_temperature_avg,
      minute.cpu_temperature_max,
      minute.cpu_temperature_min,
    ) else {
      return;
    };

    let band = match CpuLoadBand::classify(cpu_usage_avg) {
      CpuLoadBand::Idle => &mut self.idle,
      CpuLoadBand::Low => &mut self.low,
      CpuLoadBand::Mid => &mut self.mid,
      CpuLoadBand::High => &mut self.high,
    };
    let ambient = minute.ambient_temperature;
    band.push(avg - ambient, max - ambient, min - ambient);
  }

  fn finish(self, date: NaiveDate, source: &str) -> ThermalDeltaDailySummary {
    ThermalDeltaDailySummary {
      date,
      source: source.to_string(),
      coverage_minutes: self.coverage_minutes.min(MINUTES_PER_DAY),
      idle: self.idle.finish_band(),
      low: self.low.finish_band(),
      mid: self.mid.finish_band(),
      high: self.high.finish_band(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()
  }

  /// A paired minute whose CPU extremes sit 1 K either side of `cpu`.
  fn paired(
    source: &str,
    cpu_usage: f32,
    cpu: f32,
    ambient: f32,
  ) -> ThermalDeltaMinuteSample {
    ThermalDeltaMinuteSample {
      timestamp: DateTime::<Utc>::from_timestamp(1_776_600_000, 0).unwrap(),
      source: source.to_string(),
      ambient_temperature: ambient,
      cpu_usage_avg: Some(cpu_usage),
      cpu_temperature_avg: Some(cpu),
      cpu_temperature_max: Some(cpu + 1.0),
      cpu_temperature_min: Some(cpu - 1.0),
      cpu_power_avg: None,
    }
  }

  #[test]
  fn each_ambient_source_gets_its_own_row() {
    let summaries = summarize_thermal_delta_day(
      date(),
      &[
        paired("Living Room", 5.0, 40.0, 25.0),
        paired("Desk", 5.0, 40.0, 28.0),
        paired("Living Room", 5.0, 50.0, 25.0),
      ],
    );

    assert_eq!(
      summaries
        .iter()
        .map(|summary| summary.source.as_str())
        .collect::<Vec<_>>(),
      vec!["Desk", "Living Room"]
    );
    assert_eq!(summaries[0].idle.avg, Some(12.0));
    assert_eq!(summaries[0].coverage_minutes, 1);
    assert_eq!(summaries[1].idle.avg, Some(20.0));
    assert_eq!(summaries[1].idle.sample_minutes, 2);
    assert_eq!(summaries[1].coverage_minutes, 2);
  }

  #[test]
  fn two_sources_sharing_a_minute_never_average_into_one_delta() {
    // The property #2062 exists for. Two placements 3 K apart read the
    // same CPU minute: each row carries its own sensor's ΔT, and the
    // 16.5 K a per-minute mean would have produced appears nowhere.
    let summaries = summarize_thermal_delta_day(
      date(),
      &[
        paired("Desk", 5.0, 40.0, 25.0),
        paired("Living Room", 5.0, 40.0, 22.0),
      ],
    );

    let deltas: Vec<_> = summaries
      .iter()
      .map(|summary| (summary.source.as_str(), summary.idle.avg))
      .collect();
    assert_eq!(
      deltas,
      vec![("Desk", Some(15.0)), ("Living Room", Some(18.0))]
    );
  }

  #[test]
  fn a_day_without_any_paired_minute_produces_no_rows() {
    assert_eq!(summarize_thermal_delta_day(date(), &[]), Vec::new());
  }

  #[test]
  fn a_delta_band_is_nested_inside_its_temperature_band() {
    // Two minutes in different load bands: the delta lands in the same
    // band the temperature did, never pooled across bands.
    let summaries = summarize_thermal_delta_day(
      date(),
      &[
        paired("Desk", 5.0, 40.0, 25.0),
        paired("Desk", 80.0, 70.0, 25.0),
      ],
    );

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].idle.avg, Some(15.0));
    assert_eq!(summaries[0].idle.sample_minutes, 1);
    assert_eq!(summaries[0].high.avg, Some(45.0));
    assert_eq!(summaries[0].high.sample_minutes, 1);
    assert_eq!(summaries[0].low, BandSummary::default());
    assert_eq!(summaries[0].mid, BandSummary::default());
  }

  #[test]
  fn a_minute_the_band_gate_rejects_contributes_coverage_but_no_delta() {
    // An ambient reading cannot let a minute with no usable CPU
    // temperature into a delta band - there is nothing to subtract it
    // from. It still paired, though, and coverage says so.
    let summaries = summarize_thermal_delta_day(
      date(),
      &[
        paired("Desk", 5.0, 40.0, 25.0),
        ThermalDeltaMinuteSample {
          cpu_temperature_avg: None,
          ..paired("Desk", 5.0, 40.0, 25.0)
        },
        ThermalDeltaMinuteSample {
          cpu_usage_avg: None,
          ..paired("Desk", 5.0, 40.0, 25.0)
        },
      ],
    );

    assert_eq!(summaries[0].idle.sample_minutes, 1);
    assert_eq!(summaries[0].idle.avg, Some(15.0));
    assert_eq!(
      summaries[0].coverage_minutes, 3,
      "ambient availability is a separate capability from CPU sensing"
    );
  }

  #[test]
  fn a_source_without_a_cpu_temperature_sensor_still_records_a_coverage_row() {
    // The machine the backfill cursor must not rewind on forever: an
    // ambient sensor, no CPU temperature sensor. No delta is possible,
    // but the row it leaves is what tells the cursor the day was done.
    let summaries = summarize_thermal_delta_day(
      date(),
      &[ThermalDeltaMinuteSample {
        cpu_temperature_avg: None,
        cpu_temperature_max: None,
        cpu_temperature_min: None,
        ..paired("Desk", 5.0, 40.0, 25.0)
      }],
    );

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].coverage_minutes, 1);
    for band in [
      CpuLoadBand::Idle,
      CpuLoadBand::Low,
      CpuLoadBand::Mid,
      CpuLoadBand::High,
    ] {
      assert_eq!(*summaries[0].band(band), BandSummary::default());
    }
  }

  #[test]
  fn a_delta_extreme_is_the_cpu_extreme_offset_by_that_minutes_ambient() {
    // Within one archived minute there is a single ambient reading, so
    // all three of avg/max/min shift together. Nothing is interpolated
    // between minutes.
    let summaries = summarize_thermal_delta_day(
      date(),
      &[
        paired("Desk", 5.0, 40.0, 25.0),
        paired("Desk", 5.0, 50.0, 20.0),
      ],
    );

    // Deltas are 15 and 30, so the average is 22.5.
    assert_eq!(summaries[0].idle.avg, Some(22.5));
    assert_eq!(summaries[0].idle.max, Some(31.0));
    assert_eq!(summaries[0].idle.min, Some(14.0));
    assert_eq!(summaries[0].idle.sample_minutes, 2);
  }

  #[test]
  fn a_negative_delta_is_kept_rather_than_clamped() {
    // A cold-boot minute, or a machine in a room warmer than its own
    // package sensor reads. The number is what it is; clamping would
    // fabricate a reading.
    let summaries =
      summarize_thermal_delta_day(date(), &[paired("Desk", 5.0, 20.0, 25.0)]);

    assert_eq!(summaries[0].idle.avg, Some(-5.0));
  }

  #[test]
  fn the_date_is_carried_onto_every_row() {
    let summaries = summarize_thermal_delta_day(
      date(),
      &[
        paired("Desk", 5.0, 40.0, 25.0),
        paired("Living Room", 5.0, 40.0, 25.0),
      ],
    );

    assert!(summaries.iter().all(|summary| summary.date == date()));
  }
}
