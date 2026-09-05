//! Cooling Insight baseline delta: how far the trailing 7-day idle
//! temperature average has drifted from the established baseline, and
//! whether that drift has been sustained long enough to call out (#2017,
//! thresholds from #1666).
//!
//! The threshold defaults live here, behind this module's boundary, so
//! the frontend never re-derives "how much warmer counts as a mild
//! rise" - it only renders the [`CoolingDeltaObservation`] Core already
//! decided.

use chrono::{Duration, NaiveDate};

use crate::persistence::cooling_band_comparison::{
  BandDeltaWindowSummary, band_delta_window_summary, dominant_delta_source,
};
use crate::persistence::cooling_baseline::{
  BaselineState, COOLING_BASELINE_RECENT_WINDOW_DAYS, DailyIdleSample, RecentIdleSummary,
  summarize_recent_idle,
};
use crate::persistence::cooling_delta_baseline::DeltaBaselineState;
use crate::persistence::cooling_rollup::{CpuLoadBand, DailyCoolingSummary};
use crate::persistence::cooling_thermal_delta_rollup::ThermalDeltaDailySummary;

/// Consecutive trailing-window days a delta must stay at or above
/// [`COOLING_DELTA_MILD_RISE_THRESHOLD`] before the rise counts as
/// "sustained" rather than a single noisy day.
pub const COOLING_DELTA_SUSTAIN_DAYS: u32 = 3;

/// Delta (degrees Celsius, recent minus baseline) at or above which a
/// sustained rise is reported as mild.
pub const COOLING_DELTA_MILD_RISE_THRESHOLD: f32 = 5.0;

/// Delta (degrees Celsius) at or above which a sustained rise is
/// reported as large instead of mild.
pub const COOLING_DELTA_LARGE_RISE_THRESHOLD: f32 = 10.0;

/// Cooling Insight's read of the current idle-temperature drift.
///
/// `Establishing` and `NotComparable` both withhold a verdict on the
/// drift itself - they differ in *why*: no baseline exists yet, versus a
/// baseline exists but the recent window does not carry enough idle
/// evidence to compare against it (see
/// [`RecentIdleSummary::is_comparable`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingDeltaObservation {
  Establishing,
  NotComparable,
  WithinRange,
  SustainedMildRise,
  SustainedLargeRise,
}

/// One trailing-7-day-window's delta against the baseline, ending on
/// `date`. Part of the series a sustained-rise verdict was computed
/// from, returned so the UI can render "n days in a row" without
/// recomputing it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DailyDelta {
  pub date: NaiveDate,
  pub delta: f32,
}

/// The ambient-normalized reading of the same drift (#2045): how far the
/// machine's idle rise *above ambient* has moved, rather than how far its
/// absolute idle temperature has moved.
///
/// This is what separates "summer made the air hotter" from "the cooling
/// degraded": a ΔT that held steady while the absolute temperature climbed
/// says the room warmed up, and a ΔT that climbed says the machine did.
///
/// `delta` subtracts the ΔT baseline from the recent window's ΔT average
/// (`recent - baseline`, so a rise reads positive), which is legitimate
/// where subtracting a CPU summary from an ambient summary is not: both
/// sides are already per-minute ΔT values paired before aggregation, so
/// this compares one period against another rather than reconstructing a
/// pairing that never happened.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientAdjustedBaselineDelta {
  /// The ΔT baseline's own lifecycle, which advances independently of
  /// the absolute baseline beside it and carries its own window - see
  /// [`crate::persistence::cooling_delta_baseline`] for why the two
  /// cannot share one.
  pub baseline_state: DeltaBaselineState,
  /// The trailing recent window's idle ΔT, over the same days
  /// [`CoolingBaselineDelta::recent`] covers, read from whichever ambient
  /// source covered the most of that window
  /// (`cooling_band_comparison::dominant_delta_source`).
  pub recent: BandDeltaWindowSummary,
  /// `recent - baseline`, or `None` unless `comparable`.
  pub delta: Option<f32>,
  /// Whether the ΔT baseline is established, the recent window carries
  /// enough paired minutes for the subtraction to mean anything, *and*
  /// both were measured against the same ambient source (#2062). After a
  /// sensor change the recent window is still reported, but "recent minus
  /// baseline" would be the difference between two placements rather
  /// than a drift, so it is withheld.
  pub comparable: bool,
}

/// Everything Cooling Insight needs to render the baseline delta card.
#[derive(Debug, Clone, PartialEq)]
pub struct CoolingBaselineDelta {
  pub baseline_state: BaselineState,
  pub recent: RecentIdleSummary,
  /// `recent.idle_temperature_avg - baseline`, or `None` whenever
  /// `observation` is `Establishing` or `NotComparable`.
  pub delta: Option<f32>,
  pub observation: CoolingDeltaObservation,
  /// The trailing-window deltas actually examined to reach
  /// `sustained_days`, oldest first, ending at the same day `recent`
  /// covers. Empty when `observation` is `Establishing` or
  /// `NotComparable`.
  pub daily_deltas: Vec<DailyDelta>,
  pub sustained_days: u32,
  /// The ambient-normalized reading of the same drift (#2045). Always
  /// present, carrying its own lifecycle: an install with no
  /// environmental sensor reports `Establishing { qualifying_days: 0 }`
  /// with an empty recent window, which is honest and fabricates
  /// nothing. Every field above is computed exactly as it was before
  /// #2045 regardless of what this one says.
  pub ambient_adjusted: AmbientAdjustedBaselineDelta,
}

