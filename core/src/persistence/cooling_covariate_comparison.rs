//! Cooling co-variate comparison (#2068): for the baseline and recent
//! windows the observation strip already compares, which archived
//! co-variates moved with the Thermal Delta and which stayed within
//! range - and the ΔT-per-watt fit of each window, so a ΔT change can be
//! read at matched package power.
//!
//! Everything here is an observation about two windows of one ambient
//! source's rows. A factor that moved is reported as having moved, and
//! nothing more: the view this feeds says what moved together, and the
//! observation strip's checklist remains the only place that suggests
//! what to look at.
//!
//! The windows and the gate are the ambient-adjusted comparison's own
//! ([`crate::persistence::cooling_band_comparison`]): the baseline side
//! is the Thermal Delta Baseline's pinned window, read from the source it
//! was established from; the recent side is the trailing
//! [`COOLING_BASELINE_RECENT_WINDOW_DAYS`], read from whichever source
//! covered most of it; and the two are compared only when they are the
//! same source and both carry enough paired minutes. A recent window
//! from a different sensor is reported but never judged - the difference
//! between two placements is not a factor that moved.

use std::collections::BTreeSet;

use chrono::{Duration, NaiveDate};

use crate::persistence::cooling_band_comparison::dominant_delta_source;
use crate::persistence::cooling_baseline::COOLING_BASELINE_RECENT_WINDOW_DAYS;
use crate::persistence::cooling_covariate_rollup::{
  CovariateDailySummary, FanCovariateDailySummary, PairedFitStatistics, median,
};
use crate::persistence::cooling_delta_baseline::DeltaBaselineState;
use crate::persistence::cooling_rollup::CpuLoadBand;
use crate::persistence::cooling_thermal_delta_rollup::ThermalDeltaDailySummary;

/// Minimum Thermal Delta paired minutes a window must carry, in the
/// compared band, before its co-variates are compared against the other
/// window's.
///
/// Twice the ambient-adjusted comparison's 30-minute bar
/// (`COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES`), and its own
/// constant rather than a multiple of it. That bar guards an average,
/// which needs only a count; this one also guards a slope, which needs
/// spread along the power axis as well, and one hour of one band is the
/// least over which package power can be expected to have ranged at all.
/// Below it the windows are reported and nothing is judged (DP-02).
pub const COOLING_COVARIATE_COMPARISON_MINIMUM_PAIRED_MINUTES: u32 = 60;

/// Minimum baseline days carrying a factor before that factor's
/// interquartile range is read.
///
/// Four is the smallest count at which each quartile falls between two
/// observed days rather than on one; with fewer, the "range" collapses
/// onto the values themselves and any recent day reads as having moved.
/// The baseline window is seven days, so a factor archived through the
/// window clears this comfortably.
pub const COOLING_COVARIATE_RANGE_MINIMUM_DAYS: usize = 4;

/// Where a factor's recent value sits against the baseline window's own
/// daily spread.
///
/// The rule: the baseline window's daily medians of the factor form a
/// series; `WithinRange` means the recent window's median lies inside
/// that series' interquartile range (first to third quartile, inclusive,
/// quartiles by linear interpolation between the sorted daily values),
/// and `Moved` means it lies outside. `NotComparable` is a factor both
/// windows carry but that cannot be judged - too few baseline days for a
/// range ([`COOLING_COVARIATE_RANGE_MINIMUM_DAYS`]), no recent day, or
/// the windows as a whole not comparable. `Absent` is a factor neither
/// window ever archived: it is never reported as zero, and never as
/// having stayed within range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorJudgement {
  WithinRange,
  Moved,
  NotComparable,
  Absent,
}

/// One archived co-variate across the two windows.
///
/// `baseline` and `recent` are each window's median of its daily
/// medians - the same series the interquartile range is read from, so
/// "within range" means "a day like the baseline's own days".
/// `change` is `recent - baseline`, present only when both are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FactorComparison {
  pub baseline: Option<f32>,
  pub recent: Option<f32>,
  pub change: Option<f32>,
  pub judgement: FactorJudgement,
}

/// The least-squares line through one window's paired minutes, derived
/// from the summed statistics the rollup stores.
///
/// For the ΔT-power fit, `slope` is kelvin per watt - the thermal
/// resistance of the heat path at steady state - and `intercept` the ΔT
/// the line reads at zero power. For the ΔT-fan fit the slope is kelvin
/// per rpm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeastSquaresFit {
  pub slope: f32,
  pub intercept: f32,
  pub pearson_r: f32,
  /// Paired minutes the line was fitted over.
  pub paired_minutes: u32,
}

impl LeastSquaresFit {
  /// The fit, or `None` where there is none to have: fewer than two
  /// paired minutes, or no spread in either reading. A slope through
  /// minutes that all sit at one power is not a slope, and a Pearson r
  /// over a constant ΔT is not a number; both read as absent rather than
  /// as zero.
  pub fn from_statistics(statistics: &PairedFitStatistics) -> Option<Self> {
    if statistics.n < 2 {
      return None;
    }
    let n = statistics.n as f64;
    let sxx = statistics.sum_xx - statistics.sum_x * statistics.sum_x / n;
    let syy = statistics.sum_yy - statistics.sum_y * statistics.sum_y / n;
    let sxy = statistics.sum_xy - statistics.sum_x * statistics.sum_y / n;
    if sxx <= 0.0 || syy <= 0.0 {
      return None;
    }
    let slope = sxy / sxx;
    let intercept = (statistics.sum_y - slope * statistics.sum_x) / n;
    Some(Self {
      slope: slope as f32,
      intercept: intercept as f32,
      pearson_r: (sxy / (sxx * syy).sqrt()) as f32,
      paired_minutes: statistics.n,
    })
  }

