//! Cooling co-variate daily rollup (#2068).
//!
//! Folds one completed local day's paired minutes into what the "what
//! moved with it" view needs: for each ambient Sensor Source Label and
//! each CPU-load band, the sufficient statistics of a least-squares fit
//! of the Thermal Delta against CPU package power, the same six sums per
//! fan for the Thermal Delta against fan speed, and the day's medians of
//! package power, Thermal Delta, ambient temperature, fan speed and
//! load-band share. It runs inside the existing single-pass catch-up in
//! [`crate::persistence::cooling_rollup`], the same way the fan and ΔT
//! rollups do.
//!
//! **Pair first, aggregate second - per pair.** A minute is one tuple
//! `(ΔT, package power, fan rpm, load band)` and each pair inside it is
//! independent: the ΔT-power sums see only minutes carrying *both* a ΔT
//! and a power reading, the ΔT-rpm sums only minutes carrying both a ΔT
//! and that fan's reading, and a median sees every minute carrying its
//! own reading. Nothing is interpolated or zero-filled across minutes
//! (DP-02): a minute missing one member of a pair contributes to no fit
//! for that pair, and still contributes to whatever it does carry.
//!
//! **Sufficient statistics, not minutes.** Slope, intercept and Pearson r
//! are all functions of `n, Σx, Σy, Σxy, Σx², Σy²`, and those sums add
//! across days, so a window's fit is the fit over every paired minute in
//! it without storing a single minute past the archive's own retention.
//! The six sums are the whole fit; the medians beside them are the
//! per-day values the factor table compares.
//!
//! **Per ambient source, like the ΔT rollup it consumes.** The samples
//! are the [`ThermalDeltaMinuteSample`]s the ΔT rollup summarizes, so the
//! ΔT fitted here is the ΔT `cooling_thermal_delta_daily_summary` holds,
//! measured against the same sensor - which is what lets the query
//! boundary refuse to compare two placements, exactly as the
//! ambient-adjusted comparison already does. A sensor change can never
//! mix two placements into one fit because the source is on every row.

use chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeMap;

use crate::persistence::cooling_fan_rollup::FanArchiveMinuteSample;
use crate::persistence::cooling_rollup::CpuLoadBand;
use crate::persistence::cooling_thermal_delta_rollup::ThermalDeltaMinuteSample;

/// The sufficient statistics of a least-squares fit of `y` against `x`,
/// summed over paired minutes: `n, Σx, Σy, Σxy, Σx², Σy²`.
///
/// Sums add, so statistics of several days combine with [`Self::merge`]
/// into the statistics of the whole window - which is why the rollup
/// stores these rather than the minutes. Slope, intercept and Pearson r
/// are derived at query time
/// (`cooling_covariate_comparison::LeastSquaresFit`). `n == 0` is the
/// empty fit; every sum is then zero, and nothing reads them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PairedFitStatistics {
  pub n: u32,
  pub sum_x: f64,
  pub sum_y: f64,
  pub sum_xy: f64,
  pub sum_xx: f64,
  pub sum_yy: f64,
}

impl PairedFitStatistics {
  /// Fold one paired observation in.
  pub fn push(&mut self, x: f64, y: f64) {
    self.n += 1;
    self.sum_x += x;
    self.sum_y += y;
    self.sum_xy += x * y;
    self.sum_xx += x * x;
    self.sum_yy += y * y;
  }

  /// Fold another set of sums in - the statistics of two disjoint sample
  /// sets are the statistics of their union.
  pub fn merge(&mut self, other: &Self) {
    self.n += other.n;
    self.sum_x += other.sum_x;
    self.sum_y += other.sum_y;
    self.sum_xy += other.sum_xy;
    self.sum_xx += other.sum_xx;
    self.sum_yy += other.sum_yy;
  }
}

