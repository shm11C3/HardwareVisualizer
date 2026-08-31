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
  BandDeltaWindowSummary, band_delta_window_summary,
};
use crate::persistence::cooling_baseline::{
  BaselineState, COOLING_BASELINE_RECENT_WINDOW_DAYS, DailyIdleSample, RecentIdleSummary,
  summarize_recent_idle,
};
use crate::persistence::cooling_rollup::{CpuLoadBand, DailyCoolingSummary};

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
/// `delta` subtracts two ΔT window averages, which is legitimate where
/// subtracting a CPU summary from an ambient summary is not: both sides
/// are already per-minute ΔT values paired before aggregation, so this
/// compares one period against another rather than reconstructing a
/// pairing that never happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientAdjustedBaselineDelta {
  /// The pinned baseline window's idle ΔT.
  pub baseline: BandDeltaWindowSummary,
  /// The trailing recent window's idle ΔT, over the same days
  /// [`CoolingBaselineDelta::recent`] covers.
  pub recent: BandDeltaWindowSummary,
  /// `recent - baseline`, or `None` unless `comparable`.
  pub delta: Option<f32>,
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
  /// The ambient-normalized reading of the same drift (#2045), or `None`
  /// when no day in either window recorded an idle ΔT minute - the normal
  /// state on an install with no environmental sensor, and what keeps
  /// every field above exactly what it was before #2045.
  pub ambient_adjusted: Option<AmbientAdjustedBaselineDelta>,
}