  /// The line's value at `x`.
  pub fn at(&self, x: f32) -> f32 {
    self.slope * x + self.intercept
  }
}

/// One fan's speed across the two windows, with each window's ΔT-per-rpm
/// fit.
#[derive(Debug, Clone, PartialEq)]
pub struct FanCovariateComparison {
  /// The fan's stable channel-derived identifier, as archived.
  pub fan_source: String,
  pub speed: FactorComparison,
  pub baseline_fit: Option<LeastSquaresFit>,
  pub recent_fit: Option<LeastSquaresFit>,
}

/// Why the two windows are, or are not, compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovariateComparability {
  Comparable,
  /// One window carries fewer than
  /// [`COOLING_COVARIATE_COMPARISON_MINIMUM_PAIRED_MINUTES`] in the
  /// compared band - including a recent window no source paired at all.
  TooFewPairedMinutes,
  /// The recent window's dominant source is not the source the Thermal
  /// Delta Baseline was established from (#2062).
  DifferentAmbientSource,
}

/// The comparison once the Thermal Delta Baseline is established.
#[derive(Debug, Clone, PartialEq)]
pub struct EstablishedCovariateComparison {
  /// The CPU-load band both windows are read under.
  pub band: CpuLoadBand,
  /// The ambient source the Thermal Delta Baseline was established from;
  /// the baseline side is read from its rows only.
  pub baseline_source: String,
  pub baseline_window_start_date: NaiveDate,
  pub baseline_window_end_date: NaiveDate,
  /// The source that covered most of the recent window, or `None` when
  /// no source paired a minute in it.
  pub recent_source: Option<String>,
  pub recent_window_start_date: NaiveDate,
  pub recent_window_end_date: NaiveDate,
  /// Thermal Delta paired minutes in the band, per window - the evidence
  /// the gate is decided on.
  pub baseline_paired_minutes: u32,
  pub recent_paired_minutes: u32,
  pub package_power: FactorComparison,
  pub ambient_temperature: FactorComparison,
  /// The band's share of each window's classifiable paired minutes.
  pub load_band_share: FactorComparison,
  /// One entry per fan either window archived, ordered by fan source.
  pub fans: Vec<FanCovariateComparison>,
  /// Each window's ΔT-per-watt line, present wherever that window alone
  /// supports one - regardless of `comparable`, which gates only the
  /// comparison between them.
  pub baseline_fit: Option<LeastSquaresFit>,
  pub recent_fit: Option<LeastSquaresFit>,
  /// How much higher the recent line sits than the baseline line at the
  /// baseline window's median package power: the ΔT change at matched
  /// power. `None` unless `comparable`, both fits exist, and the
  /// baseline window archived power.
  pub delta_at_baseline_median_power: Option<f32>,
  pub comparable: bool,
  pub comparability: CovariateComparability,
}

/// Cooling Insight's co-variate comparison, gated by the Thermal Delta
/// Baseline's lifecycle exactly as the ambient-adjusted band comparison
/// is: while it establishes there is no baseline window to read.
#[derive(Debug, Clone, PartialEq)]
pub enum CoolingCovariateComparison {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  /// Boxed for the same reason `CoolingBandComparison` boxes its bands:
  /// the established variant dwarfs the other.
  Established(Box<EstablishedCovariateComparison>),
}

/// Derive the co-variate comparison for `band` from every summarized
/// co-variate row and the Thermal Delta Baseline's lifecycle.
///
/// `window_end_date` is the most recent completed local day (yesterday),
/// matching [`crate::persistence::cooling_band_comparison::derive_band_comparison`].
/// `delta_days` is the row-per-source ΔT rollup, consulted only to pick
/// the recent window's dominant source by coverage - the same rule the
/// ambient-adjusted readings use, so all three views read one sensor.
pub fn derive_covariate_comparison(
  covariate_days: &[CovariateDailySummary],
  fan_days: &[FanCovariateDailySummary],
  delta_days: &[ThermalDeltaDailySummary],
  delta_baseline_state: DeltaBaselineState,
  band: CpuLoadBand,
  window_end_date: NaiveDate,
) -> CoolingCovariateComparison {
  let (baseline_source, baseline_start, baseline_end) = match delta_baseline_state {
    DeltaBaselineState::Establishing {
      qualifying_days,
      required_days,
    } => {
      return CoolingCovariateComparison::Establishing {
        qualifying_days,
        required_days,
      };
    }
    DeltaBaselineState::Established {
      source,
      window_start_date,
      window_end_date,
      ..
    } => (source, window_start_date, window_end_date),
  };

  let recent_start =
    window_end_date - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
  let recent_source = dominant_delta_source(delta_days, recent_start, window_end_date);

  let baseline = WindowSlice::of(
    covariate_days,
    fan_days,
    &baseline_source,
    band,
    baseline_start,
    baseline_end,
  );
  let recent = recent_source.map_or_else(WindowSlice::default, |source| {
    WindowSlice::of(
      covariate_days,
      fan_days,
      source,
      band,
      recent_start,
      window_end_date,
    )
  });

  let baseline_paired_minutes = baseline.paired_minutes();
  let recent_paired_minutes = recent.paired_minutes();
  // Thinness first, then the source: a recent window no sensor paired at
  // all is "not enough evidence", not "a different sensor".
  let comparability = if baseline_paired_minutes
    < COOLING_COVARIATE_COMPARISON_MINIMUM_PAIRED_MINUTES
    || recent_paired_minutes < COOLING_COVARIATE_COMPARISON_MINIMUM_PAIRED_MINUTES
  {
    CovariateComparability::TooFewPairedMinutes
  } else if recent_source != Some(baseline_source.as_str()) {
    CovariateComparability::DifferentAmbientSource
  } else {
    CovariateComparability::Comparable
  };
  let comparable = comparability == CovariateComparability::Comparable;

  let package_power =
    compare_factor(&baseline.power_values(), &recent.power_values(), comparable);
  let baseline_fit = LeastSquaresFit::from_statistics(&baseline.fit_statistics());
  let recent_fit = LeastSquaresFit::from_statistics(&recent.fit_statistics());
  let delta_at_baseline_median_power =
    match (comparable, baseline_fit, recent_fit, package_power.baseline) {
      (true, Some(baseline_fit), Some(recent_fit), Some(power)) => {
        Some(recent_fit.at(power) - baseline_fit.at(power))
      }
      _ => None,
    };

  let fans = baseline
    .fan_sources()
    .union(&recent.fan_sources())
    .map(|fan_source| FanCovariateComparison {
      fan_source: (*fan_source).to_string(),
      speed: compare_factor(
        &baseline.fan_values(fan_source),
        &recent.fan_values(fan_source),
        comparable,
      ),
      baseline_fit: LeastSquaresFit::from_statistics(
        &baseline.fan_fit_statistics(fan_source),
      ),
      recent_fit: LeastSquaresFit::from_statistics(
        &recent.fan_fit_statistics(fan_source),
      ),
    })
    .collect();

  CoolingCovariateComparison::Established(Box::new(EstablishedCovariateComparison {
    band,
    baseline_source,
    baseline_window_start_date: baseline_start,
    baseline_window_end_date: baseline_end,
    recent_source: recent_source.map(str::to_string),
    recent_window_start_date: recent_start,
    recent_window_end_date: window_end_date,
    baseline_paired_minutes,
    recent_paired_minutes,
    package_power,
    ambient_temperature: compare_factor(
      &baseline.ambient_values(),
      &recent.ambient_values(),
      comparable,
    ),
    load_band_share: compare_factor(
      &baseline.band_share_values(),
      &recent.band_share_values(),
      comparable,
    ),
    fans,
    baseline_fit,
    recent_fit,
    delta_at_baseline_median_power,
    comparable,
    comparability,
  }))
}