/// One `cooling_covariate_daily_summary` row: one ambient source's
/// co-variates for one CPU-load band on one completed local day.
///
/// A row exists only for a `(date, source, band)` that saw at least one
/// paired minute with a classifiable CPU usage (`sample_minutes >= 1`).
/// Within it each reading is independent: `delta_minutes` counts the
/// minutes that also carried a CPU temperature, `power_minutes` those
/// that also carried package power, and the ΔT-power sums cover only the
/// minutes that carried both. A median is `None` exactly when its count
/// is zero - never 0 K or 0 W.
#[derive(Debug, Clone, PartialEq)]
pub struct CovariateDailySummary {
  pub date: NaiveDate,
  /// The ambient Sensor Source Label, as archived.
  pub source: String,
  pub band: CpuLoadBand,
  /// Paired minutes with a classifiable CPU usage that fell in this
  /// band.
  pub sample_minutes: u32,
  /// This band's share of the source's paired, classifiable minutes that
  /// day, in `0.0..=1.0`.
  pub band_share: f32,
  /// Median ambient temperature over the band's paired minutes. Every
  /// paired minute carries one, so a row always has it.
  pub ambient_temperature_median: f32,
  /// Minutes that also carried a CPU temperature - the Thermal Delta
  /// paired-minute count the factor table reports.
  pub delta_minutes: u32,
  pub delta_temperature_median: Option<f32>,
  /// Minutes that also carried CPU package power.
  pub power_minutes: u32,
  pub cpu_power_median: Option<f32>,
  /// ΔT (y, kelvin-equivalent degrees) against package power (x, watts)
  /// over the minutes that carried both.
  pub delta_per_watt: PairedFitStatistics,
}

/// One `cooling_fan_covariate_daily_summary` row: one fan's speed beside
/// one ambient source's Thermal Delta for one band on one day.
///
/// A row exists only for a fan that reported during at least one of the
/// band's paired minutes (`rpm_minutes >= 1`). The fit covers only the
/// minutes that also carried a ΔT.
#[derive(Debug, Clone, PartialEq)]
pub struct FanCovariateDailySummary {
  pub date: NaiveDate,
  /// The ambient Sensor Source Label, as archived.
  pub source: String,
  /// The fan's stable channel-derived identifier, as archived.
  pub fan_source: String,
  pub band: CpuLoadBand,
  pub rpm_minutes: u32,
  pub rpm_median: f32,
  /// ΔT (y) against fan speed (x, rpm) over the minutes that carried
  /// both.
  pub delta_per_rpm: PairedFitStatistics,
}

/// One completed day's co-variate rows, both shapes, ordered by
/// `(source, band[, fan_source])` so the write order is deterministic.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CovariateDaySummary {
  pub bands: Vec<CovariateDailySummary>,
  pub fans: Vec<FanCovariateDailySummary>,
}

/// Fold one local day's paired minutes and the same day's archived fan
/// readings into co-variate rows.
///
/// `paired` is the ΔT rollup's own read: one archived minute beside one
/// source's ambient reading for that same minute, so the ΔT computed
/// here is the ΔT that rollup summarizes. Fan readings are paired to a
/// minute by the same minute key the SQL pairing uses; a fan that
/// reported more than once inside a minute contributes that minute's
/// mean, mirroring the per-`(minute, source)` collapse on the ambient
/// side. A fan reading with no paired minute to sit beside contributes
/// nothing - there is no ΔT for it to be paired with.
pub fn summarize_covariate_day(
  date: NaiveDate,
  paired: &[ThermalDeltaMinuteSample],
  fans: &[FanArchiveMinuteSample],
) -> CovariateDaySummary {
  let fan_readings = fan_readings_by_minute(fans);
  let mut sources: BTreeMap<&str, SourceAccumulator> = BTreeMap::new();

  for minute in paired {
    // The band is the minute's own; without a usage reading there is no
    // band to file the minute under, so it contributes to nothing here.
    let Some(cpu_usage_avg) = minute.cpu_usage_avg else {
      continue;
    };
    let band = CpuLoadBand::classify(cpu_usage_avg);
    let fans_this_minute = fan_readings.get(&minute_key(minute.timestamp));
    sources.entry(minute.source.as_str()).or_default().push(
      band,
      minute,
      fans_this_minute,
    );
  }

  let mut summary = CovariateDaySummary::default();
  for (source, accumulator) in sources {
    accumulator.finish(date, source, &mut summary);
  }
  summary
}

