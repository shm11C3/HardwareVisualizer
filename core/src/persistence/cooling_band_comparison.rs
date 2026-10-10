//! Cooling Insight load-band comparison: for each CPU-load band
//! (idle/low/mid/high), the baseline window's temperature versus the
//! recent window's temperature, with sample counts (#2017).
//!
//! The baseline window is the same calendar range that established the
//! idle cooling baseline (see
//! [`crate::persistence::cooling_baseline`]) - reusing it keeps every
//! band's "baseline" anchored to the same period rather than each band
//! silently picking its own qualifying days. When no baseline is
//! established yet, there is no such window, so every band reports
//! [`BaselineState::Establishing`](crate::persistence::cooling_baseline::BaselineState::Establishing)
//! rather than a partial comparison.
//!
//! The idle band's baseline side *is* that pinned window. The other three
//! bands start from it and, when it holds too few of their minutes, extend
//! forward day by day until it does (#2333) - see
//! [`COOLING_BAND_BASELINE_EXTENSION_MAX_CALENDAR_DAYS`] for why, and for
//! the cap.

use std::collections::BTreeMap;
use std::ops::Bound;

use chrono::{Duration, NaiveDate};

use crate::persistence::cooling_baseline::{BaselineState, recent_window_start};
use crate::persistence::cooling_delta_baseline::DeltaBaselineState;
#[cfg(test)]
use crate::persistence::cooling_rollup::PowerSummary;
use crate::persistence::cooling_rollup::{BandSummary, CpuLoadBand, DailyCoolingSummary};
use crate::persistence::cooling_thermal_delta_rollup::ThermalDeltaDailySummary;

/// Minimum sample minutes a band's window must carry before that band's
/// comparison is meaningful. Applied independently to the baseline side
/// and the recent side of each band - a band comparable on one side but
/// not the other is still not comparable overall (DP-02: no delta
/// computed from a handful of minutes as if it were a measurement).
pub const COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES: u32 = 30;

/// Minimum ΔT sample minutes a window must carry before the
/// ambient-adjusted reading is offered for it (#2045).
///
/// Deliberately its own constant at the same value as
/// [`COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES`] rather than an alias
/// of it: the bar is the same idea (below it, report nothing rather than a
/// number derived from a handful of minutes) but the evidence is scarcer,
/// since a ΔT minute needs *both* archives to have produced a reading.
/// Keeping the two separate means tightening one later does not silently
/// move the other.
pub const COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES: u32 = 30;

/// How far forward a non-idle band's baseline side may extend past the
/// pinned window, in inclusive calendar days counted from the window's
/// start (#2333): the extended window ends at the latest on
/// `baseline_start + 29`.
///
/// The pinned window is the first seven days with enough *idle* minutes,
/// and it is fixed forever once pinned - that is what makes the idle
/// baseline a reference. But the load bands are not what qualified those
/// days. A machine that idled through its first week recorded a handful of
/// high-band minutes in it, and because the window never moves, the high
/// band could never become comparable for the lifetime of the install
/// (observed: 12 high-band minutes in a 13-day pinned window, reported as
/// "not comparable" for good). So each of the low, mid and high bands
/// keeps the pinned window's start and walks forward from the pinned
/// window's end, day by day, until its own minutes reach
/// [`COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES`]. Forward, never
/// backward: the days before the pinned window are the ones that did not
/// qualify, and an "earlier" baseline would be anchored to nothing the
/// user can see.
///
/// The cap keeps the word "baseline" honest. A band that took three months
/// to accrue 30 minutes was not a baseline-era observation, it is just the
/// first time the user ran that load - and without a cap, the extended
/// window would eventually overlap the recent one and compare a period
/// with itself. Thirty days is long enough to absorb the common case (a
/// bursty band that fills in within a few weeks of the install) and short
/// enough that a band still thin at the cap is reported as what it is:
/// [`BandComparability::TooFewSampleMinutes`], with its real counts, over
/// `[baseline_start, cap]`. Never extended into the recent window either:
/// the walk stops the day before it starts, so the baseline and recent
/// sides of one comparison never read the same day. A day with no rollup
/// row contributes nothing.
///
/// The retention cleanup exempts the same `[baseline_start, cap]` range
/// (see [`baseline_extension_cap_end`]), because an extended baseline side
/// that lost its rows to retention would drift exactly the way pinning
/// exists to prevent.
pub const COOLING_BAND_BASELINE_EXTENSION_MAX_CALENDAR_DAYS: u32 = 30;

/// The last calendar day a band's extended baseline side may reach, for a
/// pinned window `[baseline_start, baseline_end]`: `baseline_start` plus
/// [`COOLING_BAND_BASELINE_EXTENSION_MAX_CALENDAR_DAYS`]` - 1`, or the
/// pinned window's own end if the seven qualifying days were spread wider
/// than that (the cap bounds the *extension*, it never shortens the pinned
/// window itself).
///
/// Shared with the rollup's retention cleanup so the rows an extended
/// baseline side reads and the rows cleanup refuses to delete are one
/// range by construction.
pub fn baseline_extension_cap_end(
  baseline_start: NaiveDate,
  baseline_end: NaiveDate,
) -> NaiveDate {
  let cap = baseline_start
    + Duration::days(COOLING_BAND_BASELINE_EXTENSION_MAX_CALENDAR_DAYS as i64 - 1);
  baseline_end.max(cap)
}

/// Whether one band's two windows can be compared, and if not, why.
///
/// A reason rather than a `bool` because the two reasons resolve
/// differently and the UI has to say which one applies: too few sample
/// minutes fills in on its own as the machine keeps running, and the panel
/// can say how many minutes are still missing, while a different ambient
/// sensor never does - no amount of waiting makes two placements one. A
/// `bool` presented both as "not enough samples", which is wrong for the
/// second and was observed misleading a user whose sensor had changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandComparability {
  Comparable,
  /// One or both windows carry fewer sample minutes than the band's
  /// minimum (DP-02: no delta computed from a handful of minutes as if
  /// it were a measurement). Which side is short is read off the two
  /// window summaries beside it.
  TooFewSampleMinutes,
  /// The recent window's ambient source is not the one the ΔT baseline
  /// was established from (#2062). Takes precedence over
  /// [`Self::TooFewSampleMinutes`]: a thin window will thicken, a changed
  /// sensor will not, so the reason that does not resolve is the one to
  /// report.
  DifferentAmbientSource,
}

impl BandComparability {
  /// The both-sides-or-nothing rule shared by the absolute and the
  /// ambient-adjusted readings: a band comparable on one side but not the
  /// other is still not comparable overall.
  fn from_sides(baseline_sufficient: bool, recent_sufficient: bool) -> Self {
    if baseline_sufficient && recent_sufficient {
      Self::Comparable
    } else {
      Self::TooFewSampleMinutes
    }
  }

  pub fn is_comparable(self) -> bool {
    self == Self::Comparable
  }
}

/// One band's sample-minute-weighted temperature over some date window.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BandWindowSummary {
  pub temperature_avg: Option<f32>,
  pub sample_minutes: u32,
}

impl BandWindowSummary {
  fn is_comparable(&self) -> bool {
    self.sample_minutes >= COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES
  }
}

/// One band's sample-minute-weighted ΔT over some date window (#2045).
///
/// The value is a *difference* (CPU package temperature minus ambient), so
/// it is named apart from [`BandWindowSummary::temperature_avg`]: mixing
/// the two up at a call site would silently compare an absolute
/// temperature against a delta.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BandDeltaWindowSummary {
  pub delta_avg: Option<f32>,
  pub sample_minutes: u32,
}

impl BandDeltaWindowSummary {
  pub(crate) fn is_comparable(&self) -> bool {
    self.sample_minutes >= COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES
  }
}