/// Derive the baseline delta from every summarized day's idle-band
/// facts and the current baseline lifecycle state.
///
/// `window_end_date` is the most recent completed local day (yesterday),
/// matching [`crate::persistence::cooling_baseline::derive_cooling_baseline`].
/// `delta_days` carries the row-per-source Thermal Delta rollup, which is
/// where the per-band ΔT lives (#2045, #2062). It is a second slice rather
/// than part of `days` because the absolute-temperature verdict above
/// must not change shape at all when ambient data appears: pass an empty
/// slice with an establishing `delta_baseline_state` and every field but
/// `ambient_adjusted` is computed exactly as it was before #2045.
///
/// `delta_baseline_state` is a second lifecycle rather than something
/// derived from `baseline_state`, because the two establish
/// independently - see [`crate::persistence::cooling_delta_baseline`].
pub fn derive_baseline_delta(
  days: &[DailyIdleSample],
  delta_days: &[ThermalDeltaDailySummary],
  baseline_state: BaselineState,
  delta_baseline_state: DeltaBaselineState,
  window_end_date: NaiveDate,
) -> CoolingBaselineDelta {
  let recent = summarize_recent_idle(days, window_end_date);
  // Derived regardless of the absolute verdict below: the two readings
  // answer different questions, and one being unavailable says nothing
  // about the other. A machine can have an established ΔT baseline while
  // the absolute one is still establishing, and vice versa.
  let ambient_adjusted =
    derive_ambient_adjusted(delta_days, delta_baseline_state, window_end_date);

  let Some(baseline_temperature) = established_temperature(baseline_state) else {
    return CoolingBaselineDelta {
      baseline_state,
      recent,
      delta: None,
      observation: CoolingDeltaObservation::Establishing,
      daily_deltas: Vec::new(),
      sustained_days: 0,
      ambient_adjusted,
    };
  };

  if !recent.is_comparable() {
    return CoolingBaselineDelta {
      baseline_state,
      recent,
      delta: None,
      observation: CoolingDeltaObservation::NotComparable,
      daily_deltas: Vec::new(),
      sustained_days: 0,
      ambient_adjusted,
    };
  }

  let (daily_deltas, mild_sustained_days, large_sustained_days) =
    trailing_daily_deltas(days, baseline_temperature, window_end_date);
  let (observation, sustained_days) =
    classify_observation(mild_sustained_days, large_sustained_days);
  // `recent.is_comparable()` guarantees an average is present. This is
  // the same quantity `trailing_daily_deltas` computes for `cursor ==
  // window_end_date` (when that day has its own rollup row), kept as a
  // direct computation here so a day recent enough to be "comparable" as
  // a trailing window, but too sparse to itself carry an idle-sample row
  // exactly on `window_end_date` (see `has_idle_sample_on`), still gets a
  // reported delta - only the sustain streak requires a same-day row.
  let delta = recent.idle_temperature_avg.unwrap() - baseline_temperature;

  CoolingBaselineDelta {
    baseline_state,
    recent,
    delta: Some(delta),
    observation,
    daily_deltas,
    sustained_days,
    ambient_adjusted,
  }
}

/// The ΔT baseline against the idle ΔT of the trailing recent window
/// (#2045).
///
/// `delta_baseline_state` is resolved independently of the absolute
/// baseline - see [`crate::persistence::cooling_delta_baseline`] for the
/// failure that avoids. Anchoring this reading to the absolute
/// baseline's window (the obvious design) leaves every machine that
/// began collecting ambient data *after* that window permanently
/// non-comparable, because the archive cannot grow ambient readings for
/// past days retroactively.
///
/// The recent window is read from the source that covered most of it, and
/// compared only if that is the source the baseline was established from
/// (#2062).
fn derive_ambient_adjusted(
  days: &[ThermalDeltaDailySummary],
  delta_baseline_state: DeltaBaselineState,
  window_end_date: NaiveDate,
) -> AmbientAdjustedBaselineDelta {
  let recent_start =
    window_end_date - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
  let recent_source = dominant_delta_source(days, recent_start, window_end_date);
  let recent = recent_source.map_or_else(BandDeltaWindowSummary::default, |source| {
    band_delta_window_summary(
      days,
      source,
      CpuLoadBand::Idle,
      recent_start,
      window_end_date,
    )
  });

  let baseline = match &delta_baseline_state {
    DeltaBaselineState::Established {
      source,
      delta_temperature_avg,
      ..
    } => Some((source.as_str(), *delta_temperature_avg)),
    DeltaBaselineState::Establishing { .. } => None,
  };
  let comparable = baseline.is_some_and(|(baseline_source, _)| {
    recent.is_comparable() && recent_source == Some(baseline_source)
  });

  AmbientAdjustedBaselineDelta {
    // `comparable` implies both are present.
    delta: comparable.then(|| recent.delta_avg.unwrap() - baseline.unwrap().1),
    baseline_state: delta_baseline_state,
    recent,
    comparable,
  }
}

fn established_temperature(state: BaselineState) -> Option<f32> {
  match state {
    BaselineState::Established {
      idle_temperature_avg,
      ..
    } => Some(idle_temperature_avg),
    BaselineState::Establishing { .. } => None,
  }
}