/// Derive the baseline delta from every summarized day's idle-band
/// facts and the current baseline lifecycle state.
///
/// `window_end_date` is the most recent completed local day (yesterday),
/// matching [`crate::persistence::cooling_baseline::derive_cooling_baseline`].
/// `ambient_days` carries the same days' full rollup rows, which is where
/// the per-band ΔT lives (#2045). It is a second slice rather than a
/// replacement for `days` because the absolute-temperature verdict above
/// must not change shape at all when ambient data appears: pass an empty
/// slice and every field but `ambient_adjusted` is computed exactly as it
/// was before #2045.
pub fn derive_baseline_delta(
  days: &[DailyIdleSample],
  ambient_days: &[DailyCoolingSummary],
  baseline_state: BaselineState,
  window_end_date: NaiveDate,
) -> CoolingBaselineDelta {
  let recent = summarize_recent_idle(days, window_end_date);
  // Derived regardless of the absolute verdict below: the two readings
  // answer different questions and one being unavailable says nothing
  // about the other.
  let ambient_adjusted =
    derive_ambient_adjusted(ambient_days, baseline_state, window_end_date);

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

/// The idle ΔT of the pinned baseline window against the idle ΔT of the
/// trailing recent window (#2045), or `None` when neither window recorded
/// a single idle ΔT minute.
///
/// **The ΔT baseline is deliberately not pinned**, unlike the absolute
/// idle baseline it sits beside. The absolute baseline is pinned because
/// re-deriving it would let "the first N qualifying days" silently advance
/// as the original days aged out. That failure mode does not apply here:
/// this reads the ΔT of the *already-pinned* window, whose
/// `cooling_daily_summary` rows are exempt from retention cleanup for
/// exactly as long as the baseline names them (see
/// `cooling_rollup::cleanup_old_data`), so the days it averages cannot
/// change underneath it.
///
/// Not pinning also buys something pinning would forfeit. Ambient
/// collection commonly starts *after* the absolute baseline was
/// established - a user adds a sensor, or #2045 ships to an install that
/// already has months of history - and the backfill then fills the pinned
/// window's ΔT columns in from the one-minute archive. A ΔT baseline
/// captured at establishment time would have frozen "no ambient data"
/// permanently and never noticed.
fn derive_ambient_adjusted(
  days: &[DailyCoolingSummary],
  baseline_state: BaselineState,
  window_end_date: NaiveDate,
) -> Option<AmbientAdjustedBaselineDelta> {
  let BaselineState::Established {
    window_start_date,
    window_end_date: baseline_end,
    ..
  } = baseline_state
  else {
    // No baseline window exists yet, so there is nothing to normalize
    // against - the same gate the absolute reading applies.
    return None;
  };

  let recent_start =
    window_end_date - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
  let baseline =
    band_delta_window_summary(days, CpuLoadBand::Idle, window_start_date, baseline_end);
  let recent =
    band_delta_window_summary(days, CpuLoadBand::Idle, recent_start, window_end_date);

  if baseline.sample_minutes == 0 && recent.sample_minutes == 0 {
    return None;
  }

  let comparable = baseline.is_comparable() && recent.is_comparable();
  Some(AmbientAdjustedBaselineDelta {
    // `comparable` implies both averages are present.
    delta: comparable.then(|| recent.delta_avg.unwrap() - baseline.delta_avg.unwrap()),
    baseline,
    recent,
    comparable,
  })
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

  // Reads the full rollup rows rather than the idle-only projection since
  // #2045: the ambient-adjusted reading needs the per-band ΔT columns, and
  // the idle facts it also needs are already on the same row, so this is
  // one wider query rather than two. The table holds at most
  // `COOLING_DAILY_SUMMARY_RETENTION_DAYS` rows - see
  // `select_daily_idle_samples` for why reading all of them is cheap.
  let summaries =
    database::cooling_daily_summary::select_all_daily_cooling_summaries_from_pool(pool)
      .await?;
  let days: Vec<_> = summaries.iter().map(to_idle_sample).collect();
  let baseline_state = resolve_baseline_state_from_pool(pool, &days).await?;
  let yesterday = today - Duration::days(1);

  Ok(derive_baseline_delta(
    &days,
    &summaries,
    baseline_state,
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
    COOLING_BASELINE_DDL, COOLING_DAILY_SUMMARY_DDL, create_tables,
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
    let result = derive_baseline_delta(&[], &[], state, date(2026, 8, 20));

    assert_eq!(result.observation, CoolingDeltaObservation::Establishing);
    assert_eq!(result.delta, None);
    assert_eq!(result.sustained_days, 0);
    assert!(result.daily_deltas.is_empty());
  }

  #[test]
  fn an_established_baseline_without_recent_idle_evidence_is_not_comparable() {
    let result = derive_baseline_delta(&[], &[], established(30.0), date(2026, 8, 20));

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
    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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
    let result = derive_baseline_delta(&days, &[], established(30.0), end);

    assert!((result.delta.unwrap() - 5.0).abs() < 0.001);
    assert_eq!(result.sustained_days, 1);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
  }

  // ── sustain-length boundary ──

  #[test]
  fn two_sustained_days_are_not_yet_enough_to_report_a_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 2, 35.0);
    let result = derive_baseline_delta(&days, &[], established(30.0), end);

    assert_eq!(result.sustained_days, 2);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
  }

  #[test]
  fn three_sustained_days_at_a_mild_rise_report_sustained_mild_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 3, 35.0);
    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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
    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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
    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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

    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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

    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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

    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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

    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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

    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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

    let result = derive_baseline_delta(&days, &[], established(30.0), end);

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

    let result = derive_baseline_delta(&days, &[], established(30.0), end);

    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
    assert_eq!(result.sustained_days, 2);
  }

  // ── ambient-adjusted baseline delta (#2045) ──

  mod ambient_adjusted {
    use super::*;
    use crate::persistence::cooling_band_comparison::COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES;
    use crate::persistence::cooling_rollup::{
      AmbientDeltaSummary, BandSummary, PowerSummary,
    };

    fn band(avg: f32, minutes: u32) -> BandSummary {
      BandSummary {
        avg: Some(avg),
        max: Some(avg + 1.0),
        min: Some(avg - 1.0),
        sample_minutes: minutes,
      }
    }

    /// One rollup row carrying an idle band and, optionally, an idle ΔT.
    fn summary(
      date: NaiveDate,
      temperature: f32,
      minutes: u32,
      delta: Option<(f32, u32)>,
    ) -> DailyCoolingSummary {
      DailyCoolingSummary {
        date,
        coverage_minutes: 1440,
        idle: band(temperature, minutes),
        low: BandSummary::default(),
        mid: BandSummary::default(),
        high: BandSummary::default(),
        power: PowerSummary::default(),
        ambient: match delta {
          Some((avg, delta_minutes)) => AmbientDeltaSummary {
            coverage_minutes: delta_minutes,
            idle: band(avg, delta_minutes),
            ..AmbientDeltaSummary::default()
          },
          None => AmbientDeltaSummary::default(),
        },
      }
    }

    /// The baseline window used throughout: a single day, 8-01.
    fn baseline_window() -> BaselineState {
      BaselineState::Established {
        idle_temperature_avg: 30.0,
        window_start_date: date(2026, 8, 1),
        window_end_date: date(2026, 8, 1),
        sample_minutes: 210,
      }
    }

    #[test]
    fn an_install_with_no_ambient_data_offers_no_ambient_adjusted_reading() {
      // The zero-ambient invariant: every existing field keeps its value
      // and the new one is simply absent.
      let end = date(2026, 8, 20);
      let idle = days_ending_at(end, 5, 35.0);
      let summaries: Vec<_> = idle
        .iter()
        .map(|d| {
          summary(
            d.date,
            d.idle_temperature_avg.unwrap(),
            d.idle_sample_minutes,
            None,
          )
        })
        .collect();

      let with_ambient_slice =
        derive_baseline_delta(&idle, &summaries, baseline_window(), end);
      let without_ambient_slice =
        derive_baseline_delta(&idle, &[], baseline_window(), end);

      assert_eq!(with_ambient_slice.ambient_adjusted, None);
      assert_eq!(
        with_ambient_slice, without_ambient_slice,
        "a rollup carrying no ΔT must answer exactly as one with no ambient columns at all"
      );
      // ...and the absolute verdict is the one it always was.
      assert_eq!(
        with_ambient_slice.observation,
        CoolingDeltaObservation::SustainedMildRise
      );
    }

    #[test]
    fn a_flat_delta_under_a_rising_absolute_temperature_reports_no_ambient_drift() {
      // The reading the whole feature exists for. Absolute idle climbed
      // 10 K between the windows while ΔT held at 12 K: the room warmed,
      // the cooling did not degrade.
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = vec![
        summary(date(2026, 8, 1), 30.0, 120, Some((12.0, 120))),
        summary(recent_start, 40.0, 120, Some((12.0, 120))),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(&idle, &summaries, baseline_window(), end);

      let adjusted = result.ambient_adjusted.expect("ambient data exists");
      assert!(adjusted.comparable);
      assert_eq!(adjusted.baseline.delta_avg, Some(12.0));
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
      let summaries = vec![
        summary(date(2026, 8, 1), 30.0, 120, Some((12.0, 120))),
        summary(recent_start, 40.0, 120, Some((19.5, 120))),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(&idle, &summaries, baseline_window(), end);

      let adjusted = result.ambient_adjusted.unwrap();
      assert!((adjusted.delta.unwrap() - 7.5).abs() < 0.001);
    }

    #[test]
    fn a_thin_window_reports_present_but_not_comparable_with_no_delta() {
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let short = COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES - 1;
      let summaries = vec![
        summary(date(2026, 8, 1), 30.0, 120, Some((12.0, short))),
        summary(recent_start, 40.0, 120, Some((19.0, 120))),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(&idle, &summaries, baseline_window(), end);

      let adjusted = result.ambient_adjusted.unwrap();
      assert!(!adjusted.comparable);
      assert_eq!(
        adjusted.delta, None,
        "no number may be reported from a window this thin"
      );
      assert_eq!(adjusted.baseline.sample_minutes, short);
    }

    #[test]
    fn no_established_baseline_means_no_ambient_adjusted_reading_either() {
      // The ΔT reading is anchored to the pinned baseline's window, so
      // it cannot exist before that window does.
      let end = date(2026, 8, 20);
      let summaries = vec![summary(date(2026, 8, 1), 30.0, 120, Some((12.0, 120)))];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(
        &idle,
        &summaries,
        BaselineState::Establishing {
          qualifying_days: 3,
          required_days: 7,
        },
        end,
      );

      assert_eq!(result.observation, CoolingDeltaObservation::Establishing);
      assert_eq!(result.ambient_adjusted, None);
    }

    #[test]
    fn the_delta_baseline_reads_the_pinned_window_not_the_first_ambient_days() {
      // The consequence of *not* pinning the ΔT baseline separately: it
      // is always the ΔT of whatever days the pinned window names, so a
      // later, hotter stretch outside that window cannot pull it.
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
      let summaries = vec![
        summary(date(2026, 8, 1), 30.0, 120, Some((12.0, 120))),
        // A day between the two windows, with a wildly different ΔT.
        summary(date(2026, 8, 8), 60.0, 120, Some((90.0, 120))),
        summary(recent_start, 40.0, 120, Some((12.0, 120))),
      ];
      let idle: Vec<_> = summaries.iter().map(to_idle_sample).collect();

      let result = derive_baseline_delta(&idle, &summaries, baseline_window(), end);

      let adjusted = result.ambient_adjusted.unwrap();
      assert_eq!(
        adjusted.baseline.delta_avg,
        Some(12.0),
        "only the pinned window's own days may form the ΔT baseline"
      );
      assert_eq!(adjusted.baseline.sample_minutes, 120);
    }

    #[test]
    fn ambient_added_after_the_baseline_was_pinned_still_produces_a_reading() {
      // Why the ΔT baseline is derived rather than captured at
      // establishment: the user added a sensor later and the backfill
      // filled the pinned window's ΔT columns in from the one-minute
      // archive. A value frozen at establishment time would have said
      // "no ambient data" forever.
      let end = date(2026, 8, 20);
      let recent_start =
        end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);

      // Before the backfill: the pinned window has no ΔT at all.
      let before = vec![
        summary(date(2026, 8, 1), 30.0, 120, None),
        summary(recent_start, 40.0, 120, Some((12.0, 120))),
      ];
      let idle_before: Vec<_> = before.iter().map(to_idle_sample).collect();
      let result_before =
        derive_baseline_delta(&idle_before, &before, baseline_window(), end);
      assert!(
        !result_before.ambient_adjusted.unwrap().comparable,
        "with only the recent side backfilled there is nothing to compare against"
      );

      // After the backfill reached the pinned window.
      let after = vec![
        summary(date(2026, 8, 1), 30.0, 120, Some((11.0, 120))),
        summary(recent_start, 40.0, 120, Some((12.0, 120))),
      ];
      let idle_after: Vec<_> = after.iter().map(to_idle_sample).collect();
      let result_after =
        derive_baseline_delta(&idle_after, &after, baseline_window(), end);

      let adjusted = result_after.ambient_adjusted.unwrap();
      assert!(adjusted.comparable);
      assert_eq!(adjusted.delta, Some(1.0));
    }
  }

  // ── pinned baseline (DB-backed) ──

  mod pinned_baseline {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_tables(pool: &SqlitePool) {
      create_tables(pool, &[COOLING_DAILY_SUMMARY_DDL, COOLING_BASELINE_DDL]).await;
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