/// One source's rows for one band across one date window.
///
/// Reads `source`'s rows only. Folding two sources' rows into one window
/// would blend two sensor placements, so there is deliberately no
/// source-agnostic variant.
#[derive(Default)]
struct WindowSlice<'a> {
  /// This band's rows in the window, at most one per day.
  band_rows: Vec<&'a CovariateDailySummary>,
  /// Every day in the window on which the source has a row in *any*
  /// band - what makes a day the source observed entirely outside this
  /// band an honest zero share rather than a missing one.
  observed_days: BTreeSet<NaiveDate>,
  fan_rows: Vec<&'a FanCovariateDailySummary>,
}

impl<'a> WindowSlice<'a> {
  fn of(
    covariate_days: &'a [CovariateDailySummary],
    fan_days: &'a [FanCovariateDailySummary],
    source: &str,
    band: CpuLoadBand,
    start: NaiveDate,
    end: NaiveDate,
  ) -> Self {
    let in_window = |date: NaiveDate| date >= start && date <= end;
    let source_rows = covariate_days
      .iter()
      .filter(|row| row.source == source && in_window(row.date));
    let mut slice = Self::default();
    for row in source_rows {
      slice.observed_days.insert(row.date);
      if row.band == band {
        slice.band_rows.push(row);
      }
    }
    slice.fan_rows = fan_days
      .iter()
      .filter(|row| row.source == source && row.band == band && in_window(row.date))
      .collect();
    slice
  }

  fn paired_minutes(&self) -> u32 {
    self.band_rows.iter().map(|row| row.delta_minutes).sum()
  }

  fn fit_statistics(&self) -> PairedFitStatistics {
    let mut statistics = PairedFitStatistics::default();
    for row in &self.band_rows {
      statistics.merge(&row.delta_per_watt);
    }
    statistics
  }

  fn power_values(&self) -> Vec<f32> {
    self
      .band_rows
      .iter()
      .filter_map(|row| row.cpu_power_median)
      .collect()
  }

  fn ambient_values(&self) -> Vec<f32> {
    self
      .band_rows
      .iter()
      .map(|row| row.ambient_temperature_median)
      .collect()
  }

  fn band_share_values(&self) -> Vec<f32> {
    self
      .observed_days
      .iter()
      .map(|date| {
        self
          .band_rows
          .iter()
          .find(|row| row.date == *date)
          .map_or(0.0, |row| row.band_share)
      })
      .collect()
  }

  fn fan_sources(&self) -> BTreeSet<&'a str> {
    self
      .fan_rows
      .iter()
      .map(|row| row.fan_source.as_str())
      .collect()
  }

  fn fan_values(&self, fan_source: &str) -> Vec<f32> {
    self
      .fan_rows
      .iter()
      .filter(|row| row.fan_source == fan_source)
      .map(|row| row.rpm_median)
      .collect()
  }

  fn fan_fit_statistics(&self, fan_source: &str) -> PairedFitStatistics {
    let mut statistics = PairedFitStatistics::default();
    for row in self
      .fan_rows
      .iter()
      .filter(|row| row.fan_source == fan_source)
    {
      statistics.merge(&row.delta_per_rpm);
    }
    statistics
  }
}