/// One band's ambient-adjusted baseline-vs-recent comparison (#2045):
/// the same two windows as [`BandComparison`], but over ΔT instead of
/// absolute temperature, so a rise the weather explains and a rise the
/// cooling explains can be told apart.
///
/// Subtracting `baseline.delta_avg` from `recent.delta_avg` (so a rise
/// reads positive, matching every other delta in Cooling Insight) is
/// legitimate where subtracting a CPU summary from an ambient summary is
/// not: both sides here are already per-minute ΔT values that were paired
/// before aggregation, so this compares one period against another rather
/// than reconstructing a pairing that never happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientAdjustedBandComparison {
  /// The ΔT baseline's own source over this band's baseline side: the ΔT
  /// baseline's pinned window for the idle band, that window extended
  /// forward over the same source's rows for the other bands (#2333).
  pub baseline: BandDeltaWindowSummary,
  /// The calendar range `baseline` was read over, inclusive. Starts at the
  /// ΔT baseline's pinned start; ends at its pinned end for idle, and for
  /// the other bands at whichever day their paired minutes reached
  /// [`COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES`] (or at the cap).
  pub baseline_window_start_date: NaiveDate,
  pub baseline_window_end_date: NaiveDate,
  /// The recent window's ΔT, read from whichever ambient source covered
  /// the most of it (see [`dominant_delta_source`]).
  pub recent: BandDeltaWindowSummary,
  /// Whether both windows carry enough paired minutes for the
  /// ambient-adjusted reading to mean anything, on the same
  /// both-sides-or-nothing rule as [`BandComparison::comparability`] - and
  /// whether they were measured against the *same* sensor (#2062). A
  /// recent window from a different source than the baseline's is
  /// reported but never compared: the two are different quantities, not
  /// a drift.
  pub comparability: BandComparability,
}

/// One [`CpuLoadBand`]'s baseline-vs-recent comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandComparison {
  pub band: CpuLoadBand,
  /// This band's baseline side: the pinned window for idle, and for the
  /// other bands the pinned window extended forward until it held enough
  /// of their minutes (#2333) - see `baseline_window_*_date` below.
  pub baseline: BandWindowSummary,
  /// The calendar range `baseline` was read over, inclusive. Always starts
  /// at the pinned window's start. For idle it ends at the pinned end; for
  /// low, mid and high it ends at the first day on or after the pinned end
  /// at which the band's minutes reached
  /// [`COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES`], or at
  /// [`baseline_extension_cap_end`] when they never did - in which case
  /// `comparability` says so.
  pub baseline_window_start_date: NaiveDate,
  pub baseline_window_end_date: NaiveDate,
  pub recent: BandWindowSummary,
  /// Whether both windows carry enough evidence for `recent` minus
  /// `baseline` to mean anything. Anything but `Comparable` means present
  /// the reason rather than a number, even though both fields above still
  /// carry whatever (insufficient) data was found.
  pub comparability: BandComparability,
  /// The ambient-adjusted reading of the same two windows (#2045), or
  /// `None` when neither window recorded a single ΔT minute for this
  /// band.
  ///
  /// `None` and `Some(..)` with a non-comparable reason say different
  /// things on purpose. `None` means this machine has no ambient evidence
  /// here at all - the normal state on an install with no environmental
  /// sensor, and what keeps every ambient-unaware reading of this response
  /// exactly what it was before #2045. `Some` that is not comparable means
  /// ambient data exists but one window is too thin to compare, or was
  /// measured against another sensor, which is worth telling the user
  /// about because the former will resolve on its own.
  pub ambient_adjusted: Option<AmbientAdjustedBandComparison>,
}

/// Cooling Insight's load-band comparison, gated by the same baseline
/// lifecycle as [`crate::persistence::cooling_baseline::CoolingBaseline`].
#[derive(Debug, Clone, PartialEq)]
pub enum CoolingBandComparison {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    baseline_window_start_date: NaiveDate,
    baseline_window_end_date: NaiveDate,
    recent_window_start_date: NaiveDate,
    recent_window_end_date: NaiveDate,
    /// Boxed because the array dwarfs the `Establishing` variant, which
    /// makes every value of this enum pay for the larger one. Still a
    /// fixed-size `[_; 4]` rather than a `Vec`: there are exactly four
    /// bands and the type should keep saying so. One allocation per
    /// query, on a path that has just read the whole daily table.
    bands: Box<[BandComparison; 4]>,
    /// The ΔT baseline's lifecycle (#2045), which advances independently
    /// of the absolute one this variant's window dates describe.
    ///
    /// One fact for all four bands rather than a copy on each: the
    /// window is a property of the baseline, not of a band. While this
    /// is `Establishing`, every band's `ambient_adjusted` is `None` -
    /// there is no reference window to compare against yet, the same way
    /// the absolute comparison withholds every band while its own
    /// baseline establishes.
    ambient_adjusted_baseline: DeltaBaselineState,
  },
}