/// The archive minute an instant falls in: the same
/// `epoch_milliseconds / 60000` key the SQL pairing joins on, so a fan
/// reading pairs with exactly the hardware minute the ambient reading
/// did.
fn minute_key(instant: DateTime<Utc>) -> i64 {
  instant.timestamp_millis() / 60_000
}

/// Per minute, each fan's mean rpm over the readings it made inside that
/// minute.
fn fan_readings_by_minute(
  fans: &[FanArchiveMinuteSample],
) -> BTreeMap<i64, BTreeMap<&str, f64>> {
  let mut sums: BTreeMap<i64, BTreeMap<&str, (f64, u32)>> = BTreeMap::new();
  for reading in fans {
    let (sum, count) = sums
      .entry(minute_key(reading.timestamp))
      .or_default()
      .entry(reading.source.as_str())
      .or_default();
    *sum += reading.rpm as f64;
    *count += 1;
  }

  sums
    .into_iter()
    .map(|(minute, by_fan)| {
      (
        minute,
        by_fan
          .into_iter()
          .map(|(fan, (sum, count))| (fan, sum / count as f64))
          .collect(),
      )
    })
    .collect()
}

#[derive(Default)]
struct SourceAccumulator {
  classified_minutes: u32,
  bands: BTreeMap<CpuLoadBand, BandAccumulator>,
}

impl SourceAccumulator {
  fn push(
    &mut self,
    band: CpuLoadBand,
    minute: &ThermalDeltaMinuteSample,
    fans: Option<&BTreeMap<&str, f64>>,
  ) {
    self.classified_minutes += 1;
    self.bands.entry(band).or_default().push(minute, fans);
  }

  fn finish(self, date: NaiveDate, source: &str, into: &mut CovariateDaySummary) {
    for (band, accumulator) in self.bands {
      accumulator.finish(date, source, band, self.classified_minutes, into);
    }
  }
}

#[derive(Default)]
struct BandAccumulator {
  ambient: Vec<f32>,
  delta: Vec<f32>,
  power: Vec<f32>,
  delta_per_watt: PairedFitStatistics,
  fans: BTreeMap<String, FanAccumulator>,
}

#[derive(Default)]
struct FanAccumulator {
  rpm: Vec<f32>,
  delta_per_rpm: PairedFitStatistics,
}

impl BandAccumulator {
  fn push(
    &mut self,
    minute: &ThermalDeltaMinuteSample,
    fans: Option<&BTreeMap<&str, f64>>,
  ) {
    let ambient = minute.ambient_temperature;
    self.ambient.push(ambient);

    // Each pair on its own: a minute keeps every reading it has and is
    // paired only where both members are present.
    let delta = minute.cpu_temperature_avg.map(|cpu| cpu - ambient);
    if let Some(delta) = delta {
      self.delta.push(delta);
    }
    if let Some(power) = minute.cpu_power_avg {
      self.power.push(power);
      if let Some(delta) = delta {
        self.delta_per_watt.push(power as f64, delta as f64);
      }
    }
    for (fan, rpm) in fans.into_iter().flatten() {
      let accumulator = self.fans.entry((*fan).to_string()).or_default();
      accumulator.rpm.push(*rpm as f32);
      if let Some(delta) = delta {
        accumulator.delta_per_rpm.push(*rpm, delta as f64);
      }
    }
  }