/// Judge one factor from each window's daily values - see
/// [`FactorJudgement`] for the rule.
fn compare_factor(
  baseline_values: &[f32],
  recent_values: &[f32],
  comparable: bool,
) -> FactorComparison {
  let baseline = median(baseline_values);
  let recent = median(recent_values);
  if baseline.is_none() && recent.is_none() {
    return FactorComparison {
      baseline: None,
      recent: None,
      change: None,
      judgement: FactorJudgement::Absent,
    };
  }

  let judgement = match (comparable, recent, interquartile_range(baseline_values)) {
    (true, Some(recent), Some((first_quartile, third_quartile))) => {
      if first_quartile <= recent && recent <= third_quartile {
        FactorJudgement::WithinRange
      } else {
        FactorJudgement::Moved
      }
    }
    _ => FactorJudgement::NotComparable,
  };

  FactorComparison {
    baseline,
    recent,
    change: baseline
      .zip(recent)
      .map(|(baseline, recent)| recent - baseline),
    judgement,
  }
}

/// `(Q1, Q3)` of `values`, or `None` below
/// [`COOLING_COVARIATE_RANGE_MINIMUM_DAYS`] values.
fn interquartile_range(values: &[f32]) -> Option<(f32, f32)> {
  if values.len() < COOLING_COVARIATE_RANGE_MINIMUM_DAYS {
    return None;
  }
  let mut sorted = values.to_vec();
  sorted.sort_by(f32::total_cmp);
  Some((quantile(&sorted, 0.25), quantile(&sorted, 0.75)))
}

/// The `p`-quantile of `sorted` by linear interpolation between the
/// sorted values at position `p × (n − 1)`.
fn quantile(sorted: &[f32], p: f32) -> f32 {
  let position = p * (sorted.len() - 1) as f32;
  let lower = position.floor() as usize;
  let fraction = position - lower as f32;
  match sorted.get(lower + 1) {
    Some(upper) => sorted[lower] + fraction * (upper - sorted[lower]),
    None => sorted[lower],
  }
}

/// [`derive_covariate_comparison`] over the whole co-variate tables,
/// resolving the Thermal Delta Baseline through its own resolver rather
/// than re-deriving it, for the same reason every other reader does: the
/// pinned row must win once one exists.
pub(crate) async fn load_cooling_covariate_comparison_from_pool(
  pool: &sqlx::SqlitePool,
  band: CpuLoadBand,
  today: NaiveDate,
) -> Result<CoolingCovariateComparison, sqlx::Error> {
  use crate::infrastructure::database;

  let delta_days =
    database::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries_from_pool(
      pool,
    )
    .await?;
  let delta_baseline_state =
    crate::persistence::cooling_delta_baseline::resolve_delta_baseline_state_from_pool(
      pool,
      &delta_days,
    )
    .await?;
  let covariate_days =
    database::cooling_covariate_daily_summary::select_all_covariate_daily_summaries_from_pool(
      pool,
    )
    .await?;
  let fan_days =
    database::cooling_covariate_daily_summary::select_all_fan_covariate_daily_summaries_from_pool(
      pool,
    )
    .await?;
  let yesterday = today - Duration::days(1);

  Ok(derive_covariate_comparison(
    &covariate_days,
    &fan_days,
    &delta_days,
    delta_baseline_state,
    band,
    yesterday,
  ))
}

