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

use crate::persistence::cooling_baseline::{
  BaselineState, DailyIdleSample, RecentIdleSummary, summarize_recent_idle,
};

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
}

/// Derive the baseline delta from every summarized day's idle-band
/// facts and the current baseline lifecycle state.
///
/// `window_end_date` is the most recent completed local day (yesterday),
/// matching [`crate::persistence::cooling_baseline::derive_cooling_baseline`].
pub fn derive_baseline_delta(
  days: &[DailyIdleSample],
  baseline_state: BaselineState,
  window_end_date: NaiveDate,
) -> CoolingBaselineDelta {
  let recent = summarize_recent_idle(days, window_end_date);

  let Some(baseline_temperature) = established_temperature(baseline_state) else {
    return CoolingBaselineDelta {
      baseline_state,
      recent,
      delta: None,
      observation: CoolingDeltaObservation::Establishing,
      daily_deltas: Vec::new(),
      sustained_days: 0,
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
    };
  }

  // `recent.is_comparable()` guarantees an average is present.
  let delta = recent.idle_temperature_avg.unwrap() - baseline_temperature;
  let (daily_deltas, sustained_days) =
    trailing_daily_deltas(days, baseline_temperature, window_end_date);
  let observation = classify_observation(delta, sustained_days);

  CoolingBaselineDelta {
    baseline_state,
    recent,
    delta: Some(delta),
    observation,
    daily_deltas,
    sustained_days,
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

fn classify_observation(delta: f32, sustained_days: u32) -> CoolingDeltaObservation {
  if sustained_days < COOLING_DELTA_SUSTAIN_DAYS {
    return CoolingDeltaObservation::WithinRange;
  }
  if delta >= COOLING_DELTA_LARGE_RISE_THRESHOLD {
    CoolingDeltaObservation::SustainedLargeRise
  } else if delta >= COOLING_DELTA_MILD_RISE_THRESHOLD {
    CoolingDeltaObservation::SustainedMildRise
  } else {
    CoolingDeltaObservation::WithinRange
  }
}

/// Walk backward day by day from `window_end_date`, recomputing the
/// trailing 7-day idle average at each day and comparing it against
/// `baseline_temperature`. Stops (without including that day) at the
/// first day whose trailing window is not comparable, and stops
/// (including that day) at the first day whose delta drops below
/// [`COOLING_DELTA_MILD_RISE_THRESHOLD`] - the walk only needs to go far
/// enough to explain `sustained_days`, not the whole rollup history.
///
/// Returned oldest-first, so the series reads left-to-right as a
/// timeline.
fn trailing_daily_deltas(
  days: &[DailyIdleSample],
  baseline_temperature: f32,
  window_end_date: NaiveDate,
) -> (Vec<DailyDelta>, u32) {
  let mut series = Vec::new();
  let mut sustained_days = 0u32;
  let mut cursor = window_end_date;

  loop {
    let window = summarize_recent_idle(days, cursor);
    if !window.is_comparable() {
      break;
    }
    // `is_comparable()` guarantees an average is present.
    let delta = window.idle_temperature_avg.unwrap() - baseline_temperature;
    let sustained = delta >= COOLING_DELTA_MILD_RISE_THRESHOLD;
    series.push(DailyDelta {
      date: cursor,
      delta,
    });
    if !sustained {
      break;
    }
    sustained_days += 1;
    cursor -= Duration::days(1);
  }

  series.reverse();
  (series, sustained_days)
}

/// [`derive_baseline_delta`] over the whole `cooling_daily_summary`
/// table.
pub async fn load_cooling_baseline_delta() -> Result<CoolingBaselineDelta, sqlx::Error> {
  use crate::infrastructure::database;
  use crate::persistence::cooling_baseline::derive_baseline_state;

  let days = database::cooling_daily_summary::select_daily_idle_samples().await?;
  let baseline_state = derive_baseline_state(&days);
  let yesterday = chrono::Local::now().date_naive() - Duration::days(1);

  Ok(derive_baseline_delta(&days, baseline_state, yesterday))
}

#[cfg(test)]
mod tests {
  use super::*;
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
    let result = derive_baseline_delta(&[], state, date(2026, 8, 20));

    assert_eq!(result.observation, CoolingDeltaObservation::Establishing);
    assert_eq!(result.delta, None);
    assert_eq!(result.sustained_days, 0);
    assert!(result.daily_deltas.is_empty());
  }

  #[test]
  fn an_established_baseline_without_recent_idle_evidence_is_not_comparable() {
    let result = derive_baseline_delta(&[], established(30.0), date(2026, 8, 20));

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
    let result = derive_baseline_delta(&days, established(30.0), end);

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
    let result = derive_baseline_delta(&days, established(30.0), end);

    assert!((result.delta.unwrap() - 5.0).abs() < 0.001);
    assert_eq!(result.sustained_days, 1);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
  }

  // ── sustain-length boundary ──

  #[test]
  fn two_sustained_days_are_not_yet_enough_to_report_a_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 2, 35.0);
    let result = derive_baseline_delta(&days, established(30.0), end);

    assert_eq!(result.sustained_days, 2);
    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
  }

  #[test]
  fn three_sustained_days_at_a_mild_rise_report_sustained_mild_rise() {
    let end = date(2026, 8, 20);
    let days = days_ending_at(end, 3, 35.0);
    let result = derive_baseline_delta(&days, established(30.0), end);

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
    let result = derive_baseline_delta(&days, established(30.0), end);

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
    let result = derive_baseline_delta(&days, established(30.0), end);

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

    let result = derive_baseline_delta(&days, established(30.0), end);

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

    let result = derive_baseline_delta(&days, established(30.0), end);

    assert_eq!(result.sustained_days, 2);
    assert_eq!(result.daily_deltas.len(), 2);
  }
}
