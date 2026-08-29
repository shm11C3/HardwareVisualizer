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

  let days =
    database::cooling_daily_summary::select_daily_idle_samples_from_pool(pool).await?;
  let baseline_state = resolve_baseline_state_from_pool(pool, &days).await?;
  let yesterday = today - Duration::days(1);

  Ok(derive_baseline_delta(&days, baseline_state, yesterday))
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

    let result = derive_baseline_delta(&days, established(30.0), end);

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

    let result = derive_baseline_delta(&days, established(30.0), end);

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

    let result = derive_baseline_delta(&days, established(30.0), end);

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

    let result = derive_baseline_delta(&days, established(30.0), end);

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

    let result = derive_baseline_delta(&days, established(30.0), end);

    assert_eq!(result.observation, CoolingDeltaObservation::WithinRange);
    assert_eq!(result.sustained_days, 2);
  }

  // ── pinned baseline (DB-backed) ──

  mod pinned_baseline {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_tables(pool: &SqlitePool) {
      sqlx::query(
        "CREATE TABLE cooling_daily_summary (
          date TEXT PRIMARY KEY,
          idle_cpu_temperature_avg REAL,
          idle_cpu_temperature_max REAL,
          idle_cpu_temperature_min REAL,
          idle_sample_minutes INTEGER NOT NULL DEFAULT 0,
          low_cpu_temperature_avg REAL,
          low_cpu_temperature_max REAL,
          low_cpu_temperature_min REAL,
          low_sample_minutes INTEGER NOT NULL DEFAULT 0,
          mid_cpu_temperature_avg REAL,
          mid_cpu_temperature_max REAL,
          mid_cpu_temperature_min REAL,
          mid_sample_minutes INTEGER NOT NULL DEFAULT 0,
          high_cpu_temperature_avg REAL,
          high_cpu_temperature_max REAL,
          high_cpu_temperature_min REAL,
          high_sample_minutes INTEGER NOT NULL DEFAULT 0,
          coverage_minutes INTEGER NOT NULL
        )",
      )
      .execute(pool)
      .await
      .unwrap();
      sqlx::query(
        "CREATE TABLE cooling_baseline (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          window_start_date TEXT NOT NULL,
          window_end_date TEXT NOT NULL,
          idle_temperature_avg REAL NOT NULL,
          sample_minutes INTEGER NOT NULL,
          established_at TEXT NOT NULL
        )",
      )
      .execute(pool)
      .await
      .unwrap();
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