/// Classify the observation from the two independently-counted streaks
/// [`trailing_daily_deltas`] returns, and report the streak length that
/// backs the returned observation (so the UI's "n days in a row" always
/// matches what actually triggered it).
///
/// A large rise requires [`COOLING_DELTA_SUSTAIN_DAYS`] consecutive days
/// at or above [`COOLING_DELTA_LARGE_RISE_THRESHOLD`] specifically - not
/// merely a mild-or-above streak whose *most recent* day happens to have
/// crossed the large threshold. Two days at +7 followed by a today at
/// +10 is a 3-day mild streak, not a 3-day large one.
fn classify_observation(
  mild_sustained_days: u32,
  large_sustained_days: u32,
) -> (CoolingDeltaObservation, u32) {
  if large_sustained_days >= COOLING_DELTA_SUSTAIN_DAYS {
    (
      CoolingDeltaObservation::SustainedLargeRise,
      large_sustained_days,
    )
  } else if mild_sustained_days >= COOLING_DELTA_SUSTAIN_DAYS {
    (
      CoolingDeltaObservation::SustainedMildRise,
      mild_sustained_days,
    )
  } else {
    (CoolingDeltaObservation::WithinRange, mild_sustained_days)
  }
}

/// Whether `date` itself has a rollup row carrying idle-band evidence.
///
/// Required before a cursor day may extend either streak in
/// [`trailing_daily_deltas`]: without it, a single real day's minutes can
/// still appear (weighted) inside several different 7-day trailing
/// windows a few calendar days apart, since those windows overlap. Only
/// gating on window comparability would let that overlap alone report
/// several "sustained" days from one real observation - exactly the
/// streak the day itself never spanned (DP-02).
fn has_idle_sample_on(days: &[DailyIdleSample], date: NaiveDate) -> bool {
  days
    .iter()
    .any(|day| day.date == date && day.idle_sample_minutes > 0)
}

/// Walk backward day by day from `window_end_date`, recomputing the
/// trailing 7-day idle average at each day and comparing it against
/// `baseline_temperature`. A day only extends either streak - and is
/// only added to the series - when it carries its own rollup row (see
/// [`has_idle_sample_on`]); the walk stops there otherwise, since an
/// unobserved day cannot be counted as part of a sustained trend. It
/// also stops (including that day) at the first day whose delta drops
/// below [`COOLING_DELTA_MILD_RISE_THRESHOLD`] - the walk only needs to
/// go far enough to explain the streak lengths, not the whole rollup
/// history.
///
/// Returns `(daily_deltas, mild_sustained_days, large_sustained_days)`.
/// `daily_deltas` is oldest first, so the series reads left-to-right as a
/// timeline. `mild_sustained_days` counts the streak at
/// [`COOLING_DELTA_MILD_RISE_THRESHOLD`] or above; `large_sustained_days`
/// counts the streak at [`COOLING_DELTA_LARGE_RISE_THRESHOLD`] or above,
/// and stops growing (without ending the walk) the first time a day
/// falls below the large threshold while still at or above the mild one.
fn trailing_daily_deltas(
  days: &[DailyIdleSample],
  baseline_temperature: f32,
  window_end_date: NaiveDate,
) -> (Vec<DailyDelta>, u32, u32) {
  let mut series = Vec::new();
  let mut mild_sustained_days = 0u32;
  let mut large_sustained_days = 0u32;
  let mut large_streak_intact = true;
  let mut cursor = window_end_date;

  loop {
    if !has_idle_sample_on(days, cursor) {
      break;
    }
    let window = summarize_recent_idle(days, cursor);
    if !window.is_comparable() {
      break;
    }
    // `is_comparable()` guarantees an average is present.
    let delta = window.idle_temperature_avg.unwrap() - baseline_temperature;
    let is_mild = delta >= COOLING_DELTA_MILD_RISE_THRESHOLD;
    series.push(DailyDelta {
      date: cursor,
      delta,
    });
    if !is_mild {
      break;
    }
    mild_sustained_days += 1;
    if large_streak_intact && delta >= COOLING_DELTA_LARGE_RISE_THRESHOLD {
      large_sustained_days += 1;
    } else {
      large_streak_intact = false;
    }
    cursor -= Duration::days(1);
  }

  series.reverse();
  (series, mild_sustained_days, large_sustained_days)
}