  fn finish(
    self,
    date: NaiveDate,
    source: &str,
    band: CpuLoadBand,
    classified_minutes: u32,
    into: &mut CovariateDaySummary,
  ) {
    let sample_minutes = self.ambient.len() as u32;
    // `classified_minutes` counts this band's minutes among others, so it
    // is at least `sample_minutes >= 1` and the share is well defined.
    let band_share = sample_minutes as f32 / classified_minutes as f32;
    into.bands.push(CovariateDailySummary {
      date,
      source: source.to_string(),
      band,
      sample_minutes,
      band_share,
      ambient_temperature_median: median(&self.ambient)
        .expect("a band accumulator only exists once a minute was pushed"),
      delta_minutes: self.delta.len() as u32,
      delta_temperature_median: median(&self.delta),
      power_minutes: self.power.len() as u32,
      cpu_power_median: median(&self.power),
      delta_per_watt: self.delta_per_watt,
    });
    for (fan_source, fan) in self.fans {
      into.fans.push(FanCovariateDailySummary {
        date,
        source: source.to_string(),
        fan_source,
        band,
        rpm_minutes: fan.rpm.len() as u32,
        rpm_median: median(&fan.rpm)
          .expect("a fan accumulator only exists once a reading was pushed"),
        delta_per_rpm: fan.delta_per_rpm,
      });
    }
  }
}