/// [`load_cooling_covariate_comparison_from_pool`] against Core's
/// process-wide pool, for `band` - the App command names the band the
/// observation strip compares under.
pub async fn load_cooling_covariate_comparison(
  band: CpuLoadBand,
) -> Result<CoolingCovariateComparison, sqlx::Error> {
  let pool = crate::infrastructure::database::db::get_pool().await?;
  load_cooling_covariate_comparison_from_pool(
    &pool,
    band,
    chrono::Local::now().date_naive(),
  )
  .await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::persistence::cooling_rollup::BandSummary;

  /// The baseline window: the first week of August.
  const BASELINE: (NaiveDate, NaiveDate) = (
    NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
    NaiveDate::from_ymd_opt(2026, 8, 7).unwrap(),
  );
  /// Yesterday; the recent window is the seven days ending here.
  const RECENT_END: NaiveDate = NaiveDate::from_ymd_opt(2026, 8, 20).unwrap();
  const RECENT_START: NaiveDate = NaiveDate::from_ymd_opt(2026, 8, 14).unwrap();

  fn established(source: &str) -> DeltaBaselineState {
    DeltaBaselineState::Established {
      source: source.to_string(),
      delta_temperature_avg: 15.0,
      window_start_date: BASELINE.0,
      window_end_date: BASELINE.1,
      sample_minutes: 4200,
    }
  }

  fn establishing() -> DeltaBaselineState {
    DeltaBaselineState::Establishing {
      qualifying_days: 2,
      required_days: 7,
    }
  }

  /// The statistics of `points`, as the rollup would have summed them.
  fn statistics(points: &[(f64, f64)]) -> PairedFitStatistics {
    let mut statistics = PairedFitStatistics::default();
    for (x, y) in points {
      statistics.push(*x, *y);
    }
    statistics
  }

  /// A day's ΔT-power sums over four powers whose least-squares line is
  /// exactly `y = slope × x + intercept`: the residuals are symmetric
  /// about the mean power, so they leave the slope and intercept alone
  /// while keeping the ΔT spread r needs.
  fn line(slope: f64, intercept: f64) -> PairedFitStatistics {
    statistics(&[
      (10.0, slope * 10.0 + intercept + 0.5),
      (20.0, slope * 20.0 + intercept - 0.5),
      (30.0, slope * 30.0 + intercept - 0.5),
      (40.0, slope * 40.0 + intercept + 0.5),
    ])
  }

  /// One idle-band row: 100 paired minutes, ambient 25 °C, the given
  /// power median, and a ΔT-power line of 1 K/W through 5 K.
  fn row(date: NaiveDate, source: &str, power: Option<f32>) -> CovariateDailySummary {
    CovariateDailySummary {
      date,
      source: source.to_string(),
      band: CpuLoadBand::Idle,
      sample_minutes: 100,
      band_share: 0.8,
      ambient_temperature_median: 25.0,
      delta_minutes: 100,
      delta_temperature_median: Some(15.0),
      power_minutes: if power.is_some() { 100 } else { 0 },
      cpu_power_median: power,
      delta_per_watt: if power.is_some() {
        line(1.0, 5.0)
      } else {
        PairedFitStatistics::default()
      },
    }
  }

  fn fan_row(
    date: NaiveDate,
    source: &str,
    fan: &str,
    rpm: f32,
  ) -> FanCovariateDailySummary {
    FanCovariateDailySummary {
      date,
      source: source.to_string(),
      fan_source: fan.to_string(),
      band: CpuLoadBand::Idle,
      rpm_minutes: 100,
      rpm_median: rpm,
      delta_per_rpm: statistics(&[(800.0, 16.0), (900.0, 15.0), (1000.0, 14.0)]),
    }
  }

  /// A ΔT row carrying only coverage, so the recent window's dominant
  /// source can be chosen.
  fn coverage(date: NaiveDate, source: &str, minutes: u32) -> ThermalDeltaDailySummary {
    ThermalDeltaDailySummary {
      date,
      source: source.to_string(),
      coverage_minutes: minutes,
      idle: BandSummary::default(),
      low: BandSummary::default(),
      mid: BandSummary::default(),
      high: BandSummary::default(),
    }
  }

  fn days(window: (NaiveDate, NaiveDate)) -> impl Iterator<Item = NaiveDate> {
    window.0.iter_days().take_while(move |d| *d <= window.1)
  }

  /// `source`'s rows over the full baseline window at `baseline_power`
  /// and the full recent window at `recent_power`, plus the ΔT coverage
  /// that makes it the recent window's dominant source.
  fn both_windows(
    source: &str,
    baseline_power: Option<f32>,
    recent_power: Option<f32>,
  ) -> (Vec<CovariateDailySummary>, Vec<ThermalDeltaDailySummary>) {
    let rows = days(BASELINE)
      .map(|d| row(d, source, baseline_power))
      .chain(days((RECENT_START, RECENT_END)).map(|d| row(d, source, recent_power)))
      .collect();
    let coverage = days((RECENT_START, RECENT_END))
      .map(|d| coverage(d, source, 1000))
      .collect();
    (rows, coverage)
  }

  fn established_result(
    result: CoolingCovariateComparison,
  ) -> EstablishedCovariateComparison {
    match result {
      CoolingCovariateComparison::Established(comparison) => *comparison,
      CoolingCovariateComparison::Establishing { .. } => {
        panic!("expected an established comparison")
      }
    }
  }

  fn derive(
    rows: &[CovariateDailySummary],
    fans: &[FanCovariateDailySummary],
    coverage: &[ThermalDeltaDailySummary],
    baseline: DeltaBaselineState,
  ) -> EstablishedCovariateComparison {
    established_result(derive_covariate_comparison(
      rows,
      fans,
      coverage,
      baseline,
      CpuLoadBand::Idle,
      RECENT_END,
    ))
  }

  // ── the fit ──

  #[test]
  fn the_slope_recovered_from_the_summed_statistics_matches_a_hand_computed_fit() {
    // x = 10, 20, 30, 40; y = 15, 24, 37, 44.
    // Σx = 100, Σy = 120, Σxy = 3500, Σx² = 3000, Σy² = 4106.
    // Sxx = 3000 − 100²/4 = 500; Sxy = 3500 − 100·120/4 = 500;
    // Syy = 4106 − 120²/4 = 506.
    // slope = 500/500 = 1; intercept = (120 − 1·100)/4 = 5;
    // r = 500/√(500·506) ≈ 0.99405.
    let fit = LeastSquaresFit::from_statistics(&statistics(&[
      (10.0, 15.0),
      (20.0, 24.0),
      (30.0, 37.0),
      (40.0, 44.0),
    ]))
    .unwrap();

    assert!((fit.slope - 1.0).abs() < 1e-6, "slope was {}", fit.slope);
    assert!(
      (fit.intercept - 5.0).abs() < 1e-5,
      "intercept was {}",
      fit.intercept
    );
    assert!(
      (fit.pearson_r - 0.99405).abs() < 1e-4,
      "r was {}",
      fit.pearson_r
    );
    assert_eq!(fit.paired_minutes, 4);
  }

  #[test]
  fn the_fit_over_a_window_is_the_fit_over_its_days_summed() {
    // Two days each holding half of the four points above: the merged
    // sums must reproduce the same line, which is the whole reason the
    // rollup stores sums rather than a per-day slope.
    let mut merged = statistics(&[(10.0, 15.0), (20.0, 24.0)]);
    merged.merge(&statistics(&[(30.0, 37.0), (40.0, 44.0)]));

    let fit = LeastSquaresFit::from_statistics(&merged).unwrap();

    assert!((fit.slope - 1.0).abs() < 1e-6);
    assert!((fit.intercept - 5.0).abs() < 1e-5);
  }

  #[test]
  fn a_fit_needs_two_minutes_and_spread_in_both_readings() {
    assert_eq!(
      LeastSquaresFit::from_statistics(&statistics(&[(10.0, 15.0)])),
      None,
      "one minute is a point, not a line"
    );
    assert_eq!(
      LeastSquaresFit::from_statistics(&statistics(&[(10.0, 15.0), (10.0, 20.0)])),
      None,
      "every minute at one power has no slope"
    );
    assert_eq!(
      LeastSquaresFit::from_statistics(&statistics(&[(10.0, 15.0), (20.0, 15.0)])),
      None,
      "a constant ΔT has no correlation to report"
    );
    assert_eq!(
      LeastSquaresFit::from_statistics(&PairedFitStatistics::default()),
      None
    );
  }

  // ── lifecycle and gate ──

  #[test]
  fn an_establishing_delta_baseline_reports_establishing() {
    let (rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));

    let result = derive_covariate_comparison(
      &rows,
      &[],
      &coverage,
      establishing(),
      CpuLoadBand::Idle,
      RECENT_END,
    );

    assert_eq!(
      result,
      CoolingCovariateComparison::Establishing {
        qualifying_days: 2,
        required_days: 7,
      }
    );
  }

  #[test]
  fn a_recent_window_from_a_different_ambient_source_is_not_comparable() {
    // The baseline was pinned from the Desk sensor; a Living Room sensor
    // covered the whole recent window. Both windows are reported, nothing
    // is judged, and the matched-power difference is withheld.
    let (mut rows, coverage) = both_windows("Living Room", Some(20.0), Some(20.0));
    rows.extend(days(BASELINE).map(|d| row(d, "Desk", Some(20.0))));

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert!(!result.comparable);
    assert_eq!(
      result.comparability,
      CovariateComparability::DifferentAmbientSource
    );
    assert_eq!(result.baseline_source, "Desk");
    assert_eq!(result.recent_source.as_deref(), Some("Living Room"));
    assert_eq!(result.package_power.baseline, Some(20.0));
    assert_eq!(result.package_power.recent, Some(20.0));
    assert_eq!(
      result.package_power.judgement,
      FactorJudgement::NotComparable
    );
    assert_eq!(
      result.ambient_temperature.judgement,
      FactorJudgement::NotComparable
    );
    assert!(result.baseline_fit.is_some());
    assert!(result.recent_fit.is_some());
    assert_eq!(result.delta_at_baseline_median_power, None);
  }

  #[test]
  fn too_few_paired_minutes_withholds_the_comparison() {
    let (mut rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    for row in rows.iter_mut().filter(|row| row.date >= RECENT_START) {
      row.delta_minutes = 8;
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.recent_paired_minutes, 56);
    assert!(!result.comparable);
    assert_eq!(
      result.comparability,
      CovariateComparability::TooFewPairedMinutes
    );
    assert_eq!(
      result.package_power.judgement,
      FactorJudgement::NotComparable
    );
  }

  #[test]
  fn a_recent_window_no_source_paired_is_too_thin_rather_than_a_different_sensor() {
    let rows: Vec<_> = days(BASELINE).map(|d| row(d, "Desk", Some(20.0))).collect();

    let result = derive(&rows, &[], &[], established("Desk"));

    assert_eq!(result.recent_source, None);
    assert_eq!(result.recent_paired_minutes, 0);
    assert_eq!(
      result.comparability,
      CovariateComparability::TooFewPairedMinutes
    );
  }

  #[test]
  fn exactly_the_minimum_paired_minutes_on_both_sides_is_comparable() {
    let (mut rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    for row in rows.iter_mut() {
      row.delta_minutes = if row.date == BASELINE.0 || row.date == RECENT_START {
        COOLING_COVARIATE_COMPARISON_MINIMUM_PAIRED_MINUTES
      } else {
        0
      };
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert!(result.comparable);
    assert_eq!(result.comparability, CovariateComparability::Comparable);
  }

  // ── factors ──

  #[test]
  fn a_factor_never_archived_reports_absent_rather_than_zero() {
    // No power sampler on this machine: the factor is absent on both
    // sides, and so is the fit that would have needed it.
    let (rows, coverage) = both_windows("Desk", None, None);

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert!(result.comparable, "power is a factor, not the gate");
    assert_eq!(
      result.package_power,
      FactorComparison {
        baseline: None,
        recent: None,
        change: None,
        judgement: FactorJudgement::Absent,
      }
    );
    assert_eq!(result.baseline_fit, None);
    assert_eq!(result.recent_fit, None);
    assert_eq!(result.delta_at_baseline_median_power, None);
    assert!(result.fans.is_empty(), "no fan was ever archived either");
  }

  #[test]
  fn a_recent_value_inside_the_baselines_interquartile_range_is_within_range() {
    // Baseline power medians 18, 19, 20, 21, 22, 23, 24 W: Q1 = 19.5,
    // Q3 = 22.5. A recent week at 22 W sits inside.
    let (mut rows, coverage) = both_windows("Desk", None, Some(22.0));
    for (row, power) in rows
      .iter_mut()
      .filter(|row| row.date <= BASELINE.1)
      .zip([18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0])
    {
      row.cpu_power_median = Some(power);
      row.power_minutes = 100;
      row.delta_per_watt = line(1.0, 5.0);
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.package_power.baseline, Some(21.0));
    assert_eq!(result.package_power.recent, Some(22.0));
    assert_eq!(result.package_power.change, Some(1.0));
    assert_eq!(result.package_power.judgement, FactorJudgement::WithinRange);
  }

  #[test]
  fn a_recent_value_outside_the_baselines_interquartile_range_has_moved() {
    // Same baseline spread; a recent week at 26 W is above Q3 = 22.5.
    let (mut rows, coverage) = both_windows("Desk", None, Some(26.0));
    for (row, power) in rows
      .iter_mut()
      .filter(|row| row.date <= BASELINE.1)
      .zip([18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0])
    {
      row.cpu_power_median = Some(power);
      row.power_minutes = 100;
      row.delta_per_watt = line(1.0, 5.0);
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.package_power.change, Some(5.0));
    assert_eq!(result.package_power.judgement, FactorJudgement::Moved);
  }

  #[test]
  fn the_quartile_edges_themselves_count_as_within_range() {
    let values = [18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0];
    let (q1, q3) = interquartile_range(&values).unwrap();
    assert_eq!((q1, q3), (19.5, 22.5));

    assert_eq!(
      compare_factor(&values, &[19.5], true).judgement,
      FactorJudgement::WithinRange
    );
    assert_eq!(
      compare_factor(&values, &[22.5], true).judgement,
      FactorJudgement::WithinRange
    );
    assert_eq!(
      compare_factor(&values, &[19.4], true).judgement,
      FactorJudgement::Moved
    );
  }

  #[test]
  fn too_few_baseline_days_for_an_interquartile_range_is_not_comparable() {
    // Three baseline days carry power; the factor is present on both
    // sides, but the range would collapse onto the values themselves.
    let (mut rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    for row in rows
      .iter_mut()
      .filter(|row| row.date <= BASELINE.1)
      .skip(COOLING_COVARIATE_RANGE_MINIMUM_DAYS - 1)
    {
      row.cpu_power_median = None;
      row.power_minutes = 0;
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert!(result.comparable);
    assert_eq!(result.package_power.baseline, Some(20.0));
    assert_eq!(result.package_power.recent, Some(20.0));
    assert_eq!(
      result.package_power.judgement,
      FactorJudgement::NotComparable
    );
  }

  #[test]
  fn a_factor_present_on_one_side_only_is_not_comparable_rather_than_absent() {
    // A power sampler that arrived after the baseline: nothing to compare
    // against yet, but the recent value is real and reported.
    let (rows, coverage) = both_windows("Desk", None, Some(20.0));

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.package_power.baseline, None);
    assert_eq!(result.package_power.recent, Some(20.0));
    assert_eq!(result.package_power.change, None);
    assert_eq!(
      result.package_power.judgement,
      FactorJudgement::NotComparable
    );
  }

  #[test]
  fn a_day_the_source_saw_only_other_bands_counts_as_zero_share_for_this_band() {
    // Baseline: seven days at 0.8 idle share. Recent: the same, except
    // three days on which the source paired only high-band minutes. The
    // recent share series is 0.8 ×4 and 0 ×3, median 0.8 - still within
    // range - but the zeros are there, not missing.
    let (mut rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    for row in rows
      .iter_mut()
      .filter(|row| row.date > RECENT_START + Duration::days(3))
    {
      row.band = CpuLoadBand::High;
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.load_band_share.baseline, Some(0.8));
    assert_eq!(result.load_band_share.recent, Some(0.8));
    assert_eq!(result.recent_paired_minutes, 400);
  }

  #[test]
  fn ambient_temperature_is_judged_on_the_days_medians() {
    let (mut rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    for row in rows.iter_mut().filter(|row| row.date >= RECENT_START) {
      row.ambient_temperature_median = 31.0;
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.ambient_temperature.baseline, Some(25.0));
    assert_eq!(result.ambient_temperature.recent, Some(31.0));
    assert_eq!(result.ambient_temperature.change, Some(6.0));
    // Every baseline day sits at 25 °C, so its IQR is the point 25.
    assert_eq!(result.ambient_temperature.judgement, FactorJudgement::Moved);
  }

  // ── fans ──

  #[test]
  fn each_fan_is_judged_on_its_own_rows_with_its_own_fits() {
    let (rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    let mut fans: Vec<_> = days(BASELINE)
      .flat_map(|d| {
        [
          fan_row(d, "Desk", "Fan 1", 900.0),
          fan_row(d, "Desk", "Fan 2", 1500.0),
        ]
      })
      .collect();
    fans.extend(days((RECENT_START, RECENT_END)).flat_map(|d| {
      [
        fan_row(d, "Desk", "Fan 1", 900.0),
        fan_row(d, "Desk", "Fan 2", 1200.0),
      ]
    }));

    let result = derive(&rows, &fans, &coverage, established("Desk"));

    assert_eq!(
      result
        .fans
        .iter()
        .map(|fan| (
          fan.fan_source.as_str(),
          fan.speed.change,
          fan.speed.judgement
        ))
        .collect::<Vec<_>>(),
      vec![
        ("Fan 1", Some(0.0), FactorJudgement::WithinRange),
        ("Fan 2", Some(-300.0), FactorJudgement::Moved),
      ]
    );
    let fit = result.fans[0].baseline_fit.unwrap();
    assert!(
      (fit.slope - -0.01).abs() < 1e-6,
      "K per rpm; slope was {}",
      fit.slope
    );
    assert!(result.fans[0].recent_fit.is_some());
  }

  #[test]
  fn a_fan_archived_in_one_window_only_is_listed_as_not_comparable() {
    let (rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    let fans: Vec<_> = days((RECENT_START, RECENT_END))
      .map(|d| fan_row(d, "Desk", "Fan 1", 900.0))
      .collect();

    let result = derive(&rows, &fans, &coverage, established("Desk"));

    assert_eq!(result.fans.len(), 1);
    assert_eq!(result.fans[0].speed.baseline, None);
    assert_eq!(result.fans[0].speed.recent, Some(900.0));
    assert_eq!(
      result.fans[0].speed.judgement,
      FactorJudgement::NotComparable
    );
    assert_eq!(result.fans[0].baseline_fit, None);
  }

  // ── matched power ──

  #[test]
  fn the_delta_difference_is_read_at_the_baseline_windows_median_power() {
    // Baseline line 1 K/W through 5 K, recent line 1.2 K/W through 6 K,
    // baseline median power 25 W: (1.2·25 + 6) − (1·25 + 5) = 6 K.
    let (mut rows, coverage) = both_windows("Desk", Some(25.0), Some(25.0));
    for row in rows.iter_mut().filter(|row| row.date >= RECENT_START) {
      row.delta_per_watt = line(1.2, 6.0);
    }

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert!(result.comparable);
    let baseline_fit = result.baseline_fit.unwrap();
    let recent_fit = result.recent_fit.unwrap();
    assert!((baseline_fit.slope - 1.0).abs() < 1e-5);
    assert!((recent_fit.slope - 1.2).abs() < 1e-5);
    assert_eq!(baseline_fit.paired_minutes, 28);
    let difference = result.delta_at_baseline_median_power.unwrap();
    assert!(
      (difference - 6.0).abs() < 1e-4,
      "difference was {difference}"
    );
  }

  // ── one source, never blended ──

  #[test]
  fn windows_are_read_from_one_source_and_never_blended() {
    // A second sensor's rows inside the baseline window, at very
    // different values, must leave the Desk-based comparison untouched.
    let (mut rows, coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    rows.extend(days(BASELINE).map(|d| CovariateDailySummary {
      ambient_temperature_median: 40.0,
      cpu_power_median: Some(90.0),
      ..row(d, "Living Room", Some(90.0))
    }));

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.package_power.baseline, Some(20.0));
    assert_eq!(result.ambient_temperature.baseline, Some(25.0));
    assert_eq!(result.baseline_paired_minutes, 700);
  }

  #[test]
  fn the_recent_window_is_read_from_the_source_that_covered_most_of_it() {
    // Desk covered the baseline; in the recent window both sensors have
    // rows, and the Living Room one covered more minutes. It wins, and
    // being a different source than the baseline's, nothing is judged.
    let (mut rows, mut coverage) = both_windows("Desk", Some(20.0), Some(20.0));
    rows.extend(
      days((RECENT_START, RECENT_END)).map(|d| row(d, "Living Room", Some(50.0))),
    );
    coverage
      .extend(days((RECENT_START, RECENT_END)).map(|d| coverage_row(d, "Living Room")));

    let result = derive(&rows, &[], &coverage, established("Desk"));

    assert_eq!(result.recent_source.as_deref(), Some("Living Room"));
    assert_eq!(result.package_power.recent, Some(50.0));
    assert_eq!(
      result.comparability,
      CovariateComparability::DifferentAmbientSource
    );
  }

  fn coverage_row(date: NaiveDate, source: &str) -> ThermalDeltaDailySummary {
    coverage(date, source, 1400)
  }

  // ── the loader ──

  #[tokio::test]
  async fn the_loader_reads_the_baseline_window_from_the_pinned_delta_baseline() {
    use crate::infrastructure::database::test_schema::{
      COOLING_COVARIATE_DAILY_SUMMARY_DDL, COOLING_DELTA_BASELINE_DDL,
      COOLING_FAN_COVARIATE_DAILY_SUMMARY_DDL, COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL,
      create_tables,
    };
    use crate::infrastructure::database::{
      cooling_covariate_daily_summary, cooling_delta_baseline,
      cooling_thermal_delta_daily_summary,
    };
    use crate::persistence::cooling_delta_baseline::EstablishedDeltaBaseline;

    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    create_tables(
      &pool,
      &[
        COOLING_DELTA_BASELINE_DDL,
        COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL,
        COOLING_COVARIATE_DAILY_SUMMARY_DDL,
        COOLING_FAN_COVARIATE_DAILY_SUMMARY_DDL,
      ],
    )
    .await;
    cooling_delta_baseline::insert_established_delta_baseline_from_pool(
      &pool,
      &EstablishedDeltaBaseline {
        source: "Desk".to_string(),
        delta_temperature_avg: 15.0,
        window_start_date: BASELINE.0,
        window_end_date: BASELINE.1,
        sample_minutes: 4200,
      },
      chrono::Utc::now(),
    )
    .await
    .unwrap();
    let (rows, coverage) = both_windows("Desk", Some(20.0), Some(24.0));
    for row in &rows {
      cooling_covariate_daily_summary::upsert_with(&pool, row)
        .await
        .unwrap();
    }
    for day in &coverage {
      cooling_thermal_delta_daily_summary::upsert_with(&pool, day)
        .await
        .unwrap();
    }
    cooling_covariate_daily_summary::upsert_fan_with(
      &pool,
      &fan_row(RECENT_END, "Desk", "Fan 1", 900.0),
    )
    .await
    .unwrap();

    let result = established_result(
      load_cooling_covariate_comparison_from_pool(
        &pool,
        CpuLoadBand::Idle,
        RECENT_END + Duration::days(1),
      )
      .await
      .unwrap(),
    );

    assert_eq!(result.baseline_source, "Desk");
    assert_eq!(
      (
        result.baseline_window_start_date,
        result.baseline_window_end_date
      ),
      BASELINE
    );
    assert_eq!(
      (
        result.recent_window_start_date,
        result.recent_window_end_date
      ),
      (RECENT_START, RECENT_END)
    );
    assert!(result.comparable);
    assert_eq!(result.package_power.change, Some(4.0));
    assert_eq!(result.fans.len(), 1);
  }

  #[tokio::test]
  async fn the_loader_reports_establishing_while_no_delta_baseline_exists() {
    use crate::infrastructure::database::test_schema::{
      COOLING_COVARIATE_DAILY_SUMMARY_DDL, COOLING_DELTA_BASELINE_DDL,
      COOLING_FAN_COVARIATE_DAILY_SUMMARY_DDL, COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL,
      create_tables,
    };

    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    create_tables(
      &pool,
      &[
        COOLING_DELTA_BASELINE_DDL,
        COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL,
        COOLING_COVARIATE_DAILY_SUMMARY_DDL,
        COOLING_FAN_COVARIATE_DAILY_SUMMARY_DDL,
      ],
    )
    .await;

    let result =
      load_cooling_covariate_comparison_from_pool(&pool, CpuLoadBand::Idle, RECENT_END)
        .await
        .unwrap();

    assert_eq!(
      result,
      CoolingCovariateComparison::Establishing {
        qualifying_days: 0,
        required_days: 7,
      }
    );
  }
}