/// [`derive_baseline_delta`] over the whole `cooling_daily_summary`
/// table, resolving the baseline lifecycle state through
/// [`crate::persistence::cooling_baseline::resolve_baseline_state_from_pool`]
/// rather than re-deriving it - the pinned baseline row must win once one
/// exists, or this delta would silently drift once the rollup rows the
/// original establishment came from age out.
pub(crate) async fn load_cooling_baseline_delta_from_pool(
  pool: &sqlx::SqlitePool,
  today: NaiveDate,
) -> Result<CoolingBaselineDelta, sqlx::Error> {
  use crate::infrastructure::database;
  use crate::persistence::cooling_baseline::resolve_baseline_state_from_pool;

  // The table holds at most `COOLING_DAILY_SUMMARY_RETENTION_DAYS` rows -
  // see `select_daily_idle_samples` for why reading all of them is cheap.
  let summaries =
    database::cooling_daily_summary::select_all_daily_cooling_summaries_from_pool(pool)
      .await?;
  let days: Vec<_> = summaries.iter().map(to_idle_sample).collect();
  let baseline_state = resolve_baseline_state_from_pool(pool, &days).await?;
  // The Thermal Delta lives in its own row-per-source table (#2062).
  let delta_days =
    database::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries_from_pool(
      pool,
    )
    .await?;
  // Resolved (and pinned) through its own resolver, against its own
  // table: the ΔT baseline establishes on its own schedule.
  let delta_baseline_state =
    crate::persistence::cooling_delta_baseline::resolve_delta_baseline_state_from_pool(
      pool,
      &delta_days,
    )
    .await?;
  let yesterday = today - Duration::days(1);

  Ok(derive_baseline_delta(
    &days,
    &delta_days,
    baseline_state,
    delta_baseline_state,
    yesterday,
  ))
}

fn to_idle_sample(day: &DailyCoolingSummary) -> DailyIdleSample {
  DailyIdleSample {
    date: day.date,
    idle_temperature_avg: day.idle.avg,
    idle_sample_minutes: day.idle.sample_minutes,
  }
}

