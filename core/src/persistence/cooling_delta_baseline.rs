//! The ambient-normalized cooling baseline: the thermal delta
//! (`ΔT = CPU package temperature − ambient temperature`) this machine
//! settles at when idle, used as the reference recent ΔT is compared
//! against (#2045).
//!
//! **This baseline establishes independently of the absolute one**, and
//! that independence is the whole point of the module existing.
//!
//! The obvious design - read the ΔT of whatever window the absolute
//! baseline pinned - is wrong, and wrong in a way that only shows up on
//! the path that matters most. Ambient collection commonly begins *after*
//! the absolute baseline was established: the user adds a sensor, or
//! #2045 ships to an install that already has months of history. The
//! absolute baseline's window is then a stretch of past days that never
//! had an ambient reading, and the archive cannot be made to grow one
//! retroactively - so no amount of ambient data accumulating from today
//! onward would ever make that window comparable. The ambient-adjusted
//! reading would sit at "not comparable" forever, on exactly the machines
//! that just started collecting the data for it.
//!
//! So the ΔT baseline runs the *same* establishment rule
//! ([`crate::persistence::cooling_baseline::derive_baseline_window`],
//! shared so the two cannot drift apart on what "established" means) over
//! its own projection: a qualifying day is one whose idle band recorded
//! at least [`COOLING_DELTA_BASELINE_QUALIFYING_MINUTES`] of *paired*
//! minutes. Its window therefore starts wherever ambient data actually
//! starts, which is generally later than - and sometimes disjoint from -
//! the absolute baseline's.
//!
//! Like the absolute baseline it is **established by derivation, then
//! pinned** into a single-row table, for the same reason: the
//! `cooling_daily_summary` rows it was derived from are eventually
//! cleaned up, and re-deriving forever would let "the first N qualifying
//! days" silently advance as they aged out.
//!
//! It gets its *own* table rather than columns on `cooling_baseline`.
//! Pinning there is write-once (`INSERT OR IGNORE` against a
//! `CHECK (id = 1)` row), and that is precisely what makes an established
//! baseline undriftable. Two baselines that establish at different times
//! cannot share one write-once row: the second would have to arrive as an
//! `UPDATE ... WHERE <column> IS NULL`, a weaker rule that has to be got
//! right rather than being structurally impossible to get wrong. Separate
//! rows keep each one's insert-once invariant exactly as strong as it was.

use chrono::NaiveDate;

use crate::persistence::cooling_baseline::{
  BaselineWindow, COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS, DailyBaselineSample,
  derive_baseline_window,
};
use crate::persistence::cooling_rollup::{CpuLoadBand, DailyCoolingSummary};

/// Minimum *paired* idle minutes a completed local day must carry before
/// it counts toward the ΔT baseline.
///
/// The same bar as the absolute baseline's qualifying day
/// (`COOLING_BASELINE_QUALIFYING_IDLE_MINUTES`), deliberately restated
/// rather than aliased: this counts minutes that needed both a hardware
/// and an ambient reading, so it is a materially harder bar to clear on
/// the same machine, and the two should be able to move apart if
/// experience says they should.
pub const COOLING_DELTA_BASELINE_QUALIFYING_MINUTES: u32 = 30;

/// Lifecycle of the ΔT baseline, mirroring
/// [`crate::persistence::cooling_baseline::BaselineState`].
///
/// There is deliberately no third "this machine has no ambient sensor"
/// variant, following the same reasoning the absolute baseline records:
/// such a machine is `Establishing { qualifying_days: 0, .. }`, which the
/// UI renders with the same "n of N days" copy. Whether an environmental
/// sensor exists at all is already answered by Ambient Sensor
/// Availability (#2043), so this enum does not need to answer it too.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeltaBaselineState {
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    /// The baseline ΔT in degrees, sample-minute weighted across the
    /// window.
    delta_temperature_avg: f32,
    window_start_date: NaiveDate,
    window_end_date: NaiveDate,
    sample_minutes: u32,
  },
}

impl DeltaBaselineState {
  /// The window this baseline was established over, or `None` while
  /// still establishing. Callers that re-aggregate the window per band
  /// (the load-band comparison) read it through here.
  pub fn window(&self) -> Option<(NaiveDate, NaiveDate)> {
    match *self {
      Self::Established {
        window_start_date,
        window_end_date,
        ..
      } => Some((window_start_date, window_end_date)),
      Self::Establishing { .. } => None,
    }
  }
}

/// The established ΔT baseline as pinned into `cooling_delta_baseline`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EstablishedDeltaBaseline {
  pub delta_temperature_avg: f32,
  pub window_start_date: NaiveDate,
  pub window_end_date: NaiveDate,
  pub sample_minutes: u32,
}

impl EstablishedDeltaBaseline {
  /// The record to pin for `state`, or `None` while still establishing.
  pub fn from_state(state: &DeltaBaselineState) -> Option<Self> {
    match *state {
      DeltaBaselineState::Established {
        delta_temperature_avg,
        window_start_date,
        window_end_date,
        sample_minutes,
      } => Some(Self {
        delta_temperature_avg,
        window_start_date,
        window_end_date,
        sample_minutes,
      }),
      DeltaBaselineState::Establishing { .. } => None,
    }
  }