/// Derive the load-band comparison from every completed day's rollup row
/// and the current baseline lifecycle states.
///
/// `window_end_date` is the most recent completed local day (yesterday),
/// matching [`crate::persistence::cooling_baseline::derive_cooling_baseline`].
/// The recent window ending there is the one
/// [`recent_window_start`] defines from `days`' dates - the hardware
/// rollup's recorded days - and the ambient-adjusted readings below use
/// that same window rather than one of their own, so every band and both
/// readings of one response describe the same stretch of days.
///
/// `delta_days` carries the row-per-source Thermal Delta rollup the
/// ambient-adjusted readings are built from (#2045, #2062), and
/// `delta_baseline_state` is a second, independent lifecycle: the ΔT
/// baseline establishes over its own window, which is generally later
/// than the absolute one on any machine that added an ambient sensor
/// after it had been running for a while.
pub fn derive_band_comparison(
  days: &[DailyCoolingSummary],
  delta_days: &[ThermalDeltaDailySummary],
  baseline_state: BaselineState,
  delta_baseline_state: DeltaBaselineState,
  window_end_date: NaiveDate,
) -> CoolingBandComparison {
  let (baseline_start, baseline_end) = match baseline_state {
    BaselineState::Establishing {
      qualifying_days,
      required_days,
    } => {
      return CoolingBandComparison::Establishing {
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
    recent_window_start(days.iter().map(|day| day.date), window_end_date);
  // An extended baseline side stops the day before the recent window
  // starts, so the two sides of one comparison never share a day: a band
  // that only fills in during the recent window would otherwise be
  // compared partly with itself.
  let extension_ceiling = recent_start - Duration::days(1);
  // Note the *ΔT* baseline's own source and window, not the absolute
  // window above: the two are different date ranges whenever ambient
  // collection started later than the machine did.
  let delta_reference = match &delta_baseline_state {
    DeltaBaselineState::Established {
      source,
      window_start_date,
      window_end_date,
      ..
    } => Some((source.as_str(), (*window_start_date, *window_end_date))),
    DeltaBaselineState::Establishing { .. } => None,
  };
  let recent_delta_source =
    dominant_delta_source(delta_days, recent_start, window_end_date);

  let bands = [
    CpuLoadBand::Idle,
    CpuLoadBand::Low,
    CpuLoadBand::Mid,
    CpuLoadBand::High,
  ]
  .map(|band| {
    // The pinned window for idle; for the other bands, the pinned window
    // extended forward until it holds enough of their minutes (#2333).
    let (band_baseline_start, band_baseline_end) = band_baseline_window(
      days,
      band,
      (baseline_start, baseline_end),
      extension_ceiling,
    );
    let baseline =
      band_window_summary(days, band, band_baseline_start, band_baseline_end);
    let recent = band_window_summary(days, band, recent_start, window_end_date);
    let comparability =
      BandComparability::from_sides(baseline.is_comparable(), recent.is_comparable());
    BandComparison {
      band,
      baseline,
      baseline_window_start_date: band_baseline_start,
      baseline_window_end_date: band_baseline_end,
      recent,
      comparability,
      ambient_adjusted: delta_reference.and_then(|(baseline_source, baseline_window)| {
        ambient_adjusted_band_comparison(
          delta_days,
          band,
          (baseline_source, baseline_window),
          (recent_delta_source, (recent_start, window_end_date)),
          extension_ceiling,
        )
      }),
    }
  });

  CoolingBandComparison::Established {
    baseline_window_start_date: baseline_start,
    baseline_window_end_date: baseline_end,
    recent_window_start_date: recent_start,
    recent_window_end_date: window_end_date,
    bands: Box::new(bands),
    ambient_adjusted_baseline: delta_baseline_state,
  }
}

fn band_summary_for(day: &DailyCoolingSummary, band: CpuLoadBand) -> &BandSummary {
  match band {
    CpuLoadBand::Idle => &day.idle,
    CpuLoadBand::Low => &day.low,
    CpuLoadBand::Mid => &day.mid,
    CpuLoadBand::High => &day.high,
  }
}

/// The last day of a band's baseline side, walked forward from the pinned
/// window `[pinned_start, pinned_end]` over `day_minutes` - that band's
/// `(date, sample minutes)` per rollup row - until the cumulative count
/// reaches `minimum_sample_minutes` (#2333).
///
/// The rule in order: a pinned window that already holds enough stops at
/// `pinned_end` (nothing moves for a band that was fine). Otherwise the
/// days after `pinned_end` are taken in date order and the first one at
/// which the running total reaches the minimum ends the window. A band
/// that never gets there ends at the limit: [`baseline_extension_cap_end`]
/// of the pinned window, or `extension_ceiling` - the day before the
/// recent window starts - when that comes sooner, so an extended
/// baseline never reads a day the recent side also summarizes. A pinned
/// window that already reaches the ceiling is not extended at all.
///
/// Rows are bucketed by date rather than trusted to arrive sorted, so the
/// answer does not depend on the order a query happened to return them
/// in; a date with no row is simply absent from the walk and contributes
/// nothing, which is what "no evidence" should contribute.
fn extended_baseline_end(
  day_minutes: impl IntoIterator<Item = (NaiveDate, u32)>,
  minimum_sample_minutes: u32,
  (pinned_start, pinned_end): (NaiveDate, NaiveDate),
  extension_ceiling: NaiveDate,
) -> NaiveDate {
  let limit = baseline_extension_cap_end(pinned_start, pinned_end)
    .min(extension_ceiling)
    .max(pinned_end);
  let minimum = minimum_sample_minutes as u64;

  let mut minutes_by_date: BTreeMap<NaiveDate, u64> = BTreeMap::new();
  for (date, minutes) in day_minutes {
    if date >= pinned_start && date <= limit {
      *minutes_by_date.entry(date).or_default() += minutes as u64;
    }
  }

  let mut cumulative: u64 = minutes_by_date.range(..=pinned_end).map(|(_, m)| m).sum();
  if cumulative >= minimum {
    return pinned_end;
  }
  for (date, minutes) in
    minutes_by_date.range((Bound::Excluded(pinned_end), Bound::Included(limit)))
  {
    cumulative += minutes;
    if cumulative >= minimum {
      return *date;
    }
  }
  limit
}

/// The calendar range `band`'s absolute baseline side is read over: the
/// pinned window itself for idle, whose minutes are what qualified those
/// days in the first place, and for every other band the pinned window
/// extended forward per [`extended_baseline_end`] (#2333).
///
/// Only days that carry an average for the band count toward the walk,
/// matching what [`band_window_summary`] will then sum - so a window this
/// returns as "enough" is one the summary will also find enough.
fn band_baseline_window(
  days: &[DailyCoolingSummary],
  band: CpuLoadBand,
  pinned: (NaiveDate, NaiveDate),
  extension_ceiling: NaiveDate,
) -> (NaiveDate, NaiveDate) {
  if band == CpuLoadBand::Idle {
    return pinned;
  }
  let end = extended_baseline_end(
    days.iter().filter_map(|day| {
      let summary = band_summary_for(day, band);
      summary.avg.map(|_| (day.date, summary.sample_minutes))
    }),
    COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES,
    pinned,
    extension_ceiling,
  );
  (pinned.0, end)
}

/// [`band_baseline_window`] for the ambient-adjusted side: the same walk
/// over the ΔT baseline's own pinned window, counting `source`'s rows only
/// (#2333). Another sensor's minutes must not carry a band over the
/// threshold any more than they may enter its average - the window a
/// source's ΔT is read over has to be one that source itself filled.
fn band_delta_baseline_window(
  days: &[ThermalDeltaDailySummary],
  source: &str,
  band: CpuLoadBand,
  pinned: (NaiveDate, NaiveDate),
  extension_ceiling: NaiveDate,
) -> (NaiveDate, NaiveDate) {
  if band == CpuLoadBand::Idle {
    return pinned;
  }
  let end = extended_baseline_end(
    days
      .iter()
      .filter(|day| day.source == source)
      .filter_map(|day| {
        let summary = day.band(band);
        summary.avg.map(|_| (day.date, summary.sample_minutes))
      }),
    COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES,
    pinned,
    extension_ceiling,
  );
  (pinned.0, end)
}

/// Sample-minute-weighted average temperature for `band` across
/// `[start, end]` (inclusive). Mirrors
/// `cooling_baseline::weighted_idle_temperature`'s weighting rule, just
/// generalized to any band instead of only idle.
fn band_window_summary(
  days: &[DailyCoolingSummary],
  band: CpuLoadBand,
  start: NaiveDate,
  end: NaiveDate,
) -> BandWindowSummary {
  let mut weighted_sum = 0.0f64;
  let mut sample_minutes: u64 = 0;

  for day in days.iter().filter(|d| d.date >= start && d.date <= end) {
    let summary = band_summary_for(day, band);
    let Some(avg) = summary.avg else { continue };
    weighted_sum += avg as f64 * summary.sample_minutes as f64;
    sample_minutes += summary.sample_minutes as u64;
  }

  BandWindowSummary {
    temperature_avg: (sample_minutes > 0)
      .then(|| (weighted_sum / sample_minutes as f64) as f32),
    sample_minutes: sample_minutes as u32,
  }
}

/// [`band_window_summary`] over one ambient source's ΔT rows instead of
/// the day's absolute temperature bands (#2045). Same sample-minute
/// weighting - the ΔT band's own `sample_minutes`, which counts only the
/// minutes that carried both readings, so a day whose ambient sensor
/// dropped out for half the day weighs exactly the half it observed.
///
/// Reads `source`'s rows only. Folding two sources' rows into one window
/// would average two sensor placements into a ΔT no sensor observed, so
/// there is deliberately no source-agnostic variant of this function.
pub(crate) fn band_delta_window_summary(
  days: &[ThermalDeltaDailySummary],
  source: &str,
  band: CpuLoadBand,
  start: NaiveDate,
  end: NaiveDate,
) -> BandDeltaWindowSummary {
  let mut weighted_sum = 0.0f64;
  let mut sample_minutes: u64 = 0;

  for day in days
    .iter()
    .filter(|d| d.source == source && d.date >= start && d.date <= end)
  {
    let summary = day.band(band);
    let Some(avg) = summary.avg else { continue };
    weighted_sum += avg as f64 * summary.sample_minutes as f64;
    sample_minutes += summary.sample_minutes as u64;
  }

  BandDeltaWindowSummary {
    delta_avg: (sample_minutes > 0)
      .then(|| (weighted_sum / sample_minutes as f64) as f32),
    sample_minutes: sample_minutes as u32,
  }
}

/// The ambient source with the most paired coverage across `[start, end]`
/// (inclusive), or `None` when no source paired a minute in it.
///
/// This is the MVP rule for which sensor a window is *read* from (#2062):
/// Cooling Insight has no source picker yet, so a window reports the
/// sensor that actually observed most of it, and only that sensor - never
/// a blend. A tie falls to the label that sorts first, so the answer is
/// the same on every read.
pub(crate) fn dominant_delta_source(
  days: &[ThermalDeltaDailySummary],
  start: NaiveDate,
  end: NaiveDate,
) -> Option<&str> {
  let mut coverage_by_source: BTreeMap<&str, u64> = BTreeMap::new();
  for day in days.iter().filter(|d| d.date >= start && d.date <= end) {
    *coverage_by_source.entry(day.source.as_str()).or_default() +=
      day.coverage_minutes as u64;
  }

  // `max_by_key` keeps the *last* maximum, so iterate in reverse to make
  // the tie fall to the first label in `BTreeMap` order.
  coverage_by_source
    .into_iter()
    .rev()
    .max_by_key(|(_, coverage)| *coverage)
    .map(|(source, _)| source)
}

/// The ambient-adjusted reading of one band's two windows, or `None` when
/// neither window recorded a ΔT minute for this band.
///
/// The baseline side is read from the ΔT baseline's own source, over its
/// pinned window for idle and that window extended forward for the other
/// bands (#2333, see [`band_delta_baseline_window`]); the recent side from
/// whichever source dominates the recent window. They are comparable only
/// when both are thick enough *and* are the same source - a sensor change
/// turns "recent minus baseline" into a difference between two placements,
/// which is not a drift in the cooling (#2062).
///
/// The `None` case is the whole reason ambient stays optional: a machine
/// with no environmental sensor produces it for every band, and the
/// response then carries exactly the facts it carried before #2045.
fn ambient_adjusted_band_comparison(
  days: &[ThermalDeltaDailySummary],
  band: CpuLoadBand,
  (baseline_source, pinned_baseline_window): (&str, (NaiveDate, NaiveDate)),
  (recent_source, recent_window): (Option<&str>, (NaiveDate, NaiveDate)),
  extension_ceiling: NaiveDate,
) -> Option<AmbientAdjustedBandComparison> {
  let (baseline_start, baseline_end) = band_delta_baseline_window(
    days,
    baseline_source,
    band,
    pinned_baseline_window,
    extension_ceiling,
  );
  let baseline =
    band_delta_window_summary(days, baseline_source, band, baseline_start, baseline_end);
  let recent = recent_source.map_or_else(BandDeltaWindowSummary::default, |source| {
    band_delta_window_summary(days, source, band, recent_window.0, recent_window.1)
  });

  if baseline.sample_minutes == 0 && recent.sample_minutes == 0 {
    return None;
  }

  // The source check comes first: a changed sensor is the reason that
  // never resolves, so it must not be hidden behind a thin window that
  // will. A recent window with no source at all is simply thin.
  let comparability = match recent_source {
    Some(source) if source != baseline_source => {
      BandComparability::DifferentAmbientSource
    }
    _ => BandComparability::from_sides(baseline.is_comparable(), recent.is_comparable()),
  };

  Some(AmbientAdjustedBandComparison {
    comparability,
    baseline,
    baseline_window_start_date: baseline_start,
    baseline_window_end_date: baseline_end,
    recent,
  })
}

fn to_idle_sample(
  day: &DailyCoolingSummary,
) -> crate::persistence::cooling_baseline::DailyIdleSample {
  crate::persistence::cooling_baseline::DailyIdleSample {
    date: day.date,
    idle_temperature_avg: day.idle.avg,
    idle_sample_minutes: day.idle.sample_minutes,
  }
}

/// [`derive_band_comparison`] over the whole `cooling_daily_summary`
/// table, resolving the baseline lifecycle state through
/// [`crate::persistence::cooling_baseline::resolve_baseline_state_from_pool`]
/// (their idle-band projection, so no second query) rather than
/// re-deriving it - the pinned baseline row must win once one exists, or
/// this comparison would silently drift once the rollup rows the
/// original establishment came from age out.
/// Test-only since #2134: [`load_cooling_band_comparison`] is routed
/// through the dispatch boundary instead of an explicit pool.
#[cfg(test)]
pub(crate) async fn load_cooling_band_comparison_from_pool(
  pool: &sqlx::SqlitePool,
  today: NaiveDate,
) -> Result<CoolingBandComparison, sqlx::Error> {
  use crate::infrastructure::database;
  use crate::persistence::cooling_baseline::resolve_baseline_state_from_pool;

  let days =
    database::cooling_daily_summary::select_all_daily_cooling_summaries_from_pool(pool)
      .await?;
  let idle_samples: Vec<_> = days.iter().map(to_idle_sample).collect();
  let baseline_state = resolve_baseline_state_from_pool(pool, &idle_samples).await?;
  // The Thermal Delta lives in its own row-per-source table (#2062), read
  // whole for the same reason the daily table is.
  let delta_days =
    database::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries_from_pool(
      pool,
    )
    .await?;
  // Resolved through its own resolver against its own pinned row, for
  // the same reason the absolute one is: re-deriving would drift once
  // the establishment window's rows age out.
  let delta_baseline_state =
    crate::persistence::cooling_delta_baseline::resolve_delta_baseline_state_from_pool(
      pool,
      &delta_days,
    )
    .await?;
  let yesterday = today - Duration::days(1);

  Ok(derive_band_comparison(
    &days,
    &delta_days,
    baseline_state,
    delta_baseline_state,
    yesterday,
  ))
}

/// [`load_cooling_band_comparison_from_pool`], routed through the dispatch
/// boundary (#2134) instead of Core's process-wide SQLite pool.
pub async fn load_cooling_band_comparison()
-> Result<CoolingBandComparison, crate::infrastructure::database::dispatch::DispatchError>
{
  use crate::infrastructure::database::dispatch;
  use crate::persistence::cooling_baseline::resolve_baseline_state;
  use crate::persistence::cooling_delta_baseline::resolve_delta_baseline_state;

  let days =
    dispatch::cooling_daily_summary::select_all_daily_cooling_summaries().await?;
  let idle_samples: Vec<_> = days.iter().map(to_idle_sample).collect();
  let baseline_state = resolve_baseline_state(&idle_samples).await?;
  let delta_days =
    dispatch::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries()
      .await?;
  let delta_baseline_state = resolve_delta_baseline_state(&delta_days).await?;
  let yesterday = chrono::Local::now().date_naive() - Duration::days(1);

  Ok(derive_band_comparison(
    &days,
    &delta_days,
    baseline_state,
    delta_baseline_state,
    yesterday,
  ))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::test_schema::{
    COOLING_BASELINE_DDL, COOLING_DAILY_SUMMARY_DDL, COOLING_DELTA_BASELINE_DDL,
    COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL, create_tables,
  };
  use crate::persistence::cooling_baseline::{
    COOLING_BASELINE_QUALIFYING_IDLE_MINUTES, COOLING_BASELINE_RECENT_WINDOW_DAYS,
  };

  // Fixtures place the baseline days in early August and end the recent
  // window on 2026-09-20: more than the recent window's calendar bound
  // (`COOLING_BASELINE_RECENT_WINDOW_MAX_CALENDAR_DAYS`) later, so a
  // baseline day is never also a recorded day of the recent window, and
  // `recent_start` below is the start the window has when its only
  // recorded day is `recent_start` itself.

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn band(avg: f32, minutes: u32) -> BandSummary {
    BandSummary {
      avg: Some(avg),
      max: Some(avg + 1.0),
      min: Some(avg - 1.0),
      sample_minutes: minutes,
    }
  }

  fn empty_band() -> BandSummary {
    BandSummary::default()
  }

  fn established_baseline(start: NaiveDate, end: NaiveDate) -> BaselineState {
    BaselineState::Established {
      idle_temperature_avg: 35.0,
      window_start_date: start,
      window_end_date: end,
      sample_minutes: 210,
    }
  }

  /// A ΔT baseline established from `source` over `[start, end]`.
  /// Deliberately taken as its own dates rather than reusing the absolute
  /// baseline's: the two windows differ on any machine whose ambient
  /// sensor arrived late, and these tests should be able to say so.
  fn established_delta_baseline(
    source: &str,
    start: NaiveDate,
    end: NaiveDate,
  ) -> DeltaBaselineState {
    DeltaBaselineState::Established {
      source: source.to_string(),
      delta_temperature_avg: 10.0,
      window_start_date: start,
      window_end_date: end,
      sample_minutes: 210,
    }
  }

  /// The ΔT baseline of a machine that has no ambient data at all.
  fn establishing_delta_baseline() -> DeltaBaselineState {
    DeltaBaselineState::Establishing {
      qualifying_days: 0,
      required_days: 7,
    }
  }

  /// One day carrying only an idle temperature band.
  fn idle_day(date: NaiveDate, temperature: f32, minutes: u32) -> DailyCoolingSummary {
    DailyCoolingSummary {
      date,
      coverage_minutes: 1440,
      idle: band(temperature, minutes),
      low: empty_band(),
      mid: empty_band(),
      high: empty_band(),
      power: PowerSummary::default(),
    }
  }

  /// One source's ΔT row carrying only an idle band (#2045, #2062).
  fn idle_delta_day(
    date: NaiveDate,
    source: &str,
    delta: f32,
    minutes: u32,
  ) -> ThermalDeltaDailySummary {
    ThermalDeltaDailySummary {
      date,
      source: source.to_string(),
      coverage_minutes: minutes,
      idle: band(delta, minutes),
      low: empty_band(),
      mid: empty_band(),
      high: empty_band(),
    }
  }

  /// The idle band's comparison out of an established result.
  fn idle_comparison(result: CoolingBandComparison) -> BandComparison {
    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };
    *bands
      .iter()
      .find(|b| b.band == CpuLoadBand::Idle)
      .expect("the idle band is always present")
  }

  // ── ambient-adjusted comparison (#2045) ──

  #[test]
  fn a_machine_with_no_ambient_data_offers_no_ambient_adjusted_reading() {
    // The invariant that keeps ambient optional: with no ΔT anywhere,
    // every band reports `None` and every other field is exactly what it
    // was before #2045.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 50.0, 60),
    ];

    let result = derive_band_comparison(
      &days,
      &[],
      established_baseline(baseline_start, baseline_start),
      establishing_delta_baseline(),
      recent_end,
    );

    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };
    for comparison in bands.iter() {
      assert_eq!(
        comparison.ambient_adjusted, None,
        "band {:?} must offer no ambient-adjusted reading",
        comparison.band
      );
    }
    // And the absolute reading is untouched.
    let idle = bands.iter().find(|b| b.band == CpuLoadBand::Idle).unwrap();
    assert_eq!(idle.baseline.temperature_avg, Some(30.0));
    assert_eq!(idle.recent.temperature_avg, Some(50.0));
    assert_eq!(idle.comparability, BandComparability::Comparable);
  }

  #[test]
  fn an_ambient_adjusted_reading_separates_a_warmer_room_from_worse_cooling() {
    // The whole point of the feature. The absolute idle temperature rose
    // 20 K between the windows, but the ΔT held flat: the room got
    // warmer, the cooling did not degrade.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 50.0, 60),
    ];
    let delta_days = vec![
      idle_delta_day(baseline_start, "Desk", 10.0, 60),
      idle_delta_day(recent_start, "Desk", 10.0, 60),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Desk", baseline_start, baseline_start),
      recent_end,
    ));

    assert_eq!(idle.recent.temperature_avg, Some(50.0));
    assert_eq!(idle.baseline.temperature_avg, Some(30.0));
    let adjusted = idle.ambient_adjusted.expect("ambient data exists");
    assert_eq!(adjusted.baseline.delta_avg, Some(10.0));
    assert_eq!(adjusted.recent.delta_avg, Some(10.0));
    assert_eq!(adjusted.comparability, BandComparability::Comparable);
  }

  #[test]
  fn each_ambient_adjusted_window_is_weighted_by_its_own_delta_sample_minutes() {
    // The ΔT band's own sample minutes do the weighting, not the
    // temperature band's: a day whose ambient sensor covered only part of
    // the day must weigh exactly the part it observed.
    let baseline_start = date(2026, 8, 1);
    let baseline_end = date(2026, 8, 7);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 1440),
      idle_day(baseline_end, 30.0, 1440),
      idle_day(recent_start, 50.0, 1440),
    ];
    let delta_days = vec![
      // 1440 temperature minutes but only 30 of them paired.
      idle_delta_day(baseline_start, "Desk", 8.0, 30),
      idle_delta_day(baseline_end, "Desk", 12.0, 90),
      idle_delta_day(recent_start, "Desk", 20.0, 60),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_end),
      established_delta_baseline("Desk", baseline_start, baseline_end),
      recent_end,
    ));

    let adjusted = idle.ambient_adjusted.expect("ambient data exists");
    assert_eq!(adjusted.baseline.sample_minutes, 120);
    let expected = (8.0 * 30.0 + 12.0 * 90.0) / 120.0;
    assert!((adjusted.baseline.delta_avg.unwrap() - expected).abs() < 0.001);
    assert_eq!(adjusted.recent.sample_minutes, 60);
    assert_eq!(adjusted.recent.delta_avg, Some(20.0));
  }

  #[test]
  fn an_ambient_adjusted_window_one_minute_short_is_present_but_not_comparable() {
    // Distinct from `None`: ambient evidence exists, there is just not
    // enough of it yet. That is worth saying, because it resolves on its
    // own as coverage accrues.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let short = COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES - 1;
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 50.0, 60),
    ];
    let delta_days = vec![
      idle_delta_day(baseline_start, "Desk", 10.0, short),
      idle_delta_day(
        recent_start,
        "Desk",
        14.0,
        COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES,
      ),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Desk", baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle
      .ambient_adjusted
      .expect("ambient evidence exists, it is merely thin");
    assert_eq!(
      adjusted.comparability,
      BandComparability::TooFewSampleMinutes
    );
    // The (insufficient) evidence is still reported, matching how the
    // absolute comparison behaves.
    assert_eq!(adjusted.baseline.sample_minutes, short);
    assert_eq!(adjusted.baseline.delta_avg, Some(10.0));
  }

  #[test]
  fn ambient_on_only_one_side_is_reported_rather_than_hidden() {
    // Ambient collection that started partway through: the recent window
    // has ΔT and the baseline window never will. Not comparable, but not
    // absent either - the user can see why.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 50.0, 60),
    ];
    let delta_days = vec![idle_delta_day(recent_start, "Desk", 14.0, 60)];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Desk", baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle.ambient_adjusted.expect("the recent side has ambient");
    assert_eq!(
      adjusted.comparability,
      BandComparability::TooFewSampleMinutes
    );
    assert_eq!(adjusted.baseline.sample_minutes, 0);
    assert_eq!(adjusted.baseline.delta_avg, None);
    assert_eq!(adjusted.recent.delta_avg, Some(14.0));
  }

  #[test]
  fn a_recent_window_from_a_different_source_than_the_baseline_is_not_comparable() {
    // The user switched sensors after the baseline was pinned against the
    // desk one. The recent window is rich, and it is reported - but
    // "recent minus baseline" would be the difference between two
    // placements, not a drift in the cooling (#2062).
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 30.0, 60),
    ];
    let delta_days = vec![
      idle_delta_day(baseline_start, "Desk", 10.0, 600),
      idle_delta_day(recent_start, "Living Room", 13.0, 600),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Desk", baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle.ambient_adjusted.expect("both sides have ambient");
    assert_eq!(adjusted.baseline.delta_avg, Some(10.0));
    assert_eq!(adjusted.recent.delta_avg, Some(13.0));
    assert_eq!(
      adjusted.comparability,
      BandComparability::DifferentAmbientSource,
      "two sensor placements must never be compared as if they were one"
    );
  }

  #[test]
  fn a_changed_sensor_is_reported_ahead_of_a_thin_window() {
    // Both reasons hold at once: the recent window is from another sensor
    // *and* thin. The sensor is the one to report - waiting fixes the
    // thinness, nothing fixes the placement.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 30.0, 60),
    ];
    let delta_days = vec![
      idle_delta_day(baseline_start, "Desk", 10.0, 600),
      idle_delta_day(
        recent_start,
        "Living Room",
        13.0,
        COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES - 1,
      ),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Desk", baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle.ambient_adjusted.expect("both sides have ambient");
    assert_eq!(
      adjusted.comparability,
      BandComparability::DifferentAmbientSource
    );
  }

  #[test]
  fn the_baseline_window_reads_only_the_baselines_own_source() {
    // Both sensors archived the baseline window. The baseline side must
    // be the pinned source's rows alone, never a blend of the two.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 30.0, 60),
    ];
    let delta_days = vec![
      idle_delta_day(baseline_start, "Desk", 10.0, 600),
      idle_delta_day(baseline_start, "Living Room", 16.0, 600),
      idle_delta_day(recent_start, "Desk", 11.0, 600),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Desk", baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle.ambient_adjusted.expect("both sides have ambient");
    assert_eq!(adjusted.baseline.delta_avg, Some(10.0));
    assert_eq!(adjusted.recent.delta_avg, Some(11.0));
    assert_eq!(adjusted.comparability, BandComparability::Comparable);
  }

  #[test]
  fn the_recent_window_reads_the_source_with_the_most_coverage() {
    // Two sensors overlap in the recent window; the one that observed
    // more of it is the one reported, and only it.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60),
      idle_day(recent_start, 30.0, 60),
    ];
    let delta_days = vec![
      idle_delta_day(baseline_start, "Living Room", 10.0, 600),
      idle_delta_day(recent_start, "Desk", 30.0, 100),
      idle_delta_day(recent_start, "Living Room", 12.0, 900),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Living Room", baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle.ambient_adjusted.expect("both sides have ambient");
    assert_eq!(adjusted.recent.delta_avg, Some(12.0));
    assert_eq!(adjusted.recent.sample_minutes, 900);
    assert_eq!(adjusted.comparability, BandComparability::Comparable);
  }

  #[test]
  fn a_coverage_tie_between_sources_falls_to_the_same_label_every_time() {
    let start = date(2026, 8, 14);
    let delta_days = vec![
      idle_delta_day(start, "Living Room", 12.0, 600),
      idle_delta_day(start, "Desk", 30.0, 600),
    ];

    assert_eq!(
      dominant_delta_source(&delta_days, start, start),
      Some("Desk"),
      "a tie must be broken deterministically, by label order"
    );
    assert_eq!(dominant_delta_source(&[], start, start), None);
  }

  #[test]
  fn an_ambient_adjusted_reading_stays_within_its_own_band() {
    // A ΔT recorded in the high band must not surface on the idle band's
    // ambient-adjusted reading.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let high_only = |date: NaiveDate| DailyCoolingSummary {
      date,
      coverage_minutes: 1440,
      idle: empty_band(),
      low: empty_band(),
      mid: empty_band(),
      high: band(70.0, 60),
      power: PowerSummary::default(),
    };
    let high_only_delta = |date: NaiveDate, delta: f32| ThermalDeltaDailySummary {
      date,
      source: "Desk".to_string(),
      coverage_minutes: 60,
      idle: empty_band(),
      low: empty_band(),
      mid: empty_band(),
      high: band(delta, 60),
    };
    let days = vec![high_only(baseline_start), high_only(recent_start)];
    let delta_days = vec![
      high_only_delta(baseline_start, 40.0),
      high_only_delta(recent_start, 45.0),
    ];

    let result = derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(baseline_start, baseline_start),
      established_delta_baseline("Desk", baseline_start, baseline_start),
      recent_end,
    );
    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };

    let idle = bands.iter().find(|b| b.band == CpuLoadBand::Idle).unwrap();
    assert_eq!(idle.ambient_adjusted, None);
    let high = bands.iter().find(|b| b.band == CpuLoadBand::High).unwrap();
    let adjusted = high.ambient_adjusted.expect("the high band has ambient");
    assert_eq!(adjusted.baseline.delta_avg, Some(40.0));
    assert_eq!(adjusted.recent.delta_avg, Some(45.0));
  }

  #[test]
  fn an_unestablished_baseline_reports_establishing_for_every_band() {
    let result = derive_band_comparison(
      &[],
      &[],
      BaselineState::Establishing {
        qualifying_days: 2,
        required_days: 7,
      },
      establishing_delta_baseline(),
      date(2026, 8, 20),
    );

    assert_eq!(
      result,
      CoolingBandComparison::Establishing {
        qualifying_days: 2,
        required_days: 7,
      }
    );
  }

  #[test]
  fn each_band_is_weighted_by_its_own_sample_minutes_in_each_window() {
    let baseline_start = date(2026, 8, 1);
    let baseline_end = date(2026, 8, 7);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);

    let days = vec![
      DailyCoolingSummary {
        date: baseline_start,
        coverage_minutes: 1440,
        idle: band(30.0, 60),
        low: band(40.0, 30),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
      },
      DailyCoolingSummary {
        date: baseline_end,
        coverage_minutes: 1440,
        idle: band(32.0, 60),
        low: band(42.0, 90),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_start,
        coverage_minutes: 1440,
        idle: band(50.0, 60),
        low: empty_band(),
        mid: empty_band(),
        high: band(70.0, 40),
        power: PowerSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_end,
        coverage_minutes: 1440,
        idle: band(52.0, 60),
        low: empty_band(),
        mid: empty_band(),
        high: band(72.0, 20),
        power: PowerSummary::default(),
      },
    ];

    let result = derive_band_comparison(
      &days,
      &[],
      established_baseline(baseline_start, baseline_end),
      establishing_delta_baseline(),
      recent_end,
    );

    match result {
      CoolingBandComparison::Established {
        baseline_window_start_date,
        baseline_window_end_date,
        recent_window_start_date,
        recent_window_end_date,
        bands,
        ..
      } => {
        assert_eq!(baseline_window_start_date, baseline_start);
        assert_eq!(baseline_window_end_date, baseline_end);
        assert_eq!(recent_window_start_date, recent_start);
        assert_eq!(recent_window_end_date, recent_end);

        let idle = bands.iter().find(|b| b.band == CpuLoadBand::Idle).unwrap();
        assert_eq!(idle.baseline.sample_minutes, 120);
        assert!((idle.baseline.temperature_avg.unwrap() - 31.0).abs() < 0.001);
        assert_eq!(idle.recent.sample_minutes, 120);
        assert!((idle.recent.temperature_avg.unwrap() - 51.0).abs() < 0.001);
        assert_eq!(idle.comparability, BandComparability::Comparable);

        let low = bands.iter().find(|b| b.band == CpuLoadBand::Low).unwrap();
        assert_eq!(low.baseline.sample_minutes, 120);
        let expected_low = (40.0 * 30.0 + 42.0 * 90.0) / 120.0;
        assert!((low.baseline.temperature_avg.unwrap() - expected_low).abs() < 0.001);
        assert_eq!(low.recent.sample_minutes, 0);
        assert_eq!(low.recent.temperature_avg, None);
        assert_eq!(
          low.comparability,
          BandComparability::TooFewSampleMinutes,
          "no recent low-band evidence at all"
        );

        let mid = bands.iter().find(|b| b.band == CpuLoadBand::Mid).unwrap();
        assert_eq!(mid.baseline.sample_minutes, 0);
        assert_eq!(mid.recent.sample_minutes, 0);
        assert_eq!(mid.comparability, BandComparability::TooFewSampleMinutes);

        let high = bands.iter().find(|b| b.band == CpuLoadBand::High).unwrap();
        assert_eq!(
          (
            high.baseline_window_start_date,
            high.baseline_window_end_date
          ),
          (baseline_start, date(2026, 8, 30)),
          "nothing to find, so the window ran to the cap"
        );
        assert_eq!(high.baseline.sample_minutes, 0);
        assert_eq!(high.recent.sample_minutes, 60);
        assert_eq!(
          high.comparability,
          BandComparability::TooFewSampleMinutes,
          "recent evidence exists but the baseline side has none"
        );
      }
      other => panic!("expected an established comparison, got {other:?}"),
    }
  }

  #[test]
  fn a_band_at_exactly_the_minimum_sample_minutes_on_both_sides_is_comparable() {
    let baseline_start = date(2026, 8, 1);
    let baseline_end = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);

    let days = vec![
      DailyCoolingSummary {
        date: baseline_start,
        coverage_minutes: 1440,
        idle: band(30.0, COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_start,
        coverage_minutes: 1440,
        idle: band(50.0, COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
      },
    ];

    let result = derive_band_comparison(
      &days,
      &[],
      established_baseline(baseline_start, baseline_end),
      establishing_delta_baseline(),
      recent_end,
    );

    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };
    let idle = bands.iter().find(|b| b.band == CpuLoadBand::Idle).unwrap();
    assert_eq!(idle.comparability, BandComparability::Comparable);
  }

  #[test]
  fn a_band_one_minute_short_on_either_side_is_not_comparable() {
    let baseline_start = date(2026, 8, 1);
    let baseline_end = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let short = COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES - 1;

    let days = vec![
      DailyCoolingSummary {
        date: baseline_start,
        coverage_minutes: 1440,
        idle: band(30.0, short),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_start,
        coverage_minutes: 1440,
        idle: band(50.0, COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
      },
    ];

    let result = derive_band_comparison(
      &days,
      &[],
      established_baseline(baseline_start, baseline_end),
      establishing_delta_baseline(),
      recent_end,
    );

    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };
    let idle = bands.iter().find(|b| b.band == CpuLoadBand::Idle).unwrap();
    assert_eq!(idle.comparability, BandComparability::TooFewSampleMinutes);
  }

  // ── per-band baseline extension (#2333) ──

  /// One day carrying only a high-load temperature band.
  fn high_day(date: NaiveDate, temperature: f32, minutes: u32) -> DailyCoolingSummary {
    DailyCoolingSummary {
      date,
      coverage_minutes: 1440,
      idle: empty_band(),
      low: empty_band(),
      mid: empty_band(),
      high: band(temperature, minutes),
      power: PowerSummary::default(),
    }
  }

  /// One source's ΔT row carrying only a high band.
  fn high_delta_day(
    date: NaiveDate,
    source: &str,
    delta: f32,
    minutes: u32,
  ) -> ThermalDeltaDailySummary {
    ThermalDeltaDailySummary {
      date,
      source: source.to_string(),
      coverage_minutes: minutes,
      idle: empty_band(),
      low: empty_band(),
      mid: empty_band(),
      high: band(delta, minutes),
    }
  }

  /// One band's comparison out of an established result.
  /// `days` plus an idle-only row on each of the 7 days ending at `end`,
  /// so the recent window (#2332: the last 7 *recorded* days) is exactly
  /// the trailing week and never reaches back into a sparse fixture's
  /// pinned window. Idle-only, so no non-idle band gains minutes.
  fn with_recorded_recent_week(
    mut days: Vec<DailyCoolingSummary>,
    end: NaiveDate,
  ) -> Vec<DailyCoolingSummary> {
    days.extend(
      (0..COOLING_BASELINE_RECENT_WINDOW_DAYS as i64)
        .map(|back| idle_day(end - Duration::days(back), 30.0, 60)),
    );
    days
  }

  fn band_comparison(result: CoolingBandComparison, band: CpuLoadBand) -> BandComparison {
    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };
    *bands
      .iter()
      .find(|b| b.band == band)
      .expect("every band is always present")
  }

  #[test]
  fn a_high_band_short_in_the_pinned_window_extends_forward_until_it_holds_enough() {
    // The case from #2333: the pinned window holds 12 high-band minutes,
    // so the high band could never become comparable while its baseline
    // side stayed pinned. It walks forward from the pinned end, day by
    // day, and stops on the first day its minutes reach the minimum.
    let pinned = (date(2026, 8, 1), date(2026, 8, 7));
    let recent_end = date(2026, 8, 31);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(pinned.0, 30.0, 60),
      high_day(date(2026, 8, 3), 60.0, 12),
      high_day(date(2026, 8, 8), 62.0, 10),
      // Cumulative 62 >= 30 here: this day ends the window.
      high_day(date(2026, 8, 9), 64.0, 40),
      // Inside the cap but past the day the band got there: not read.
      high_day(date(2026, 8, 15), 90.0, 100),
      high_day(recent_start, 70.0, 60),
    ];
    let days = with_recorded_recent_week(days, recent_end);

    let result = derive_band_comparison(
      &days,
      &[],
      established_baseline(pinned.0, pinned.1),
      establishing_delta_baseline(),
      recent_end,
    );

    let high = band_comparison(result.clone(), CpuLoadBand::High);
    assert_eq!(
      (
        high.baseline_window_start_date,
        high.baseline_window_end_date
      ),
      (pinned.0, date(2026, 8, 9))
    );
    assert_eq!(high.baseline.sample_minutes, 62);
    let expected = (60.0 * 12.0 + 62.0 * 10.0 + 64.0 * 40.0) / 62.0;
    assert!((high.baseline.temperature_avg.unwrap() - expected).abs() < 0.001);
    assert_eq!(high.comparability, BandComparability::Comparable);

    // The response-level window is still the pinned one; the extension is
    // a per-band fact.
    let CoolingBandComparison::Established {
      baseline_window_start_date,
      baseline_window_end_date,
      ..
    } = result
    else {
      panic!("expected an established comparison");
    };
    assert_eq!(
      (baseline_window_start_date, baseline_window_end_date),
      pinned
    );
  }

  #[test]
  fn a_band_whose_pinned_window_already_holds_enough_keeps_the_pinned_window() {
    // Nothing moves for a band that was fine: the days after the pinned
    // end are not read even though they exist.
    let pinned = (date(2026, 8, 1), date(2026, 8, 7));
    let days = vec![
      idle_day(pinned.0, 30.0, 60),
      high_day(
        date(2026, 8, 3),
        60.0,
        COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES,
      ),
      high_day(date(2026, 8, 8), 90.0, 50),
    ];

    let high = band_comparison(
      derive_band_comparison(
        &days,
        &[],
        established_baseline(pinned.0, pinned.1),
        establishing_delta_baseline(),
        date(2026, 8, 31),
      ),
      CpuLoadBand::High,
    );

    assert_eq!(
      (
        high.baseline_window_start_date,
        high.baseline_window_end_date
      ),
      pinned
    );
    assert_eq!(
      high.baseline.sample_minutes,
      COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES
    );
    assert_eq!(high.baseline.temperature_avg, Some(60.0));
  }

  #[test]
  fn a_band_still_short_at_the_cap_reports_the_capped_window_with_its_real_counts() {
    // 12 minutes in the pinned window and nothing more until the day
    // after the cap. The band stays withheld with the counts it really
    // has, over the window it really looked at - never a window that
    // quietly ran on until it found something.
    let pinned = (date(2026, 8, 1), date(2026, 8, 7));
    let cap_end = pinned.0
      + Duration::days(COOLING_BAND_BASELINE_EXTENSION_MAX_CALENDAR_DAYS as i64 - 1);
    assert_eq!(cap_end, date(2026, 8, 30));
    let recent_end = date(2026, 9, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(pinned.0, 30.0, 60),
      high_day(date(2026, 8, 3), 60.0, 12),
      high_day(cap_end + Duration::days(1), 64.0, 40),
      high_day(recent_start, 70.0, 60),
    ];
    let days = with_recorded_recent_week(days, recent_end);

    let high = band_comparison(
      derive_band_comparison(
        &days,
        &[],
        established_baseline(pinned.0, pinned.1),
        establishing_delta_baseline(),
        recent_end,
      ),
      CpuLoadBand::High,
    );

    assert_eq!(
      (
        high.baseline_window_start_date,
        high.baseline_window_end_date
      ),
      (pinned.0, cap_end)
    );
    assert_eq!(high.baseline.sample_minutes, 12);
    assert_eq!(high.recent.sample_minutes, 60);
    assert_eq!(high.comparability, BandComparability::TooFewSampleMinutes);
  }

  #[test]
  fn the_idle_band_always_reports_exactly_the_pinned_window() {
    // The pinned window *is* the idle baseline's value, so it never
    // extends - even in this contrived fixture where the pinned window
    // holds too few idle minutes and more sit right after it.
    let pinned = (date(2026, 8, 1), date(2026, 8, 7));
    let days = vec![
      idle_day(date(2026, 8, 3), 30.0, 12),
      idle_day(date(2026, 8, 9), 31.0, 40),
      idle_day(date(2026, 8, 25), 32.0, 60),
    ];

    let idle = band_comparison(
      derive_band_comparison(
        &days,
        &[],
        established_baseline(pinned.0, pinned.1),
        establishing_delta_baseline(),
        date(2026, 8, 31),
      ),
      CpuLoadBand::Idle,
    );

    assert_eq!(
      (
        idle.baseline_window_start_date,
        idle.baseline_window_end_date
      ),
      pinned
    );
    assert_eq!(idle.baseline.sample_minutes, 12);
    assert_eq!(idle.comparability, BandComparability::TooFewSampleMinutes);
  }

  #[test]
  fn the_extension_stops_the_day_before_the_recent_window() {
    // The band only fills in *inside* the recent window (Aug 14-20). The
    // baseline side may run up to Aug 13 and no further: reading Aug 16 on
    // both sides would compare a stretch of days with itself.
    let pinned = (date(2026, 8, 1), date(2026, 8, 7));
    let yesterday = date(2026, 8, 20);
    let days = vec![
      idle_day(pinned.0, 30.0, 60),
      high_day(date(2026, 8, 3), 60.0, 12),
      high_day(date(2026, 8, 16), 64.0, 40),
    ];
    let days = with_recorded_recent_week(days, yesterday);

    let high = band_comparison(
      derive_band_comparison(
        &days,
        &[],
        established_baseline(pinned.0, pinned.1),
        establishing_delta_baseline(),
        yesterday,
      ),
      CpuLoadBand::High,
    );

    assert_eq!(
      (
        high.baseline_window_start_date,
        high.baseline_window_end_date
      ),
      (pinned.0, date(2026, 8, 13))
    );
    assert_eq!(high.baseline.sample_minutes, 12);
    assert_eq!(high.recent.sample_minutes, 40);
    assert_eq!(high.comparability, BandComparability::TooFewSampleMinutes);
  }

  #[test]
  fn a_pinned_window_that_already_reaches_the_recent_window_is_not_extended() {
    // A fresh install: the recent window (Aug 4-10) starts inside the
    // pinned one. There is no day left to extend into, so the band keeps
    // the pinned window exactly as before #2333.
    let pinned = (date(2026, 8, 1), date(2026, 8, 7));
    let yesterday = date(2026, 8, 10);
    let days = vec![
      idle_day(pinned.0, 30.0, 60),
      high_day(date(2026, 8, 3), 60.0, 12),
      high_day(date(2026, 8, 9), 64.0, 40),
    ];

    let high = band_comparison(
      derive_band_comparison(
        &days,
        &[],
        established_baseline(pinned.0, pinned.1),
        establishing_delta_baseline(),
        yesterday,
      ),
      CpuLoadBand::High,
    );

    assert_eq!(
      (
        high.baseline_window_start_date,
        high.baseline_window_end_date
      ),
      pinned
    );
    assert_eq!(high.baseline.sample_minutes, 12);
  }

  #[test]
  fn the_cap_bounds_the_extension_without_shortening_a_wide_pinned_window() {
    assert_eq!(
      baseline_extension_cap_end(date(2026, 8, 1), date(2026, 8, 7)),
      date(2026, 8, 30),
      "30 inclusive calendar days from the pinned start"
    );
    assert_eq!(
      baseline_extension_cap_end(date(2026, 8, 1), date(2026, 9, 15)),
      date(2026, 9, 15),
      "seven qualifying days spread wider than the cap keep their own end"
    );
  }

  #[test]
  fn the_ambient_adjusted_side_extends_over_its_own_source_only() {
    // The ΔT baseline is pinned from the desk sensor. A living-room row
    // with a hundred high-band minutes sits inside the walk, and must
    // neither enter the average nor carry the band over the threshold:
    // the desk sensor's own 40 minutes two days later are what do.
    let pinned = (date(2026, 8, 1), date(2026, 8, 7));
    let recent_end = date(2026, 8, 31);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![idle_day(pinned.0, 30.0, 60)];
    let delta_days = vec![
      idle_delta_day(date(2026, 8, 3), "Desk", 10.0, 12),
      idle_delta_day(date(2026, 8, 9), "Desk", 10.0, 40),
      high_delta_day(date(2026, 8, 3), "Desk", 40.0, 12),
      high_delta_day(date(2026, 8, 8), "Living Room", 10.0, 100),
      high_delta_day(date(2026, 8, 9), "Desk", 42.0, 40),
      idle_delta_day(recent_start, "Desk", 11.0, 60),
      high_delta_day(recent_start, "Desk", 45.0, 60),
    ];

    let result = derive_band_comparison(
      &days,
      &delta_days,
      established_baseline(pinned.0, pinned.1),
      established_delta_baseline("Desk", pinned.0, pinned.1),
      recent_end,
    );

    let high = band_comparison(result.clone(), CpuLoadBand::High)
      .ambient_adjusted
      .expect("the high band has ambient");
    assert_eq!(
      (
        high.baseline_window_start_date,
        high.baseline_window_end_date
      ),
      (pinned.0, date(2026, 8, 9))
    );
    assert_eq!(high.baseline.sample_minutes, 52);
    let expected = (40.0 * 12.0 + 42.0 * 40.0) / 52.0;
    assert!((high.baseline.delta_avg.unwrap() - expected).abs() < 0.001);
    assert_eq!(high.comparability, BandComparability::Comparable);

    // Idle ΔT stays on the ΔT baseline's pinned window, like absolute idle.
    let idle = band_comparison(result, CpuLoadBand::Idle)
      .ambient_adjusted
      .expect("the idle band has ambient");
    assert_eq!(
      (
        idle.baseline_window_start_date,
        idle.baseline_window_end_date
      ),
      pinned
    );
    assert_eq!(idle.baseline.sample_minutes, 12);
    assert_eq!(idle.comparability, BandComparability::TooFewSampleMinutes);
  }

  #[test]
  fn a_machine_switched_on_a_few_days_a_week_gets_a_comparable_band() {
    // The #2332 case: recorded on 2 of the last 7 calendar days and on 5
    // more over the preceding three weeks, with ten low-band minutes on
    // each. No seven-calendar-day stretch holds the band's minimum, but
    // the seven recorded days together do, and the recent window reaches
    // back to the seventh of them.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 9, 20);
    let recorded_days_back = [0, 3, 8, 11, 15, 18, 21];
    let low_only = |date: NaiveDate| DailyCoolingSummary {
      date,
      coverage_minutes: 120,
      idle: empty_band(),
      low: band(45.0, 10),
      mid: empty_band(),
      high: empty_band(),
      power: PowerSummary::default(),
    };
    let mut days = vec![DailyCoolingSummary {
      date: baseline_start,
      coverage_minutes: 1440,
      idle: band(30.0, 60),
      low: band(40.0, 60),
      mid: empty_band(),
      high: empty_band(),
      power: PowerSummary::default(),
    }];
    days.extend(
      recorded_days_back
        .iter()
        .rev()
        .map(|back| low_only(recent_end - Duration::days(*back))),
    );

    let result = derive_band_comparison(
      &days,
      &[],
      established_baseline(baseline_start, baseline_start),
      establishing_delta_baseline(),
      recent_end,
    );

    let CoolingBandComparison::Established {
      recent_window_start_date,
      recent_window_end_date,
      bands,
      ..
    } = result
    else {
      panic!("expected an established comparison");
    };
    assert_eq!(recent_window_start_date, recent_end - Duration::days(21));
    assert_eq!(recent_window_end_date, recent_end);
    let low = bands.iter().find(|b| b.band == CpuLoadBand::Low).unwrap();
    assert_eq!(low.recent.sample_minutes, 70);
    assert_eq!(low.recent.temperature_avg, Some(45.0));
    assert_eq!(low.comparability, BandComparability::Comparable);
  }

  #[test]
  fn to_idle_sample_projects_only_the_idle_band() {
    let day = DailyCoolingSummary {
      date: date(2026, 8, 1),
      coverage_minutes: 1440,
      idle: band(30.0, COOLING_BASELINE_QUALIFYING_IDLE_MINUTES),
      low: band(40.0, 300),
      mid: empty_band(),
      high: empty_band(),
      power: PowerSummary::default(),
    };

    let sample = to_idle_sample(&day);

    assert_eq!(sample.date, day.date);
    assert_eq!(sample.idle_temperature_avg, Some(30.0));
    assert_eq!(
      sample.idle_sample_minutes,
      COOLING_BASELINE_QUALIFYING_IDLE_MINUTES
    );
  }

  // ── pinned baseline (DB-backed) ──

  mod pinned_baseline {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_tables(pool: &SqlitePool) {
      create_tables(
        pool,
        &[
          COOLING_DAILY_SUMMARY_DDL,
          COOLING_BASELINE_DDL,
          COOLING_DELTA_BASELINE_DDL,
          COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL,
        ],
      )
      .await;
    }

    async fn insert_idle_day(
      pool: &SqlitePool,
      date: NaiveDate,
      temperature: f32,
      minutes: u32,
    ) {
      sqlx::query(
        "INSERT INTO cooling_daily_summary
           (date, idle_cpu_temperature_avg, idle_sample_minutes, coverage_minutes)
         VALUES ($1, $2, $3, 1440)",
      )
      .bind(date.format("%Y-%m-%d").to_string())
      .bind(temperature)
      .bind(minutes as i64)
      .execute(pool)
      .await
      .unwrap();
    }

    async fn insert_establishing_days(
      pool: &SqlitePool,
      start: NaiveDate,
      temperature: f32,
    ) {
      use crate::persistence::cooling_baseline::{
        COOLING_BASELINE_QUALIFYING_IDLE_MINUTES,
        COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      };
      for offset in 0..COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS {
        insert_idle_day(
          pool,
          start + Duration::days(offset as i64),
          temperature,
          COOLING_BASELINE_QUALIFYING_IDLE_MINUTES,
        )
        .await;
      }
    }

    #[tokio::test]
    async fn the_baseline_window_does_not_drift_when_its_source_rows_are_deleted() {
      // Same regression as cooling_baseline's own pinning test, but for
      // the band comparison's loader: it must resolve the pinned row
      // through the shared resolver instead of re-deriving from whatever
      // `cooling_daily_summary` rows currently exist.
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      setup_tables(&pool).await;
      let start = date(2026, 8, 1);
      insert_establishing_days(&pool, start, 42.0).await;

      let established = load_cooling_band_comparison_from_pool(&pool, date(2026, 8, 20))
        .await
        .unwrap();
      let CoolingBandComparison::Established {
        baseline_window_start_date,
        ..
      } = established
      else {
        panic!("expected an established comparison");
      };
      assert_eq!(baseline_window_start_date, start);

      // Age out the rows the baseline was derived from, and record a
      // hotter stretch that would establish a different window if the
      // pinned row were ignored.
      sqlx::query("DELETE FROM cooling_daily_summary")
        .execute(&pool)
        .await
        .unwrap();
      insert_establishing_days(&pool, date(2027, 6, 1), 70.0).await;

      let after_cleanup =
        load_cooling_band_comparison_from_pool(&pool, date(2027, 6, 20))
          .await
          .unwrap();
      let CoolingBandComparison::Established {
        baseline_window_start_date,
        ..
      } = after_cleanup
      else {
        panic!("expected an established comparison");
      };
      assert_eq!(
        baseline_window_start_date, start,
        "the pinned baseline window must not drift when its source rows are deleted"
      );
    }
  }
}