/// [`load_cooling_baseline_delta_from_pool`] against Core's process-wide
/// pool.
pub async fn load_cooling_baseline_delta() -> Result<CoolingBaselineDelta, sqlx::Error> {
  let pool = crate::infrastructure::database::db::get_pool().await?;
  load_cooling_baseline_delta_from_pool(&pool, chrono::Local::now().date_naive()).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::test_schema::{
    COOLING_BASELINE_DDL, COOLING_DAILY_SUMMARY_DDL, COOLING_DELTA_BASELINE_DDL,
    COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL, create_tables,
  };
  use crate::persistence::cooling_baseline::{
    COOLING_BASELINE_COMPARABLE_IDLE_MINUTES, COOLING_BASELINE_RECENT_WINDOW_DAYS,
  };

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn day(date: NaiveDate, temperature: f32, minutes: u32) -> DailyIdleSample {
    DailyIdleSample {
      date,
      idle_temperature_avg: Some(temperature),
      idle_sample_minutes: minutes,
    }
  }

  fn established(baseline_temperature: f32) -> BaselineState {
    BaselineState::Established {
      idle_temperature_avg: baseline_temperature,
      window_start_date: date(2026, 1, 1),
      window_end_date: date(2026, 1, 7),
      sample_minutes: 210,
    }
  }

  /// `count` consecutive days ending at `end`, each carrying a full
  /// comparable window's worth of idle minutes at `temperature`.
  fn days_ending_at(
    end: NaiveDate,
    count: i64,
    temperature: f32,
  ) -> Vec<DailyIdleSample> {
    (0..count)
      .map(|offset| {
        day(
          end - Duration::days(offset),
          temperature,
          // One full comparable day's worth on its own is enough for
          // every 7-day trailing window in this range to be comparable.
          COOLING_BASELINE_COMPARABLE_IDLE_MINUTES,
        )
      })
      .collect()
  }

  // ── establishing / not comparable ──

  #[test]
  fn an_unestablished_baseline_reports_establishing_with_no_delta() {
    let state = BaselineState::Establishing {
      qualifying_days: 3,
      required_days: 7,
    };
    let result = derive_baseline_delta(
      &[],
      &[],
      state,
      establishing_delta_baseline(),
      date(2026, 8, 20),
    );

    assert_eq!(result.observation, CoolingDeltaObservation::Establishing);
    assert_eq!(result.delta, None);
    assert_eq!(result.sustained_days, 0);
    assert!(result.daily_deltas.is_empty());
  }

  #[test]
  fn an_established_baseline_without_recent_idle_evidence_is_not_comparable() {
    let result = derive_baseline_delta(
      &[],
      &[],
      established(30.0),
      establishing_delta_baseline(),
      date(2026, 8, 20),
    );

    assert_eq!(result.observation, CoolingDeltaObservation::NotComparable);
    assert_eq!(result.delta, None);
    assert_eq!(result.sustained_days, 0);
    assert!(result.daily_deltas.is_empty());
  }

  // ── delta threshold boundaries (single recent day, no sustain yet) ──

  #[test]
  fn a_delta_below_five_degrees_is_within_range_even_if_sustained() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 10, 34.9);
    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert!((result.delta.unwrap() - 4.9).abs() < 0.001);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
  }

  #[test]
  fn a_single_day_at_a_five_degree_rise_is_not_yet_sustained() {
    let end = date(2026, 8, 20);
    // Only one comparable day exists; the day before has no history at
    // all, so the walk can only confirm a one-day streak - short of the
    // 3-day sustain requirement, even though today's own delta clears
    // the rise threshold.
    let days = vec![day(
      end,
      35.0,
      COOLING_BASELINE_COMPARABLE_IDLE_MINUTES * COOLING_BASELINE_RECENT_WINDOW_DAYS,
    )];
    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert!((result.delta.unwrap() - 5.0).abs() < 0.001);
    assert_eq!(result.sustained_days, 1);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
  }

  // ── sustain-length boundary ──

  #[test]
  fn two_sustained_days_are_not_yet_enough_to_report_a_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 2, 35.0);
    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(result.sustained_days, 2);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
  }

  #[test]
  fn three_sustained_days_at_a_mild_rise_report_sustained_mild_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 3, 35.0);
    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(result.sustained_days, 3);
    assert_eq!(
      result.observation,
      CoolingDeltaObservation::SustainedMildRise
    );
  }

  // ── mild vs large boundary ──

  #[test]
  fn a_sustained_delta_just_under_ten_degrees_is_a_mild_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 5, 39.9);
    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert!((result.delta.unwrap() - 9.9).abs() < 0.001);
    assert_eq!(
      result.observation,
      CoolingDeltaObservation::SustainedMildRise
    );
  }

  #[test]
  fn a_sustained_delta_at_exactly_ten_degrees_is_a_large_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 5, 40.0);
    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert!((result.delta.unwrap() - 10.0).abs() < 0.001);
    assert_eq!(
      result.observation,
      CoolingDeltaObservation::SustainedLargeRise
    );
  }

  // ── daily delta series ──

  #[test]
  fn the_daily_delta_series_is_oldest_first_and_stops_where_the_streak_breaks() {
    let end = date(2026, 8, 20);
    // `end`'s own 7-day trailing window mixes both days (they are one
    // day apart), so its average is pulled up by the heavily-weighted
    // hot day but still shows the cooler day's influence; `end - 1`'s
    // window contains only the cooler day and reads its temperature
    // directly. Weights are chosen so the mixed window still clears the
    // sustained-rise threshold while the cooler-only window does not,
    // giving a clean one-day streak to assert on.
    let days = vec![
      day(end, 60.0, 6 * COOLING_BASELINE_COMPARABLE_IDLE_MINUTES),
      day(
        end - Duration::days(1),
        20.0,
        COOLING_BASELINE_COMPARABLE_IDLE_MINUTES,
      ),
    ];

    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(result.sustained_days, 1);
    let dates: Vec<_> = result.daily_deltas.iter().map(|d| d.date).collect();
    assert_eq!(dates, vec![end - Duration::days(1), end]);
    // The older, cooler-only window reads a negative delta...
    assert!((result.daily_deltas[0].delta - (-10.0)).abs() < 0.001);
    // ...while the newer, mixed window still clears the rise threshold.
    assert!(result.daily_deltas[1].delta >= COOLING_DELTA_MILD_RISE_THRESHOLD);
  }

  #[test]
  fn the_daily_delta_series_stops_when_history_runs_out() {
    let end = date(2026, 8, 20);
    // Only 2 comparable days exist at all; the walk cannot look further
    // back than that, so it stops there rather than treating missing
    // history as a broken streak or looping forever.
    let days = days_ending_at(end, 2, 35.0);

    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(result.sustained_days, 2);
    assert_eq!(result.daily_deltas.len(), 2);
  }

  // ── gap days must not inflate the streak via window overlap ──

  #[test]
  fn a_single_real_day_does_not_inflate_the_streak_through_overlapping_windows() {
    // Regression: a real day's minutes can appear (weighted) in several
    // different 7-day trailing windows a few calendar days apart, since
    // those windows overlap. Before requiring the cursor day to carry
    // its own rollup row, walking backward from `end` through
    // `end - 6` would all land on windows that still contain this one
    // real day 3 days back, reporting several "sustained" days from a
    // single observation.
    let end = date(2026, 8, 20);
    let real_day = end - Duration::days(3);
    let days = vec![day(real_day, 60.0, 1440)];

    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(result.sustained_days, 0);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
    assert!(result.daily_deltas.is_empty());
  }

  #[test]
  fn a_gap_immediately_before_the_most_recent_day_still_stops_the_streak() {
    // `end` itself has no rollup row (e.g. the app was not running
    // yesterday); a real day 2 days back must not let the walk treat
    // `end` as part of an ongoing streak.
    let end = date(2026, 8, 20);
    let days = vec![day(end - Duration::days(2), 60.0, 1440)];

    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(result.sustained_days, 0);
    assert!(result.daily_deltas.is_empty());
  }

  // ── mild vs large streaks are counted independently ──

  #[test]
  fn two_days_at_a_large_rise_followed_by_a_milder_third_day_report_sustained_mild_rise()
  {
    // Two consecutive days whose trailing window average clears the
    // large threshold, then (going further back) a third day whose
    // trailing window is only mild: the *mild* streak reaches 3 days,
    // but the *large* streak stops at 2 - the large threshold must not
    // be granted on the strength of a 3-day streak that only mostly
    // cleared it.
    let end = date(2026, 8, 20);
    // Chosen so each cursor's 7-day trailing window (which accumulates
    // every real row already seen, since all three days sit within 7
    // days of each other) averages out to the target delta at that
    // cursor: end-2 alone -> +7, end-1 blended with end-2 -> +10, end
    // blended with both -> +10.
    let days = vec![
      day(
        end - Duration::days(2),
        37.0,
        COOLING_BASELINE_COMPARABLE_IDLE_MINUTES,
      ),
      day(
        end - Duration::days(1),
        43.0,
        COOLING_BASELINE_COMPARABLE_IDLE_MINUTES,
      ),
      day(end, 40.0, COOLING_BASELINE_COMPARABLE_IDLE_MINUTES),
    ];

    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(
      result.observation,
      CoolingDeltaObservation::SustainedMildRise
    );
    assert_eq!(result.sustained_days, 3);
  }

  #[test]
  fn three_consecutive_days_at_a_large_rise_report_sustained_large_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 3, 40.0);

    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(
      result.observation,
      CoolingDeltaObservation::SustainedLargeRise
    );
    assert_eq!(result.sustained_days, 3);
  }

  #[test]
  fn two_large_rise_days_are_not_yet_enough_for_a_large_verdict() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 2, 45.0);

    let result = derive_baseline_delta(
      &days,
      &[],
      established(30.0),
      establishing_delta_baseline(),
      end,
    );

    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
    assert_eq!(result.sustained_days, 2);
  }

  // ── ambient-adjusted baseline delta (#2045) ──

  /// The ΔT lifecycle a machine with no ambient data reports. Most of
  /// the tests above are about the absolute reading and pass this.
  fn establishing_delta_baseline() -> DeltaBaselineState {
    DeltaBaselineState::Establishing {
      qualifying_days: 0,
      required_days: 7,
    }
  }

  mod ambient_adjusted {
    use super::*;
    use crate::persistence::cooling_band_comparison::COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES;
    use crate::persistence::cooling_delta_baseline::derive_delta_baseline_state;
    use crate::persistence::cooling_rollup::{BandSummary, PowerSummary};

    fn band(avg: f32, minutes: u32) -> BandSummary {
      BandSummary {
        avg: Some(avg),
        max: Some(avg + 1.0),
        min: Some(avg - 1.0),
        sample_minutes: minutes,
      }
    }

    /// One rollup row carrying an idle band.
    fn summary(date: NaiveDate, temperature: f32, minutes: u32) -> DailyCoolingSummary {
      DailyCoolingSummary {
        date,
        coverage_minutes: 1440,
        idle: band(temperature, minutes),
        low: BandSummary::default(),
        mid: BandSummary::default(),
        high: BandSummary::default(),
        power: PowerSummary::default(),
      }
    }

    /// One source's ΔT row carrying an idle ΔT (#2045, #2062).
    fn delta_row(
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
        low: BandSummary::default(),
        mid: BandSummary::default(),
        high: BandSummary::default(),
      }
    }

    /// The absolute baseline window used throughout: a single day, 8-01.
    fn baseline_window() -> BaselineState {
      BaselineState::Established {
        idle_temperature_avg: 30.0,
        window_start_date: date(2026, 8, 1),
        window_end_date: date(2026, 8, 1),
        sample_minutes: 210,
      }
    }

    /// A ΔT baseline established from the desk sensor.
    fn established_delta(avg: f32) -> DeltaBaselineState {
      DeltaBaselineState::Established {
        source: "Desk".to_string(),
        delta_temperature_avg: avg,
        window_start_date: date(2026, 8, 1),
        window_end_date: date(2026, 8, 7),
        sample_minutes: 210,
      }
    }

    #[test]
    fn an_install_with_no_ambient_data_reports_an_establishing_delta_baseline() {
      // The zero-ambient invariant: every pre-#2045 field keeps its
      // value, and the ambient reading says "still establishing, zero
      // qualifying days" rather than fabricating a number.
      let end = date(2026, 8, 20);
      let idle = days_ending_at(end, 5, 35.0);

      let result = derive_baseline_delta(
        &idle,
        &[],
        baseline_window(),
        establishing_delta_baseline(),
        end,
      );

      assert_eq!(
        result.ambient_adjusted.baseline_state,
        establishing_delta_baseline()
      );
      assert_eq!(result.ambient_adjusted.delta, None);
      assert!(!result.ambient_adjusted.comparable);
      assert_eq!(result.ambient_adjusted.recent.sample_minutes, 0);
      // ...and the absolute verdict is the one it always was.
      assert_eq!(
        result.observation,
        CoolingDeltaObservation::SustainedMildRise
      );
      assert_eq!(result.delta, Some(5.0));
    }

    #[test]
    fn a_flat_delta_under_a_rising_absolute_temperature_reports_no_ambient_drift() {
      // The reading the whole feature exists for. Absolute idle climbed
      // 10 K between the windows while ΔT held at 12 K: the room warmed,
      // the cooling did not degrade.
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = [
        summary(date(2026, 8, 1), 30.0, 120),
        summary(recent_start, 40.0, 120),
      ];
      let delta_days = vec![
        delta_row(date(2026, 8, 1), "Desk", 12.0, 120),
        delta_row(recent_start, "Desk", 12.0, 120),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &delta_days,
        baseline_window(),
        established_delta(12.0),
        end,
      );

      let adjusted = result.ambient_adjusted;
      assert!(adjusted.comparable);
      assert_eq!(adjusted.recent.delta_avg, Some(12.0));
      assert_eq!(adjusted.delta, Some(0.0));
      // The absolute reading still reports the 10 K rise it always did.
      assert_eq!(result.delta, Some(10.0));
    }

    #[test]
    fn a_rising_delta_reports_ambient_drift_of_its_own() {
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = [
        summary(date(2026, 8, 1), 30.0, 120),
        summary(recent_start, 40.0, 120),
      ];
      let delta_days = vec![
        delta_row(date(2026, 8, 1), "Desk", 12.0, 120),
        delta_row(recent_start, "Desk", 19.5, 120),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &delta_days,
        baseline_window(),
        established_delta(12.0),
        end,
      );

      assert!((result.ambient_adjusted.delta.unwrap() - 7.5).abs() < 0.001);
    }

    #[test]
    fn a_thin_recent_window_reports_not_comparable_with_no_delta() {
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let short = COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES - 1;
      let summaries = [
        summary(date(2026, 8, 1), 30.0, 120),
        summary(recent_start, 40.0, 120),
      ];
      let delta_days = vec![
        delta_row(date(2026, 8, 1), "Desk", 12.0, 120),
        delta_row(recent_start, "Desk", 19.0, short),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &delta_days,
        baseline_window(),
        established_delta(12.0),
        end,
      );

      let adjusted = result.ambient_adjusted;
      assert!(!adjusted.comparable);
      assert_eq!(
        adjusted.delta, None,
        "no number may be reported from a window this thin"
      );
      assert_eq!(adjusted.recent.sample_minutes, short);
    }

    #[test]
    fn a_recent_window_from_a_different_source_than_the_baseline_is_not_comparable() {
      // The user switched from the desk sensor the baseline was pinned
      // against to one across the room. The recent window is rich and is
      // reported, but subtracting the two would compare placements, not
      // periods (#2062).
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = [
        summary(date(2026, 8, 1), 30.0, 120),
        summary(recent_start, 30.0, 120),
      ];
      let delta_days = vec![
        delta_row(date(2026, 8, 1), "Desk", 12.0, 600),
        delta_row(recent_start, "Living Room", 15.0, 600),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &delta_days,
        baseline_window(),
        established_delta(12.0),
        end,
      );

      let adjusted = result.ambient_adjusted;
      assert_eq!(adjusted.recent.delta_avg, Some(15.0));
      assert!(!adjusted.comparable);
      assert_eq!(
        adjusted.delta, None,
        "a sensor change must never read as a 3 K cooling drift"
      );
    }

    #[test]
    fn the_recent_window_reads_only_the_source_with_the_most_coverage() {
      // Two sensors overlap in the recent window: the reported ΔT is the
      // dominant sensor's own, never a blend of the two.
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = [summary(recent_start, 40.0, 120)];
      let delta_days = vec![
        delta_row(recent_start, "Desk", 13.0, 900),
        delta_row(recent_start, "Living Room", 30.0, 100),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &delta_days,
        baseline_window(),
        established_delta(12.0),
        end,
      );

      let adjusted = result.ambient_adjusted;
      assert_eq!(adjusted.recent.delta_avg, Some(13.0));
      assert_eq!(adjusted.recent.sample_minutes, 900);
      assert!(adjusted.comparable);
      assert_eq!(adjusted.delta, Some(1.0));
    }

    #[test]
    fn an_establishing_delta_baseline_withholds_the_delta_however_rich_the_recent_window()
    {
      // The recent side has plenty of paired minutes, but there is no
      // reference to measure them against yet.
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = [summary(recent_start, 40.0, 120)];
      let delta_days = vec![delta_row(recent_start, "Desk", 19.0, 1200)];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &delta_days,
        baseline_window(),
        DeltaBaselineState::Establishing {
          qualifying_days: 3,
          required_days: 7,
        },
        end,
      );

      let adjusted = result.ambient_adjusted;
      assert!(!adjusted.comparable);
      assert_eq!(adjusted.delta, None);
      // The evidence gathered so far is still reported, so the UI can
      // show progress rather than nothing.
      assert_eq!(adjusted.recent.delta_avg, Some(19.0));
      assert_eq!(
        adjusted.baseline_state,
        DeltaBaselineState::Establishing {
          qualifying_days: 3,
          required_days: 7,
        }
      );
    }

    #[test]
    fn the_delta_baseline_can_establish_while_the_absolute_one_is_still_establishing() {
      // The two lifecycles are independent in both directions. A machine
      // that has been idle-poor but ambient-rich can reach an ambient
      // reading first.
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = [summary(recent_start, 40.0, 120)];
      let delta_days = vec![delta_row(recent_start, "Desk", 14.0, 120)];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &delta_days,
        BaselineState::Establishing {
          qualifying_days: 2,
          required_days: 7,
        },
        established_delta(12.0),
        end,
      );

      assert_eq!(result.observation, CoolingDeltaObservation::Establishing);
      assert_eq!(result.delta, None);
      assert!(
        result.ambient_adjusted.comparable,
        "the ambient reading must not be gated on the absolute baseline"
      );
      assert_eq!(result.ambient_adjusted.delta, Some(2.0));
    }

    // ── the regression this lifecycle exists for ──

    #[test]
    fn ambient_started_after_the_absolute_baseline_still_becomes_comparable() {
      // The failure the independent lifecycle fixes, end to end and with
      // no artificial history: a machine ran for months with no ambient
      // sensor, so the absolute baseline pinned a window that has no
      // paired minutes and never can - the archive cannot grow ambient
      // readings for past days. Then a sensor is added.
      //
      // Anchoring the ΔT reading to the absolute baseline's window would
      // leave this machine non-comparable forever, however much ambient
      // data it goes on to collect. Deriving the ΔT baseline from its own
      // qualifying days lets it establish from the sensor's own first
      // week instead.
      let absolute_window_start = date(2026, 1, 1);
      let ambient_start = date(2026, 8, 1);
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);

      let mut summaries: Vec<_> = (0..7)
        .map(|offset| summary(absolute_window_start + Duration::days(offset), 30.0, 120))
        .collect();
      // The sensor arrives, and a week of paired idle minutes accrues.
      summaries.extend(
        (0..7).map(|offset| summary(ambient_start + Duration::days(offset), 40.0, 120)),
      );
      let mut delta_days: Vec<_> = (0..7)
        .map(|offset| {
          delta_row(ambient_start + Duration::days(offset), "Desk", 12.0, 120)
        })
        .collect();
      // ...and the recent window carries paired minutes too.
      summaries.push(summary(recent_start, 40.0, 120));
      delta_days.push(delta_row(recent_start, "Desk", 13.0, 120));
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      // Derived from the same rows the loader would read, rather than
      // handed in: this is the whole point of the test.
      let delta_baseline_state = derive_delta_baseline_state(&delta_days);
      let absolute = BaselineState::Established {
        idle_temperature_avg: 30.0,
        window_start_date: absolute_window_start,
        window_end_date: absolute_window_start + Duration::days(6),
        sample_minutes: 840,
      };

      let result =
        derive_baseline_delta(&idle, &delta_days, absolute, delta_baseline_state, end);

      let adjusted = result.ambient_adjusted;
      assert_eq!(
        adjusted.baseline_state.window(),
        Some((ambient_start, ambient_start + Duration::days(6))),
        "the ΔT baseline must establish over the days ambient data exists for, \
         not over the absolute baseline's ambient-free window"
      );
      assert!(
        adjusted.comparable,
        "a machine that added a sensor later must become comparable"
      );
      assert_eq!(adjusted.delta, Some(1.0));
    }

    #[test]
    fn the_absolute_window_being_ambient_free_does_not_hold_the_delta_reading_back() {
      // The same setup as above, stated as the property that used to
      // fail: nothing about the absolute baseline's window may appear in
      // the ΔT reading's inputs.
      let ambient_start = date(2026, 8, 1);
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let mut summaries: Vec<_> = (0..7)
        .map(|offset| summary(ambient_start + Duration::days(offset), 40.0, 120))
        .collect();
      let mut delta_days: Vec<_> = (0..7)
        .map(|offset| {
          delta_row(ambient_start + Duration::days(offset), "Desk", 12.0, 120)
        })
        .collect();
      summaries.push(summary(recent_start, 40.0, 120));
      delta_days.push(delta_row(recent_start, "Desk", 12.0, 120));
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();
      let delta_baseline_state = derive_delta_baseline_state(&delta_days);

      // Two absolute baselines a decade apart, both ambient-free.
      let near = BaselineState::Established {
        idle_temperature_avg: 30.0,
        window_start_date: date(2026, 1, 1),
        window_end_date: date(2026, 1, 7),
        sample_minutes: 840,
      };
      let far = BaselineState::Established {
        idle_temperature_avg: 55.0,
        window_start_date: date(2016, 1, 1),
        window_end_date: date(2016, 1, 7),
        sample_minutes: 840,
      };

      let with_near = derive_baseline_delta(
        &idle,
        &delta_days,
        near,
        delta_baseline_state.clone(),
        end,
      );
      let with_far =
        derive_baseline_delta(&idle, &delta_days, far, delta_baseline_state, end);

      assert_eq!(
        with_near.ambient_adjusted, with_far.ambient_adjusted,
        "the ambient reading must not depend on the absolute baseline at all"
      );
      assert!(with_near.ambient_adjusted.comparable);
    }
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
      use crate::persistence::cooling_baseline::COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS;
      for offset in 0..COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS {
        insert_idle_day(
          pool,
          start + Duration::days(offset as i64),
          temperature,
          COOLING_BASELINE_COMPARABLE_IDLE_MINUTES,
        )
        .await;
      }
    }

    #[tokio::test]
    async fn the_baseline_temperature_does_not_drift_when_its_source_rows_are_deleted() {
      // Same regression as cooling_baseline's own pinning test, but for
      // the delta loader: it must resolve the pinned row through the
      // shared resolver instead of re-deriving from whatever
      // `cooling_daily_summary` rows currently exist.
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      setup_tables(&pool).await;
      insert_establishing_days(&pool, date(2026, 8, 1), 42.0).await;
      // A comparable recent window so the delta actually gets computed.
      insert_idle_day(&pool, date(2026, 8, 19), 42.0, 120).await;

      let established = load_cooling_baseline_delta_from_pool(&pool, date(2026, 8, 20))
        .await
        .unwrap();
      assert_eq!(established.delta, Some(0.0));

      // Age out the rows the baseline was derived from, and record a
      // hotter stretch that would establish a different baseline value
      // if the pinned row were ignored.
      sqlx::query("DELETE FROM cooling_daily_summary")
        .execute(&pool)
        .await
        .unwrap();
      insert_establishing_days(&pool, date(2027, 6, 1), 70.0).await;
      insert_idle_day(&pool, date(2027, 6, 19), 42.0, 120).await;

      let after_cleanup = load_cooling_baseline_delta_from_pool(&pool, date(2027, 6, 20))
        .await
        .unwrap();
      assert_eq!(
        after_cleanup.delta,
        Some(0.0),
        "the pinned baseline must not drift when its source rows are deleted"
      );
    }
  }
}