  pub fn into_state(self) -> DeltaBaselineState {
    DeltaBaselineState::Established {
      delta_temperature_avg: self.delta_temperature_avg,
      window_start_date: self.window_start_date,
      window_end_date: self.window_end_date,
      sample_minutes: self.sample_minutes,
    }
  }
}

/// Establish the ΔT baseline from the first
/// [`COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS`] days whose idle band
/// recorded at least [`COOLING_DELTA_BASELINE_QUALIFYING_MINUTES`]
/// paired minutes.
///
/// `days` must be ordered by date ascending, as
/// `database::cooling_daily_summary::select_all_daily_cooling_summaries`
/// returns them.
pub fn derive_delta_baseline_state(days: &[DailyCoolingSummary]) -> DeltaBaselineState {
  let required_days = COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS;
  let samples: Vec<_> = days
    .iter()
    .map(|day| {
      let idle = day.ambient.band(CpuLoadBand::Idle);
      DailyBaselineSample {
        date: day.date,
        value: idle.avg,
        sample_minutes: idle.sample_minutes,
      }
    })
    .collect();

  match derive_baseline_window(
    &samples,
    COOLING_DELTA_BASELINE_QUALIFYING_MINUTES,
    required_days,
  ) {
    BaselineWindow::Established {
      value,
      start_date,
      end_date,
      sample_minutes,
    } => DeltaBaselineState::Established {
      delta_temperature_avg: value,
      window_start_date: start_date,
      window_end_date: end_date,
      sample_minutes,
    },
    BaselineWindow::Establishing { qualifying_days } => {
      DeltaBaselineState::Establishing {
        qualifying_days,
        required_days,
      }
    }
  }
}

/// Resolve the ΔT baseline lifecycle: the pinned row if one exists,
/// otherwise derive it from `days` and pin it the moment it establishes.
///
/// Every loader that needs this state must go through here rather than
/// calling [`derive_delta_baseline_state`] directly, for the same reason
/// the absolute baseline insists on its resolver: otherwise it ignores
/// the pinned row and drifts as the rollup rows behind the original
/// establishment age out.
pub(crate) async fn resolve_delta_baseline_state_from_pool(
  pool: &sqlx::SqlitePool,
  days: &[DailyCoolingSummary],
) -> Result<DeltaBaselineState, sqlx::Error> {
  use crate::infrastructure::database;

  match database::cooling_delta_baseline::select_established_delta_baseline_from_pool(
    pool,
  )
  .await?
  {
    Some(pinned) => Ok(pinned.into_state()),
    None => {
      let derived = derive_delta_baseline_state(days);
      if let Some(baseline) = EstablishedDeltaBaseline::from_state(&derived) {
        // Write-once bookkeeping, not part of the answer - a transient
        // failure must not turn a valid derivation into a read error.
        // Retried on the next resolution. Same rule as the absolute
        // baseline's pin.
        if let Err(e) =
          database::cooling_delta_baseline::insert_established_delta_baseline_from_pool(
            pool,
            &baseline,
            chrono::Utc::now(),
          )
          .await
        {
          crate::log_error!(
            "Failed to pin the established ΔT cooling baseline; retrying on the next resolution",
            "persistence::cooling_delta_baseline::resolve_delta_baseline_state_from_pool",
            Some(e.to_string())
          );
        }
      }
      Ok(derived)
    }
  }
}