/// The median of `values`, or `None` for an empty slice. An even count
/// averages the two middle values.
///
/// An exact median rather than a streaming estimate: the input is at
/// most one day of one source's minutes, so sorting it is cheap, and an
/// approximation would be a number no minute observed.
pub(crate) fn median(values: &[f32]) -> Option<f32> {
  if values.is_empty() {
    return None;
  }
  let mut sorted = values.to_vec();
  sorted.sort_by(f32::total_cmp);
  let middle = sorted.len() / 2;
  Some(if sorted.len().is_multiple_of(2) {
    (sorted[middle - 1] + sorted[middle]) / 2.0
  } else {
    sorted[middle]
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()
  }

  /// The instant of archive minute `minute` of the test day.
  fn at(minute: u32) -> DateTime<Utc> {
    date().and_hms_opt(12, 0, 0).unwrap().and_utc()
      + chrono::Duration::minutes(minute as i64)
  }

  /// A paired minute carrying every reading: usage, CPU temperature,
  /// ambient and package power.
  fn paired(
    minute: u32,
    source: &str,
    cpu_usage: f32,
    cpu: f32,
    ambient: f32,
    power: Option<f32>,
  ) -> ThermalDeltaMinuteSample {
    ThermalDeltaMinuteSample {
      timestamp: at(minute),
      source: source.to_string(),
      ambient_temperature: ambient,
      cpu_usage_avg: Some(cpu_usage),
      cpu_temperature_avg: Some(cpu),
      cpu_temperature_max: Some(cpu + 1.0),
      cpu_temperature_min: Some(cpu - 1.0),
      cpu_power_avg: power,
    }
  }

  fn fan(minute: u32, fan_source: &str, rpm: u32) -> FanArchiveMinuteSample {
    FanArchiveMinuteSample {
      timestamp: at(minute),
      source: fan_source.to_string(),
      rpm,
    }
  }

  fn only_band(summary: &CovariateDaySummary) -> &CovariateDailySummary {
    assert_eq!(summary.bands.len(), 1, "expected exactly one band row");
    &summary.bands[0]
  }

  // ── pairing per minute ──

  #[test]
  fn a_minute_missing_one_member_of_a_pair_contributes_to_no_fit_for_that_pair() {
    // Minute 0 carries everything; minute 1 has no power; minute 2 has no
    // CPU temperature. The ΔT-power fit sees minute 0 alone, the power
    // median sees minutes 0 and 2, the ΔT median minutes 0 and 1.
    let summary = summarize_covariate_day(
      date(),
      &[
        paired(0, "Desk", 5.0, 40.0, 25.0, Some(10.0)),
        paired(1, "Desk", 5.0, 42.0, 25.0, None),
        ThermalDeltaMinuteSample {
          cpu_temperature_avg: None,
          ..paired(2, "Desk", 5.0, 40.0, 25.0, Some(30.0))
        },
      ],
      &[],
    );

    let row = only_band(&summary);
    assert_eq!(row.sample_minutes, 3);
    assert_eq!(row.delta_per_watt.n, 1);
    assert_eq!(row.delta_per_watt.sum_x, 10.0);
    assert_eq!(row.delta_per_watt.sum_y, 15.0);
    assert_eq!(row.delta_minutes, 2);
    assert_eq!(row.delta_temperature_median, Some(16.0));
    assert_eq!(row.power_minutes, 2);
    assert_eq!(row.cpu_power_median, Some(20.0));
  }

  #[test]
  fn a_fan_reading_pairs_only_with_the_minute_it_was_stamped_in() {
    // Fan 1 reported in minutes 0 and 1; minute 1 has no ΔT. The ΔT-rpm
    // fit sees minute 0 alone, while the rpm median sees both readings.
    // A reading in minute 5, which no paired minute covers, is nowhere.
    let summary = summarize_covariate_day(
      date(),
      &[
        paired(0, "Desk", 5.0, 40.0, 25.0, None),
        ThermalDeltaMinuteSample {
          cpu_temperature_avg: None,
          ..paired(1, "Desk", 5.0, 40.0, 25.0, None)
        },
      ],
      &[
        fan(0, "Fan 1", 900),
        fan(1, "Fan 1", 1100),
        fan(5, "Fan 1", 5000),
      ],
    );

    assert_eq!(summary.fans.len(), 1);
    let row = &summary.fans[0];
    assert_eq!(row.fan_source, "Fan 1");
    assert_eq!(row.rpm_minutes, 2);
    assert_eq!(row.rpm_median, 1000.0);
    assert_eq!(row.delta_per_rpm.n, 1);
    assert_eq!(row.delta_per_rpm.sum_x, 900.0);
    assert_eq!(row.delta_per_rpm.sum_y, 15.0);
  }

  #[test]
  fn a_fan_that_reported_twice_in_one_minute_contributes_that_minutes_mean() {
    // The same collapse the SQL pairing applies per (minute, source):
    // a shutdown flush inside a minute must not double-weight it.
    let summary = summarize_covariate_day(
      date(),
      &[paired(0, "Desk", 5.0, 40.0, 25.0, None)],
      &[fan(0, "Fan 1", 800), fan(0, "Fan 1", 1200)],
    );

    assert_eq!(summary.fans[0].rpm_minutes, 1);
    assert_eq!(summary.fans[0].rpm_median, 1000.0);
    assert_eq!(summary.fans[0].delta_per_rpm.n, 1);
    assert_eq!(summary.fans[0].delta_per_rpm.sum_x, 1000.0);
  }

  #[test]
  fn each_fan_gets_its_own_row_beside_the_same_delta() {
    let summary = summarize_covariate_day(
      date(),
      &[paired(0, "Desk", 5.0, 40.0, 25.0, None)],
      &[fan(0, "Fan 2", 1500), fan(0, "Fan 1", 900)],
    );

    assert_eq!(
      summary
        .fans
        .iter()
        .map(|row| (row.fan_source.as_str(), row.rpm_median))
        .collect::<Vec<_>>(),
      vec![("Fan 1", 900.0), ("Fan 2", 1500.0)]
    );
  }

  // ── row-per-source ──

  #[test]
  fn two_ambient_sources_on_one_day_produce_two_rows_and_two_fits() {
    // Two placements 3 K apart read the same two minutes. Each row fits
    // its own sensor's ΔT against the same power; the intercepts differ
    // by exactly the placement offset, and no row blends the two.
    let summary = summarize_covariate_day(
      date(),
      &[
        paired(0, "Desk", 5.0, 40.0, 25.0, Some(10.0)),
        paired(0, "Living Room", 5.0, 40.0, 22.0, Some(10.0)),
        paired(1, "Desk", 5.0, 50.0, 25.0, Some(20.0)),
        paired(1, "Living Room", 5.0, 50.0, 22.0, Some(20.0)),
      ],
      &[],
    );

    assert_eq!(summary.bands.len(), 2);
    let desk = &summary.bands[0];
    let living_room = &summary.bands[1];
    assert_eq!(desk.source, "Desk");
    assert_eq!(living_room.source, "Living Room");
    assert_eq!(desk.delta_per_watt.n, 2);
    assert_eq!(living_room.delta_per_watt.n, 2);
    // Σy: Desk 15 + 25 = 40; Living Room 18 + 28 = 46.
    assert_eq!(desk.delta_per_watt.sum_y, 40.0);
    assert_eq!(living_room.delta_per_watt.sum_y, 46.0);
    assert_eq!(desk.delta_temperature_median, Some(20.0));
    assert_eq!(living_room.delta_temperature_median, Some(23.0));
  }

  #[test]
  fn a_fan_is_paired_separately_beside_each_ambient_source() {
    let summary = summarize_covariate_day(
      date(),
      &[
        paired(0, "Desk", 5.0, 40.0, 25.0, None),
        paired(0, "Living Room", 5.0, 40.0, 22.0, None),
      ],
      &[fan(0, "Fan 1", 900)],
    );

    assert_eq!(
      summary
        .fans
        .iter()
        .map(|row| (row.source.as_str(), row.delta_per_rpm.sum_y))
        .collect::<Vec<_>>(),
      vec![("Desk", 15.0), ("Living Room", 18.0)]
    );
  }

  // ── bands ──

  #[test]
  fn minutes_land_in_their_own_load_band_with_the_days_share() {
    // Three idle minutes and one high minute: two rows, shares 0.75 and
    // 0.25, each fit over its own band's minutes only.
    let summary = summarize_covariate_day(
      date(),
      &[
        paired(0, "Desk", 5.0, 40.0, 25.0, Some(10.0)),
        paired(1, "Desk", 5.0, 41.0, 25.0, Some(11.0)),
        paired(2, "Desk", 5.0, 42.0, 25.0, Some(12.0)),
        paired(3, "Desk", 90.0, 80.0, 25.0, Some(60.0)),
      ],
      &[],
    );

    assert_eq!(
      summary
        .bands
        .iter()
        .map(|row| (
          row.band,
          row.sample_minutes,
          row.band_share,
          row.delta_per_watt.n
        ))
        .collect::<Vec<_>>(),
      vec![
        (CpuLoadBand::Idle, 3, 0.75, 3),
        (CpuLoadBand::High, 1, 0.25, 1)
      ]
    );
  }

  #[test]
  fn the_band_share_is_per_source_rather_than_per_day() {
    // A sensor that paired only the idle half of the day reports the
    // idle band as its whole day, because those are the minutes it
    // observed; the other sensor's share says what it saw.
    let summary = summarize_covariate_day(
      date(),
      &[
        paired(0, "Desk", 5.0, 40.0, 25.0, None),
        paired(0, "Living Room", 5.0, 40.0, 22.0, None),
        paired(1, "Living Room", 90.0, 80.0, 22.0, None),
      ],
      &[],
    );

    assert_eq!(
      summary
        .bands
        .iter()
        .map(|row| (row.source.as_str(), row.band, row.band_share))
        .collect::<Vec<_>>(),
      vec![
        ("Desk", CpuLoadBand::Idle, 1.0),
        ("Living Room", CpuLoadBand::Idle, 0.5),
        ("Living Room", CpuLoadBand::High, 0.5),
      ]
    );
  }

  #[test]
  fn a_minute_without_a_usage_reading_has_no_band_and_contributes_nothing() {
    let summary = summarize_covariate_day(
      date(),
      &[ThermalDeltaMinuteSample {
        cpu_usage_avg: None,
        ..paired(0, "Desk", 5.0, 40.0, 25.0, Some(10.0))
      }],
      &[fan(0, "Fan 1", 900)],
    );

    assert_eq!(summary, CovariateDaySummary::default());
  }

  #[test]
  fn a_day_without_any_paired_minute_produces_no_rows() {
    assert_eq!(
      summarize_covariate_day(date(), &[], &[fan(0, "Fan 1", 900)]),
      CovariateDaySummary::default()
    );
  }

  // ── absent, never zero ──

  #[test]
  fn a_machine_without_a_power_source_leaves_power_absent_and_the_fit_empty() {
    let summary =
      summarize_covariate_day(date(), &[paired(0, "Desk", 5.0, 40.0, 25.0, None)], &[]);

    let row = only_band(&summary);
    assert_eq!(row.power_minutes, 0);
    assert_eq!(row.cpu_power_median, None);
    assert_eq!(row.delta_per_watt, PairedFitStatistics::default());
    assert_eq!(row.delta_temperature_median, Some(15.0));
  }

  #[test]
  fn a_source_without_a_cpu_temperature_sensor_still_records_its_other_readings() {
    // No ΔT is possible, but power and ambient were observed and the row
    // is what tells the catch-up cursor the day was done.
    let summary = summarize_covariate_day(
      date(),
      &[ThermalDeltaMinuteSample {
        cpu_temperature_avg: None,
        cpu_temperature_max: None,
        cpu_temperature_min: None,
        ..paired(0, "Desk", 5.0, 40.0, 25.0, Some(10.0))
      }],
      &[],
    );

    let row = only_band(&summary);
    assert_eq!(row.sample_minutes, 1);
    assert_eq!(row.delta_minutes, 0);
    assert_eq!(row.delta_temperature_median, None);
    assert_eq!(row.cpu_power_median, Some(10.0));
    assert_eq!(row.ambient_temperature_median, 25.0);
    assert_eq!(row.delta_per_watt.n, 0);
  }

  // ── the sums ──

  #[test]
  fn the_fit_statistics_are_the_plain_sums_over_the_paired_minutes() {
    // x = 10, 20, 30 W; y = 15, 25, 35 K.
    let summary = summarize_covariate_day(
      date(),
      &[
        paired(0, "Desk", 5.0, 40.0, 25.0, Some(10.0)),
        paired(1, "Desk", 5.0, 50.0, 25.0, Some(20.0)),
        paired(2, "Desk", 5.0, 60.0, 25.0, Some(30.0)),
      ],
      &[],
    );

    assert_eq!(
      only_band(&summary).delta_per_watt,
      PairedFitStatistics {
        n: 3,
        sum_x: 60.0,
        sum_y: 75.0,
        sum_xy: 150.0 + 500.0 + 1050.0,
        sum_xx: 100.0 + 400.0 + 900.0,
        sum_yy: 225.0 + 625.0 + 1225.0,
      }
    );
  }

  #[test]
  fn merged_statistics_are_the_statistics_of_the_union() {
    let mut a = PairedFitStatistics::default();
    a.push(1.0, 2.0);
    let mut b = PairedFitStatistics::default();
    b.push(3.0, 5.0);
    let mut whole = PairedFitStatistics::default();
    whole.push(1.0, 2.0);
    whole.push(3.0, 5.0);

    a.merge(&b);

    assert_eq!(a, whole);
  }

  // ── median ──

  #[test]
  fn median_of_an_even_count_averages_the_middle_pair() {
    assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
  }

  #[test]
  fn median_of_an_odd_count_is_the_middle_value() {
    assert_eq!(median(&[9.0, 1.0, 5.0]), Some(5.0));
  }

  #[test]
  fn median_of_nothing_is_absent() {
    assert_eq!(median(&[]), None);
  }

  #[test]
  fn the_date_is_carried_onto_every_row() {
    let summary = summarize_covariate_day(
      date(),
      &[paired(0, "Desk", 5.0, 40.0, 25.0, Some(10.0))],
      &[fan(0, "Fan 1", 900)],
    );

    assert!(summary.bands.iter().all(|row| row.date == date()));
    assert!(summary.fans.iter().all(|row| row.date == date()));
  }
}