/// Resolve — and, on first establishment, pin — the ΔT baseline in the
/// background, mirroring
/// [`crate::persistence::cooling_baseline::ensure_baseline_pinned`]. The
/// rollup worker calls this after every catch-up pass so establishment
/// never depends on Cooling Insight being opened before retention
/// cleanup erases the establishment-window rows. Failures are logged and
/// retried on the next pass.
pub(crate) async fn ensure_delta_baseline_pinned() {
  use crate::infrastructure::database;

  let resolve = async {
    let pool = database::db::get_pool().await?;
    let days =
      database::cooling_daily_summary::select_all_daily_cooling_summaries_from_pool(
        &pool,
      )
      .await?;
    resolve_delta_baseline_state_from_pool(&pool, &days).await
  };

  if let Err(e) = resolve.await {
    crate::log_error!(
      "Failed to resolve the ΔT cooling baseline after a rollup catch-up",
      "persistence::cooling_delta_baseline::ensure_delta_baseline_pinned",
      Some(e.to_string())
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::persistence::cooling_rollup::{
    AmbientDeltaSummary, BandSummary, PowerSummary,
  };

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

  /// One rollup row whose idle band carries `delta` over `minutes`
  /// paired minutes, or no paired minutes at all when `delta` is `None`.
  pub(crate) fn day(date: NaiveDate, delta: Option<(f32, u32)>) -> DailyCoolingSummary {
    DailyCoolingSummary {
      date,
      coverage_minutes: 1440,
      idle: band(40.0, 600),
      low: BandSummary::default(),
      mid: BandSummary::default(),
      high: BandSummary::default(),
      power: PowerSummary::default(),
      ambient: match delta {
        Some((avg, minutes)) => AmbientDeltaSummary {
          coverage_minutes: minutes,
          idle: band(avg, minutes),
          ..AmbientDeltaSummary::default()
        },
        None => AmbientDeltaSummary::default(),
      },
    }
  }

  /// `count` consecutive qualifying days starting at `start`.
  fn qualifying_days(
    start: NaiveDate,
    count: i64,
    delta: f32,
  ) -> Vec<DailyCoolingSummary> {
    (0..count)
      .map(|offset| {
        day(
          start + chrono::Duration::days(offset),
          Some((delta, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
        )
      })
      .collect()
  }

  #[test]
  fn a_machine_with_no_ambient_data_stays_establishing_at_zero() {
    let days: Vec<_> = (0..30)
      .map(|offset| day(date(2026, 8, 1) + chrono::Duration::days(offset), None))
      .collect();

    assert_eq!(
      derive_delta_baseline_state(&days),
      DeltaBaselineState::Establishing {
        qualifying_days: 0,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn the_required_number_of_qualifying_days_establishes_the_baseline() {
    let start = date(2026, 8, 1);
    let days = qualifying_days(
      start,
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64,
      12.0,
    );

    let state = derive_delta_baseline_state(&days);

    assert_eq!(
      state.window(),
      Some((
        start,
        start
          + chrono::Duration::days(COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 - 1),
      ))
    );
    let DeltaBaselineState::Established {
      delta_temperature_avg,
      sample_minutes,
      ..
    } = state
    else {
      panic!("expected an established ΔT baseline");
    };
    assert_eq!(delta_temperature_avg, 12.0);
    assert_eq!(
      sample_minutes,
      COOLING_DELTA_BASELINE_QUALIFYING_MINUTES
        * COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS
    );
  }

  #[test]
  fn one_day_short_of_the_requirement_is_still_establishing() {
    let days = qualifying_days(
      date(2026, 8, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 - 1,
      12.0,
    );

    assert_eq!(
      derive_delta_baseline_state(&days),
      DeltaBaselineState::Establishing {
        qualifying_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn a_day_with_too_few_paired_minutes_does_not_qualify() {
    // The bar is on *paired* minutes: a day whose ambient sensor was only
    // briefly reachable cannot qualify however much idle time it had.
    let short = COOLING_DELTA_BASELINE_QUALIFYING_MINUTES - 1;
    let days: Vec<_> = (0..COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64)
      .map(|offset| {
        day(
          date(2026, 8, 1) + chrono::Duration::days(offset),
          Some((12.0, short)),
        )
      })
      .collect();

    assert_eq!(
      derive_delta_baseline_state(&days),
      DeltaBaselineState::Establishing {
        qualifying_days: 0,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn the_window_skips_non_qualifying_days_without_counting_them() {
    // Ambient coverage that comes and goes: the window is the first N
    // *qualifying* days, which may span a longer calendar range.
    let start = date(2026, 8, 1);
    let mut days = Vec::new();
    for offset in 0..(COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 * 2) {
      let d = start + chrono::Duration::days(offset);
      // Every other day has no ambient coverage at all.
      days.push(if offset % 2 == 0 {
        day(d, Some((12.0, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)))
      } else {
        day(d, None)
      });
    }

    let state = derive_delta_baseline_state(&days);

    // 7 qualifying days at every other day spans 13 calendar days.
    assert_eq!(
      state.window(),
      Some((start, start + chrono::Duration::days(12)))
    );
  }

  #[test]
  fn the_window_starts_where_ambient_starts_not_where_history_starts() {
    // The regression the whole module exists for: months of history with
    // no ambient, then a sensor appears. The ΔT baseline must establish
    // from the sensor's own first days, not report "not comparable"
    // forever because an older window never had ambient.
    let history_start = date(2026, 1, 1);
    let ambient_start = date(2026, 8, 1);
    let mut days: Vec<_> = (0..90)
      .map(|offset| day(history_start + chrono::Duration::days(offset), None))
      .collect();
    days.extend(qualifying_days(
      ambient_start,
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64,
      12.0,
    ));

    let state = derive_delta_baseline_state(&days);

    assert_eq!(
      state.window(),
      Some((
        ambient_start,
        ambient_start
          + chrono::Duration::days(COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 - 1),
      )),
      "the ΔT window must begin at the first day ambient data actually exists"
    );
  }

  #[test]
  fn later_hotter_days_do_not_move_an_established_window() {
    // "First N qualifying days", never the most recent ones - the
    // property that makes the derived value stable enough to pin.
    let start = date(2026, 8, 1);
    let mut days = qualifying_days(
      start,
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64,
      12.0,
    );
    days.extend(qualifying_days(
      date(2026, 10, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64,
      40.0,
    ));

    let DeltaBaselineState::Established {
      delta_temperature_avg,
      window_start_date,
      ..
    } = derive_delta_baseline_state(&days)
    else {
      panic!("expected an established ΔT baseline");
    };

    assert_eq!(window_start_date, start);
    assert_eq!(delta_temperature_avg, 12.0);
  }
}
