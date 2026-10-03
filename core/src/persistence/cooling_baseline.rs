//! Cooling baseline: the idle CPU temperature this machine settles at
//! when it is not doing anything, used later as the reference that recent
//! idle temperatures are compared against (#1666 Phase 1 MVP).
//!
//! The baseline is *established by derivation, then pinned*: it is
//! derived from `cooling_daily_summary` (see
//! [`crate::persistence::cooling_rollup`]) only while it has not been
//! established yet, and the first derivation that reaches
//! [`BaselineState::Established`] is written once into the single-row
//! `cooling_baseline` table. Every later read returns that pinned row.
//!
//! Deriving expresses the idle requirement cheaply: the rollup already
//! records, per completed local day, how many one-minute samples that day
//! spent in the
//! [`CpuLoadBand::Idle`](crate::persistence::cooling_rollup::CpuLoadBand)
//! band, so "low load sustained for a minimum duration" becomes a per-day
//! minimum ([`COOLING_BASELINE_QUALIFYING_IDLE_MINUTES`]) instead of a
//! run-length scan over one-minute archive rows.
//!
//! Pinning is what keeps it *stable*. The baseline is defined as the
//! first [`COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS`] qualifying days,
//! which never change once they are in the past - but the rows recording
//! them do: `cooling_daily_summary` is cleaned up after
//! [`COOLING_DAILY_SUMMARY_RETENTION_DAYS`](crate::persistence::cooling_rollup::COOLING_DAILY_SUMMARY_RETENTION_DAYS).
//! Re-deriving forever would therefore let "the first N qualifying days"
//! silently advance as the original days aged out, drifting the very
//! reference that deltas are measured against (or regressing an
//! established baseline to `Establishing` once fewer than N days
//! remained). Writing the value down at establishment time is what makes
//! the reference outlive the rows it was computed from.
//!
//! The recent window and its comparability are still derived on every
//! read: those describe the present, not the fixed reference.

use std::collections::BTreeSet;

use chrono::{Duration, NaiveDate};

/// Minimum idle-band sample minutes a completed local day must carry
/// before it counts toward the baseline (a "qualifying day").
///
/// This is how "idle sustained for a minimum duration" is expressed at
/// daily-rollup granularity: 30 one-minute idle-band samples within the
/// day. High enough that a single transient dip into the idle band does
/// not qualify a day, low enough that a machine which is only briefly
/// idle each day still establishes a baseline.
pub const COOLING_BASELINE_QUALIFYING_IDLE_MINUTES: u32 = 30;

/// Number of qualifying days whose idle temperatures form the baseline.
///
/// #1666 allows 7-14 days. 7 is the low end of that range: it still spans
/// a full weekly usage cycle, while letting a user whose existing
/// Hardware Archive was backfilled by the rollup's first catch-up
/// establish a baseline immediately instead of waiting two weeks for data
/// they already have. Carried in [`BaselineState::Establishing`] so the
/// UI renders Core's number rather than hardcoding its own.
pub const COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS: u32 = 7;

/// Number of *recorded* completed local days - days carrying a
/// `cooling_daily_summary` row - that make up the recent window
/// summarized as "recent idle" for comparison against the baseline (see
/// [`recent_window_start`]). Mirrors
/// [`COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS`] so a weekly aggregate is
/// compared against a weekly baseline.
///
/// Recorded days rather than calendar days (#2332): the baseline itself
/// is defined over qualifying days, not consecutive ones, and a machine
/// that is switched on a few days a week accumulates its evidence at the
/// same pace in both windows. Counting the recent side in calendar days
/// left exactly those machines "not comparable" in the low/mid/high
/// bands most of the time, because seven calendar days held only two or
/// three recorded ones.
pub const COOLING_BASELINE_RECENT_WINDOW_DAYS: u32 = 7;

/// How far back, in calendar days ending at the window end (inclusive),
/// the recent window may reach while collecting its
/// [`COOLING_BASELINE_RECENT_WINDOW_DAYS`] recorded days.
///
/// The bound is what keeps "recent" honest for a machine that has not
/// run in a long while (DP-05): without it the window would gather up
/// whatever days were recorded last, however old, and present a
/// months-old reading as the present. 30 matches the default
/// `hardwareArchive.retentionDays`, the span over which the user already
/// expects the app to be looking back at one-minute data.
pub const COOLING_BASELINE_RECENT_WINDOW_MAX_CALENDAR_DAYS: u32 = 30;

/// Minimum idle sample minutes the recent window must carry before a
/// comparison against the baseline means anything. Same bar as a single
/// qualifying day: below it, consumers report "not comparable" instead of
/// a number (see [`RecentIdleSummary::is_comparable`]).
pub const COOLING_BASELINE_COMPARABLE_IDLE_MINUTES: u32 = 30;

/// One completed local day's idle-band facts, as stored by the daily
/// rollup. `idle_temperature_avg` is `None` when the day recorded no
/// idle-band temperature at all, which by the rollup's own invariant
/// implies `idle_sample_minutes == 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DailyIdleSample {
  pub date: NaiveDate,
  pub idle_temperature_avg: Option<f32>,
  pub idle_sample_minutes: u32,
}

impl DailyIdleSample {
  /// This day projected into the shape the shared establishment rule
  /// reads (see [`derive_baseline_window`]).
  fn as_baseline_sample(&self) -> DailyBaselineSample {
    DailyBaselineSample {
      date: self.date,
      value: self.idle_temperature_avg,
      sample_minutes: self.idle_sample_minutes,
    }
  }
}

/// Lifecycle of the cooling baseline, as consumed by Cooling Insight.
///
/// There is deliberately no separate "no data" variant: a machine with no
/// rollup rows at all is `Establishing { qualifying_days: 0, .. }`, which
/// the UI renders with the same "n of N days" copy. A third state would
/// add a branch everywhere without telling the user anything the progress
/// count does not already say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaselineState {
  /// Fewer than `required_days` qualifying days have been recorded, so
  /// there is no baseline value yet and no delta may be produced.
  Establishing {
    qualifying_days: u32,
    required_days: u32,
  },
  /// The baseline value in degrees Celsius, together with the collection
  /// period it was computed from.
  Established {
    idle_temperature_avg: f32,
    window_start_date: NaiveDate,
    window_end_date: NaiveDate,
    sample_minutes: u32,
  },
}

/// The established baseline as pinned into `cooling_baseline`: the value
/// together with the collection period it was computed from.
///
/// Written exactly once, the first time the derivation reaches
/// [`BaselineState::Established`], so the reference outlives the
/// `cooling_daily_summary` rows it was derived from (see the module
/// docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EstablishedBaseline {
  pub idle_temperature_avg: f32,
  pub window_start_date: NaiveDate,
  pub window_end_date: NaiveDate,
  pub sample_minutes: u32,
}

impl EstablishedBaseline {
  /// The record to pin for `state`, or `None` while still establishing -
  /// nothing is written before the baseline exists.
  pub fn from_state(state: &BaselineState) -> Option<Self> {
    match *state {
      BaselineState::Established {
        idle_temperature_avg,
        window_start_date,
        window_end_date,
        sample_minutes,
      } => Some(Self {
        idle_temperature_avg,
        window_start_date,
        window_end_date,
        sample_minutes,
      }),
      BaselineState::Establishing { .. } => None,
    }
  }

  pub fn into_state(self) -> BaselineState {
    BaselineState::Established {
      idle_temperature_avg: self.idle_temperature_avg,
      window_start_date: self.window_start_date,
      window_end_date: self.window_end_date,
      sample_minutes: self.sample_minutes,
    }
  }
}

/// Recent-window idle summary: how much idle evidence the recent past
/// actually carries, and the temperature it recorded. This is the
/// comparability guard's input. The window's days are the ones
/// [`recent_window_start`] defines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecentIdleSummary {
  pub window_start_date: NaiveDate,
  pub window_end_date: NaiveDate,
  /// Sample-minute-weighted average idle CPU temperature in the window,
  /// or `None` when the window recorded no idle-band temperature.
  pub idle_temperature_avg: Option<f32>,
  pub sample_minutes: u32,
}

impl RecentIdleSummary {
  /// Whether the window carries enough idle evidence for a comparison
  /// against the baseline to be meaningful. Consumers report "not
  /// comparable" rather than a number when this is `false`, instead of
  /// presenting a delta computed from a handful of minutes as if it were
  /// a measurement (DP-02).
  pub fn is_comparable(&self) -> bool {
    self.idle_temperature_avg.is_some()
      && self.sample_minutes >= COOLING_BASELINE_COMPARABLE_IDLE_MINUTES
  }
}

/// Everything Cooling Insight needs about the idle cooling baseline,
/// derived from a single read of the daily rollup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoolingBaseline {
  pub state: BaselineState,
  pub recent: RecentIdleSummary,
}

/// Derive the baseline and its recent-window comparison input from the
/// daily rollup.
///
/// `days` must be ordered by date ascending, as
/// `database::cooling_daily_summary::select_daily_idle_samples` returns
/// them. `window_end_date` is the most recent *completed* local day: the
/// rollup never summarizes the current day, so passing today would always
/// include a day that cannot have a row.
pub fn derive_cooling_baseline(
  days: &[DailyIdleSample],
  window_end_date: NaiveDate,
) -> CoolingBaseline {
  CoolingBaseline {
    state: derive_baseline_state(days),
    recent: summarize_recent_idle(days, window_end_date),
  }
}

/// One completed local day's contribution to *some* baseline series: the
/// day's value and how many sample minutes back it.
///
/// The projection both baselines are expressed in - the absolute idle
/// temperature here, and the ambient-normalized ΔT in
/// [`crate::persistence::cooling_delta_baseline`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DailyBaselineSample {
  pub date: NaiveDate,
  pub value: Option<f32>,
  pub sample_minutes: u32,
}

/// The outcome of applying the establishment rule to one series.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BaselineWindow {
  Establishing {
    qualifying_days: u32,
  },
  Established {
    value: f32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    sample_minutes: u32,
  },
}

/// Establish a baseline from the first `required_days` days in `days`
/// carrying at least `qualifying_minutes` of evidence, ignoring every
/// non-qualifying day in between and every day after them.
///
/// Shared by the absolute idle baseline and the ΔT baseline (#2045) so
/// the two cannot drift apart on what "established" means. They differ
/// only in which projection of a day they read and how much of it counts
/// as a qualifying amount - never in the rule itself.
///
/// `days` must be ordered by date ascending. Taking only the *first*
/// qualifying days - never the most recent ones - is what keeps the
/// derived value stable once established.
pub(crate) fn derive_baseline_window(
  days: &[DailyBaselineSample],
  qualifying_minutes: u32,
  required_days: u32,
) -> BaselineWindow {
  let window: Vec<&DailyBaselineSample> = days
    .iter()
    .filter(|day| day.sample_minutes >= qualifying_minutes && day.value.is_some())
    .take(required_days as usize)
    .collect();
  let qualifying_days = window.len() as u32;

  match (
    window.first(),
    window.last(),
    weighted_average(window.iter().copied()),
  ) {
    (Some(first), Some(last), Some((value, sample_minutes)))
      if qualifying_days == required_days =>
    {
      BaselineWindow::Established {
        value,
        start_date: first.date,
        end_date: last.date,
        sample_minutes,
      }
    }
    _ => BaselineWindow::Establishing { qualifying_days },
  }
}

/// Sample-minute-weighted average across `days`, with the total minutes
/// behind it. Weighting by minutes rather than averaging daily averages
/// keeps a day with twelve qualifying hours from counting the same as a
/// day with thirty qualifying minutes.
pub(crate) fn weighted_average<'a>(
  days: impl IntoIterator<Item = &'a DailyBaselineSample>,
) -> Option<(f32, u32)> {
  let mut weighted_sum = 0.0f64;
  let mut sample_minutes: u64 = 0;

  for day in days {
    let Some(value) = day.value else { continue };
    weighted_sum += value as f64 * day.sample_minutes as f64;
    sample_minutes += day.sample_minutes as u64;
  }

  (sample_minutes > 0).then(|| {
    (
      (weighted_sum / sample_minutes as f64) as f32,
      sample_minutes as u32,
    )
  })
}

/// Establish the idle cooling baseline from the first
/// [`COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS`] qualifying days in
/// `days` - [`derive_baseline_window`] applied to the idle temperature
/// projection.
pub fn derive_baseline_state(days: &[DailyIdleSample]) -> BaselineState {
  let required_days = COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS;
  let samples: Vec<_> = days
    .iter()
    .map(DailyIdleSample::as_baseline_sample)
    .collect();

  match derive_baseline_window(
    &samples,
    COOLING_BASELINE_QUALIFYING_IDLE_MINUTES,
    required_days,
  ) {
    BaselineWindow::Established {
      value,
      start_date,
      end_date,
      sample_minutes,
    } => BaselineState::Established {
      idle_temperature_avg: value,
      window_start_date: start_date,
      window_end_date: end_date,
      sample_minutes,
    },
    BaselineWindow::Establishing { qualifying_days } => BaselineState::Establishing {
      qualifying_days,
      required_days,
    },
  }
}

/// The first day of the recent window ending at `window_end_date`
/// (inclusive), given the dates that carry a daily rollup row.
///
/// The window is the most recent [`COOLING_BASELINE_RECENT_WINDOW_DAYS`]
/// *recorded* days, looked for no further back than
/// [`COOLING_BASELINE_RECENT_WINDOW_MAX_CALENDAR_DAYS`] calendar days
/// ending at `window_end_date`. With fewer recorded days than that in the
/// span, the window starts at the earliest of them; with none, it falls
/// back to the trailing [`COOLING_BASELINE_RECENT_WINDOW_DAYS`] calendar
/// days, which is then an empty window and reports "not comparable"
/// exactly as a calendar-anchored window always did.
///
/// This is the one place the recent window is defined. Every comparison
/// Cooling Insight draws - the idle baseline card, the load-band
/// comparison, the baseline delta and the ambient-adjusted and
/// co-variate readings beside them - derives its recent window through
/// this function from the hardware rollup's recorded days, so they
/// cannot drift apart on what "recent" means. The ΔT and co-variate
/// readings deliberately do *not* count their own rows: a day the machine
/// ran without its ambient sensor pairing a minute is still a recorded
/// day, and the recent window must be the same stretch of days in every
/// panel of one response.
///
/// The calendar bound, not the recorded-day count, is what guards
/// against staleness (DP-05): an app that has not run for more than the
/// bound reports an empty recent window rather than presenting its last
/// few months-old days as if they were the present. Within the bound, a
/// recorded day is recent by definition - the user has been using the
/// machine at that pace, and the window should follow the machine's own
/// rhythm rather than demand that it be on every day.
///
/// `window_end_date` itself counts as a recorded day when it has a row.
/// `recorded_dates` need not be sorted or unique.
pub fn recent_window_start(
  recorded_dates: impl IntoIterator<Item = NaiveDate>,
  window_end_date: NaiveDate,
) -> NaiveDate {
  let span_start = window_end_date
    - Duration::days(COOLING_BASELINE_RECENT_WINDOW_MAX_CALENDAR_DAYS as i64 - 1);
  let recorded_in_span: BTreeSet<NaiveDate> = recorded_dates
    .into_iter()
    .filter(|date| *date >= span_start && *date <= window_end_date)
    .collect();

  // Skip every recorded day but the last `COOLING_BASELINE_RECENT_WINDOW_DAYS`;
  // the first one left is the window's start. With fewer than that many
  // nothing is skipped, and with none there is nothing to start from.
  let older_than_window = recorded_in_span
    .len()
    .saturating_sub(COOLING_BASELINE_RECENT_WINDOW_DAYS as usize);
  recorded_in_span
    .iter()
    .nth(older_than_window)
    .copied()
    .unwrap_or_else(|| {
      window_end_date - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1)
    })
}

/// Summarize the recent window ending at `window_end_date` (inclusive) -
/// the most recent [`COOLING_BASELINE_RECENT_WINDOW_DAYS`] recorded days
/// among `days`, as [`recent_window_start`] defines them.
///
/// Unlike the baseline window this counts *every* idle minute in range,
/// not only minutes from qualifying days: the guard is on how much idle
/// evidence the window carries in total
/// ([`COOLING_BASELINE_COMPARABLE_IDLE_MINUTES`]), so discarding a short
/// idle stretch before summing would hide real signal.
pub fn summarize_recent_idle(
  days: &[DailyIdleSample],
  window_end_date: NaiveDate,
) -> RecentIdleSummary {
  let window_start_date =
    recent_window_start(days.iter().map(|day| day.date), window_end_date);
  let in_window = days
    .iter()
    .filter(|day| day.date >= window_start_date && day.date <= window_end_date);

  let (idle_temperature_avg, sample_minutes) = match weighted_idle_temperature(in_window)
  {
    Some((average, minutes)) => (Some(average), minutes),
    None => (None, 0),
  };

  RecentIdleSummary {
    window_start_date,
    window_end_date,
    idle_temperature_avg,
    sample_minutes,
  }
}

/// [`weighted_average`] over the idle temperature projection. `None` when
/// no day carries any idle-band minutes.
fn weighted_idle_temperature<'a>(
  days: impl IntoIterator<Item = &'a DailyIdleSample>,
) -> Option<(f32, u32)> {
  let samples: Vec<_> = days
    .into_iter()
    .map(DailyIdleSample::as_baseline_sample)
    .collect();
  weighted_average(samples.iter())
}

/// Resolve the baseline lifecycle state for any Cooling Insight query:
/// the pinned row if one exists, otherwise derive it from `days` and pin
/// it the moment it reaches [`BaselineState::Established`].
///
/// Every loader that needs the baseline state (the baseline card itself,
/// the band comparison, the baseline delta, ...) must go through this
/// function rather than calling [`derive_baseline_state`] directly -
/// otherwise it silently ignores the pinned row and drifts as the
/// `cooling_daily_summary` rows the original establishment was derived
/// from age out (see the module docs). Keeping the write-back in this one
/// place is what keeps that guarantee from depending on every call site
/// remembering it.
/// Test-only since #2134: [`resolve_baseline_state`] is routed through the
/// dispatch boundary instead of an explicit pool.
#[cfg(test)]
pub(crate) async fn resolve_baseline_state_from_pool(
  pool: &sqlx::SqlitePool,
  days: &[DailyIdleSample],
) -> Result<BaselineState, sqlx::Error> {
  use crate::infrastructure::database;

  match database::cooling_baseline::select_established_baseline_from_pool(pool).await? {
    Some(pinned) => Ok(pinned.into_state()),
    None => {
      let derived = derive_baseline_state(days);
      if let Some(baseline) = EstablishedBaseline::from_state(&derived) {
        // Pinning is write-once bookkeeping (`INSERT OR IGNORE`), not
        // part of the answer: a transient write failure (e.g.
        // SQLITE_BUSY while the rollup worker holds the database) must
        // not turn a valid derivation into a read error. The pin is
        // simply retried on the next resolution.
        if let Err(e) = database::cooling_baseline::insert_established_baseline_from_pool(
          pool,
          &baseline,
          chrono::Utc::now(),
        )
        .await
        {
          crate::log_error!(
            "Failed to pin the established cooling baseline; retrying on the next resolution",
            "persistence::cooling_baseline::resolve_baseline_state_from_pool",
            Some(e.to_string())
          );
        }
      }
      Ok(derived)
    }
  }
}

/// [`resolve_baseline_state_from_pool`], routed through the dispatch
/// boundary (#2134) instead of an explicit pool. Every production caller
/// (this module's own loader, the band comparison, the baseline delta and
/// the load-temperature explorer) uses this, not the `_from_pool` variant -
/// see the module doc on why every one of them must go through a resolver
/// rather than [`derive_baseline_state`] directly.
pub(crate) async fn resolve_baseline_state(
  days: &[DailyIdleSample],
) -> Result<BaselineState, crate::infrastructure::database::dispatch::DispatchError> {
  use crate::infrastructure::database::dispatch;

  match dispatch::cooling_baseline::select_established_baseline().await? {
    Some(pinned) => Ok(pinned.into_state()),
    None => {
      let derived = derive_baseline_state(days);
      if let Some(baseline) = EstablishedBaseline::from_state(&derived) {
        // #2271: a read command reaches this pin outside the producers a
        // conversion pauses/drains (`native_conversion::pause_and_drain_producers`).
        // Writing it while the App's conversion lifecycle owner reports
        // `Converting` could land in SQLite after the conversion's
        // candidate snapshot was already taken and before the reconciled
        // selection commits, so it would never reach the native database -
        // it would then be silently re-pinned later, letting
        // `established_at` shift. The permit is held across the write
        // itself (not just checked beforehand): a point-in-time check
        // alone would not stop the conversion driver from closing the gate
        // and taking its snapshot while this insert is still in flight -
        // see `dispatch::acquire_write_permit`'s own doc. Skipping the
        // write when no permit is granted is safe: the derived state above
        // is still returned honestly, and the next resolution (once the
        // window closes) retries the pin exactly like a transient write
        // failure already would.
        if let Some(_permit) = dispatch::acquire_write_permit().await {
          // Same write-once, retry-on-failure bookkeeping as the
          // `_from_pool` resolver above.
          if let Err(e) =
            dispatch::cooling_baseline::insert_established_baseline(&baseline).await
          {
            crate::log_error!(
              "Failed to pin the established cooling baseline; retrying on the next resolution",
              "persistence::cooling_baseline::resolve_baseline_state",
              Some(e.to_string())
            );
          }
        }
      }
      Ok(derived)
    }
  }
}

/// Load the cooling baseline, pinning it the first time it establishes.
///
/// Reads the pinned row first: once established, the baseline is a fixed
/// reference, and re-deriving it would let it drift as the rollup rows it
/// came from age out (see the module docs). Only while nothing is pinned
/// does this derive from the rollup, and that derivation is written back
/// as soon as it reaches [`BaselineState::Established`].
///
/// The recent window is always derived - it describes the present, not
/// the fixed reference.
/// Test-only since #2134: every production caller uses [`load_cooling_baseline`],
/// which is routed through the dispatch boundary instead of an explicit pool.
#[cfg(test)]
pub(crate) async fn load_cooling_baseline_from_pool(
  pool: &sqlx::SqlitePool,
  today: NaiveDate,
) -> Result<CoolingBaseline, sqlx::Error> {
  use crate::infrastructure::database;

  let days =
    database::cooling_daily_summary::select_daily_idle_samples_from_pool(pool).await?;
  // The rollup only ever summarizes completed local days, so the newest
  // day that can carry a row is yesterday. Ending the recent window
  // there (not at the newest row present) is what lets `recent_window_start`'s
  // calendar bound make a long gap in usage report "not comparable"
  // instead of presenting a stale reading as recent.
  let recent = summarize_recent_idle(&days, today - Duration::days(1));
  let state = resolve_baseline_state_from_pool(pool, &days).await?;

  Ok(CoolingBaseline { state, recent })
}

/// [`load_cooling_baseline_from_pool`], routed through the dispatch
/// boundary (#2134) instead of Core's process-wide SQLite pool.
pub async fn load_cooling_baseline()
-> Result<CoolingBaseline, crate::infrastructure::database::dispatch::DispatchError> {
  use crate::infrastructure::database::dispatch;

  let today = chrono::Local::now().date_naive();
  let days = dispatch::cooling_daily_summary::select_daily_idle_samples().await?;
  // The rollup only ever summarizes completed local days, so the newest day
  // that can carry a row is yesterday - see `load_cooling_baseline_from_pool`
  // for why the recent window ends there rather than at the newest row.
  let recent = summarize_recent_idle(&days, today - Duration::days(1));
  let state = resolve_baseline_state(&days).await?;

  Ok(CoolingBaseline { state, recent })
}

/// Resolve — and, on first establishment, pin — the baseline in the
/// background. The rollup worker calls this after every catch-up pass,
/// so establishment never depends on Cooling Insight being opened
/// before retention cleanup erases the establishment-window rows. The
/// `scheduledDataDeletion` cleanup itself only runs after a successful
/// catch-up, which orders it after this call too. Failures are logged
/// and retried on the next pass.
pub(crate) async fn ensure_baseline_pinned() {
  if let Err(e) = load_cooling_baseline().await {
    crate::log_error!(
      "Failed to resolve the cooling baseline after a rollup catch-up",
      "persistence::cooling_baseline::ensure_baseline_pinned",
      Some(e.to_string())
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::test_schema::{
    COOLING_BASELINE_DDL, COOLING_DAILY_SUMMARY_DDL, create_tables,
  };
  use sqlx::SqlitePool;

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  /// A day carrying `minutes` of idle-band samples at `temperature`.
  fn day(date: NaiveDate, temperature: f32, minutes: u32) -> DailyIdleSample {
    DailyIdleSample {
      date,
      idle_temperature_avg: Some(temperature),
      idle_sample_minutes: minutes,
    }
  }

  /// A day the machine was recorded on but never spent in the idle band.
  fn day_without_idle(date: NaiveDate) -> DailyIdleSample {
    DailyIdleSample {
      date,
      idle_temperature_avg: None,
      idle_sample_minutes: 0,
    }
  }

  /// `count` consecutive qualifying days starting at `start`.
  fn qualifying_days(
    start: NaiveDate,
    count: u32,
    temperature: f32,
  ) -> Vec<DailyIdleSample> {
    (0..count)
      .map(|offset| {
        day(
          start + Duration::days(offset as i64),
          temperature,
          COOLING_BASELINE_QUALIFYING_IDLE_MINUTES,
        )
      })
      .collect()
  }

  // ── qualifying day boundary ──

  #[test]
  fn a_day_at_exactly_the_minimum_idle_minutes_qualifies() {
    let days = qualifying_days(
      date(2026, 8, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      40.0,
    );

    assert!(matches!(
      derive_baseline_state(&days),
      BaselineState::Established { .. }
    ));
  }

  #[test]
  fn a_day_one_minute_below_the_minimum_does_not_qualify() {
    let short = COOLING_BASELINE_QUALIFYING_IDLE_MINUTES - 1;
    let days: Vec<_> = (0..COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS)
      .map(|offset| {
        day(
          date(2026, 8, 1) + Duration::days(offset as i64),
          40.0,
          short,
        )
      })
      .collect();

    assert_eq!(
      derive_baseline_state(&days),
      BaselineState::Establishing {
        qualifying_days: 0,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn a_day_without_an_idle_temperature_never_qualifies() {
    // Defensive: the rollup's invariant already ties a missing average to
    // zero sample minutes, but a long idle stretch with no temperature
    // reading must not become a baseline day either way.
    let days = vec![
      DailyIdleSample {
        date: date(2026, 8, 1),
        idle_temperature_avg: None,
        idle_sample_minutes: 600,
      };
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as usize
    ];

    assert_eq!(
      derive_baseline_state(&days),
      BaselineState::Establishing {
        qualifying_days: 0,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  // ── establishment progress ──

  #[test]
  fn an_empty_rollup_is_establishing_with_no_progress() {
    assert_eq!(
      derive_baseline_state(&[]),
      BaselineState::Establishing {
        qualifying_days: 0,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn one_qualifying_day_short_is_still_establishing() {
    let days = qualifying_days(
      date(2026, 8, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1,
      40.0,
    );

    assert_eq!(
      derive_baseline_state(&days),
      BaselineState::Establishing {
        qualifying_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn establishes_on_exactly_the_required_number_of_qualifying_days() {
    let days = qualifying_days(
      date(2026, 8, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      42.0,
    );

    assert_eq!(
      derive_baseline_state(&days),
      BaselineState::Established {
        idle_temperature_avg: 42.0,
        window_start_date: date(2026, 8, 1),
        window_end_date: date(2026, 8, 1)
          + Duration::days(COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 - 1),
        sample_minutes: COOLING_BASELINE_QUALIFYING_IDLE_MINUTES
          * COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn non_qualifying_days_between_qualifying_ones_do_not_count_or_break_the_window() {
    let mut days = Vec::new();
    let mut cursor = date(2026, 8, 1);
    for _ in 0..COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS {
      days.push(day(cursor, 40.0, COOLING_BASELINE_QUALIFYING_IDLE_MINUTES));
      cursor += Duration::days(1);
      // A day the machine ran hard all day: recorded, but no idle
      // evidence, so it neither counts nor resets the progress.
      days.push(day_without_idle(cursor));
      cursor += Duration::days(1);
    }

    let last_qualifying = date(2026, 8, 1)
      + Duration::days(2 * (COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 - 1));
    assert_eq!(
      derive_baseline_state(&days),
      BaselineState::Established {
        idle_temperature_avg: 40.0,
        window_start_date: date(2026, 8, 1),
        window_end_date: last_qualifying,
        sample_minutes: COOLING_BASELINE_QUALIFYING_IDLE_MINUTES
          * COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  // ── weighting ──

  #[test]
  fn the_baseline_value_is_weighted_by_idle_sample_minutes() {
    let mut days = qualifying_days(
      date(2026, 8, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1,
      40.0,
    );
    // One long idle day at a different temperature must pull the average
    // further than an equally-weighted mean would.
    days.push(day(
      date(2026, 8, 1)
        + Duration::days(COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 - 1),
      50.0,
      COOLING_BASELINE_QUALIFYING_IDLE_MINUTES * 6,
    ));

    let short_days = (COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1) as f32;
    let expected = (40.0 * short_days + 50.0 * 6.0) / (short_days + 6.0);

    match derive_baseline_state(&days) {
      BaselineState::Established {
        idle_temperature_avg,
        sample_minutes,
        ..
      } => {
        assert!(
          (idle_temperature_avg - expected).abs() < 0.001,
          "expected {expected}, got {idle_temperature_avg}"
        );
        assert_eq!(
          sample_minutes,
          COOLING_BASELINE_QUALIFYING_IDLE_MINUTES
            * (COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1 + 6)
        );
      }
      other => panic!("expected an established baseline, got {other:?}"),
    }
  }

  // ── stability once established ──

  #[test]
  fn later_qualifying_days_never_move_an_established_baseline() {
    let established = qualifying_days(
      date(2026, 8, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      42.0,
    );
    let state_at_establishment = derive_baseline_state(&established);

    let mut with_later_days = established;
    with_later_days.extend(qualifying_days(
      date(2026, 9, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      // A much hotter month must not rewrite the reference the deltas are
      // measured against - that is the whole point of a baseline.
      70.0,
    ));

    assert_eq!(
      derive_baseline_state(&with_later_days),
      state_at_establishment
    );
  }

  // ── establish-then-persist (DB-backed) ──

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

  /// Record `COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS` qualifying days
  /// starting at `start`, enough to establish the baseline.
  async fn insert_establishing_days(
    pool: &SqlitePool,
    start: NaiveDate,
    temperature: f32,
  ) {
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
  async fn the_baseline_survives_the_rollup_rows_it_was_derived_from_being_deleted() {
    // The regression this whole design exists for: the rollup is cleaned
    // up after COOLING_DAILY_SUMMARY_RETENTION_DAYS, so the days the
    // baseline was established from eventually disappear. Once pinned,
    // the reference must not move when that happens - neither drifting to
    // a later set of qualifying days nor regressing to Establishing.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_tables(&pool).await;
    let start = date(2026, 8, 1);
    insert_establishing_days(&pool, start, 42.0).await;

    let established = load_cooling_baseline_from_pool(&pool, date(2026, 8, 20))
      .await
      .unwrap();
    assert!(
      matches!(
        established.state,
        BaselineState::Established {
          idle_temperature_avg: 42.0,
          ..
        }
      ),
      "expected an established baseline, got {:?}",
      established.state
    );

    // Age out every row the baseline was derived from, and record a
    // hotter stretch of days that would establish a different value.
    sqlx::query("DELETE FROM cooling_daily_summary")
      .execute(&pool)
      .await
      .unwrap();
    insert_establishing_days(&pool, date(2027, 6, 1), 70.0).await;

    let after_cleanup = load_cooling_baseline_from_pool(&pool, date(2027, 6, 20))
      .await
      .unwrap();

    assert_eq!(
      after_cleanup.state, established.state,
      "the pinned baseline must not drift when its source rows are deleted"
    );
  }

  #[tokio::test]
  async fn clearing_the_rollup_and_the_pinned_row_returns_the_baseline_to_establishing() {
    // The Insights data reset clears both tables. Only then does the
    // baseline go back to establishing and re-derive from whatever is
    // recorded afterwards.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_tables(&pool).await;
    insert_establishing_days(&pool, date(2026, 8, 1), 42.0).await;
    assert!(matches!(
      load_cooling_baseline_from_pool(&pool, date(2026, 8, 20))
        .await
        .unwrap()
        .state,
      BaselineState::Established { .. }
    ));

    for table in ["cooling_daily_summary", "cooling_baseline"] {
      sqlx::query(&format!("DELETE FROM {table}"))
        .execute(&pool)
        .await
        .unwrap();
    }

    assert_eq!(
      load_cooling_baseline_from_pool(&pool, date(2026, 8, 20))
        .await
        .unwrap()
        .state,
      BaselineState::Establishing {
        qualifying_days: 0,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );

    // Days recorded after the reset establish a fresh baseline at the new
    // temperature rather than restoring the old one.
    insert_establishing_days(&pool, date(2026, 10, 1), 55.0).await;
    assert_eq!(
      load_cooling_baseline_from_pool(&pool, date(2026, 10, 20))
        .await
        .unwrap()
        .state,
      BaselineState::Established {
        idle_temperature_avg: 55.0,
        window_start_date: date(2026, 10, 1),
        window_end_date: date(2026, 10, 1)
          + Duration::days(COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64 - 1),
        sample_minutes: COOLING_BASELINE_QUALIFYING_IDLE_MINUTES
          * COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  /// #2271: `resolve_baseline_state` (the dispatch-routed resolver every
  /// production caller uses) holds a `dispatch::acquire_write_permit()`
  /// across its lazy pin write, so a read reached while the App's
  /// conversion lifecycle owner reports `Converting` does not write a pin
  /// that could land in SQLite after the conversion's candidate snapshot
  /// was already taken. This test cannot drive `resolve_baseline_state`
  /// itself (it is routed through Core's process-wide pool via the
  /// dispatch boundary, which is `OnceLock`-configured once per process -
  /// see `infrastructure::database::db::init`'s own doc), so it exercises
  /// the same select/derive/permit/insert shape directly against an
  /// isolated pool, using the real, shared `dispatch` write gate the
  /// production resolver reads.
  ///
  /// `dispatch::test_support` (and the gate it guards) only exists with
  /// `duckdb-archive` on - the default, and the one every other build
  /// enables (see `docs/architecture/backend.md`) - since there is no
  /// conversion lifecycle to race without it.
  #[cfg(feature = "duckdb-archive")]
  #[tokio::test]
  async fn a_read_during_conversion_does_not_pin_and_pins_normally_once_the_window_closes()
   {
    use crate::infrastructure::database;
    use crate::infrastructure::database::dispatch;

    let _guard = dispatch::test_support::lock_write_gate().await;
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_tables(&pool).await;
    insert_establishing_days(&pool, date(2026, 8, 1), 42.0).await;
    let days =
      crate::infrastructure::database::cooling_daily_summary::select_daily_idle_samples_from_pool(
        &pool,
      )
      .await
      .unwrap();
    let derived = derive_baseline_state(&days);
    let established =
      EstablishedBaseline::from_state(&derived).expect("this window must establish");

    // The App marks the lifecycle owner `Converting`: the gate the resolver
    // must hold a permit from before writing closes.
    dispatch::set_writable(false).await;
    if let Some(_permit) = dispatch::acquire_write_permit().await {
      database::cooling_baseline::insert_established_baseline_from_pool(
        &pool,
        &established,
        chrono::Utc::now(),
      )
      .await
      .unwrap();
    }
    assert_eq!(
      database::cooling_baseline::select_established_baseline_from_pool(&pool)
        .await
        .unwrap(),
      None,
      "the pin must not be written while the gate reports Converting"
    );

    // The conversion resolves and the owner leaves `Converting`: the gate
    // reopens, and the very next resolution's write-back succeeds exactly
    // as the retry-on-failure bookkeeping already promises.
    dispatch::set_writable(true).await;
    if let Some(_permit) = dispatch::acquire_write_permit().await {
      database::cooling_baseline::insert_established_baseline_from_pool(
        &pool,
        &established,
        chrono::Utc::now(),
      )
      .await
      .unwrap();
    }
    assert_eq!(
      database::cooling_baseline::select_established_baseline_from_pool(&pool)
        .await
        .unwrap(),
      Some(established),
      "the pin establishes normally once the conversion window closes"
    );
  }

  #[tokio::test]
  async fn a_baseline_still_establishing_is_not_pinned_yet() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_tables(&pool).await;
    for offset in 0..(COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1) {
      insert_idle_day(
        &pool,
        date(2026, 8, 1) + Duration::days(offset as i64),
        42.0,
        COOLING_BASELINE_QUALIFYING_IDLE_MINUTES,
      )
      .await;
    }

    let baseline = load_cooling_baseline_from_pool(&pool, date(2026, 8, 20))
      .await
      .unwrap();

    assert_eq!(
      baseline.state,
      BaselineState::Establishing {
        qualifying_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS - 1,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
    let pinned: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM cooling_baseline")
      .fetch_one(&pool)
      .await
      .unwrap();
    assert_eq!(pinned, 0, "nothing may be pinned before establishment");
  }

  #[tokio::test]
  async fn the_recent_window_still_tracks_the_present_after_the_baseline_is_pinned() {
    // Pinning fixes the reference, not the comparison: recent idle must
    // keep following current data.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_tables(&pool).await;
    insert_establishing_days(&pool, date(2026, 8, 1), 42.0).await;
    load_cooling_baseline_from_pool(&pool, date(2026, 8, 20))
      .await
      .unwrap();

    insert_idle_day(&pool, date(2026, 9, 10), 61.0, 120).await;
    let baseline = load_cooling_baseline_from_pool(&pool, date(2026, 9, 11))
      .await
      .unwrap();

    assert_eq!(baseline.recent.idle_temperature_avg, Some(61.0));
    assert_eq!(baseline.recent.sample_minutes, 120);
    assert!(baseline.recent.is_comparable());
  }

  // ── recent window start (#2332) ──

  /// `end` minus each of `days_back`, as the recorded days of a machine
  /// that was switched on only on those days.
  fn recorded(end: NaiveDate, days_back: &[i64]) -> Vec<NaiveDate> {
    days_back
      .iter()
      .map(|back| end - Duration::days(*back))
      .collect()
  }

  #[test]
  fn the_recent_window_starts_at_the_seventh_most_recent_recorded_day() {
    // Recorded on 7 of the last 20 days: the window reaches back to the
    // seventh one, however many calendar days that spans.
    let end = date(2026, 8, 20);
    let dates = recorded(end, &[0, 3, 5, 8, 12, 16, 19]);

    assert_eq!(recent_window_start(dates, end), end - Duration::days(19));
  }

  #[test]
  fn more_recorded_days_than_the_window_holds_leave_the_older_ones_out() {
    let end = date(2026, 8, 20);
    let dates = recorded(end, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    assert_eq!(
      recent_window_start(dates, end),
      end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1)
    );
  }

  #[test]
  fn fewer_recorded_days_than_the_window_holds_start_it_at_the_earliest_of_them() {
    let end = date(2026, 8, 20);
    let dates = recorded(end, &[2, 11, 25]);

    assert_eq!(recent_window_start(dates, end), end - Duration::days(25));
  }

  #[test]
  fn no_recorded_day_within_the_calendar_bound_falls_back_to_the_trailing_calendar_days()
  {
    // An app that has not run for more than the bound: older rows exist,
    // but the window must not reach back to them (DP-05).
    let end = date(2026, 8, 20);
    let bound = COOLING_BASELINE_RECENT_WINDOW_MAX_CALENDAR_DAYS as i64;
    let dates = recorded(end, &[bound, bound + 1, bound + 40]);

    assert_eq!(
      recent_window_start(dates, end),
      end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1)
    );
  }

  #[test]
  fn a_recorded_day_beyond_the_calendar_bound_never_widens_the_window() {
    // Two recorded days inside the bound and one just outside it: the
    // window starts at the earlier of the two inside, not at the third.
    let end = date(2026, 8, 20);
    let bound = COOLING_BASELINE_RECENT_WINDOW_MAX_CALENDAR_DAYS as i64;
    let dates = recorded(end, &[0, bound - 1, bound]);

    assert_eq!(
      recent_window_start(dates, end),
      end - Duration::days(bound - 1)
    );
  }

  #[test]
  fn the_window_end_date_itself_counts_as_a_recorded_day() {
    let end = date(2026, 8, 20);
    let dates = recorded(end, &[0, 1, 2, 3, 4, 5, 6, 7]);

    // Eight recorded days ending at `end`: the window is the last seven,
    // so `end` fills one of the seven slots and the eighth day is out.
    assert_eq!(recent_window_start(dates, end), end - Duration::days(6));
  }

  #[test]
  fn days_after_the_window_end_are_not_recorded_days_of_the_window() {
    // The rollup cannot produce a day after yesterday, but a clock change
    // could; it must neither count toward the seven nor move the start.
    let end = date(2026, 8, 20);
    let dates = recorded(end, &[-1, 0, 10]);

    assert_eq!(recent_window_start(dates, end), end - Duration::days(10));
  }

  #[test]
  fn recorded_dates_may_repeat_without_consuming_window_slots() {
    // Callers projecting row-per-source tables hand in one date per row.
    let end = date(2026, 8, 20);
    let mut dates = recorded(end, &[0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3]);
    dates.extend(recorded(end, &[9, 9, 9]));

    assert_eq!(recent_window_start(dates, end), end - Duration::days(9));
  }

  // ── recent idle window ──

  #[test]
  fn the_recent_window_spans_the_configured_number_of_days_ending_at_the_last_completed_day()
   {
    let summary = summarize_recent_idle(&[], date(2026, 8, 20));

    assert_eq!(summary.window_end_date, date(2026, 8, 20));
    assert_eq!(
      summary.window_start_date,
      date(2026, 8, 20) - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1)
    );
  }

  #[test]
  fn the_recent_window_ignores_days_outside_it() {
    let end = date(2026, 8, 20);
    let start = end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    // Recorded every day, so the window is exactly the configured number
    // of calendar days...
    let mut days: Vec<_> = (0..COOLING_BASELINE_RECENT_WINDOW_DAYS as i64)
      .map(|offset| day(start + Duration::days(offset), 40.0, 60))
      .collect();
    // ...and one more recorded day is one day too old.
    days.insert(0, day(start - Duration::days(1), 90.0, 600));
    // The rollup cannot produce this day yet, but a clock change could.
    days.push(day(end + Duration::days(1), 90.0, 600));

    let summary = summarize_recent_idle(&days, end);

    assert_eq!(summary.window_start_date, start);
    assert_eq!(
      summary.sample_minutes,
      60 * COOLING_BASELINE_RECENT_WINDOW_DAYS
    );
    assert_eq!(summary.idle_temperature_avg, Some(40.0));
  }

  #[test]
  fn a_machine_switched_on_a_few_days_a_week_still_fills_its_recent_window() {
    // The #2332 case: recorded on 2 of the last 7 calendar days and 5
    // more over the preceding three weeks, each with too few idle
    // minutes to be comparable on its own. The window reaches back to
    // the seventh recorded day and the minutes add up.
    let end = date(2026, 8, 20);
    let days: Vec<_> = recorded(end, &[0, 3, 8, 11, 15, 18, 21])
      .into_iter()
      .map(|date| day(date, 40.0, 10))
      .collect();

    let summary = summarize_recent_idle(&days, end);

    assert_eq!(summary.window_start_date, end - Duration::days(21));
    assert_eq!(summary.sample_minutes, 70);
    assert_eq!(summary.idle_temperature_avg, Some(40.0));
    assert!(summary.is_comparable());
  }

  #[test]
  fn the_recent_window_is_weighted_by_idle_sample_minutes() {
    let end = date(2026, 8, 20);
    let days = vec![day(end - Duration::days(1), 40.0, 30), day(end, 50.0, 90)];

    let summary = summarize_recent_idle(&days, end);

    assert_eq!(summary.sample_minutes, 120);
    assert_eq!(
      summary.idle_temperature_avg,
      Some((40.0 * 30.0 + 50.0 * 90.0) / 120.0)
    );
  }

  #[test]
  fn the_recent_window_counts_idle_minutes_from_non_qualifying_days_too() {
    // Two short idle stretches are still real evidence; the guard is on
    // the window total, not on per-day qualification.
    let end = date(2026, 8, 20);
    let days = vec![day(end - Duration::days(1), 40.0, 20), day(end, 40.0, 20)];

    let summary = summarize_recent_idle(&days, end);

    assert_eq!(summary.sample_minutes, 40);
    assert_eq!(summary.idle_temperature_avg, Some(40.0));
  }

  #[test]
  fn a_recent_window_without_idle_samples_reports_no_temperature() {
    let end = date(2026, 8, 20);
    let days = vec![
      day_without_idle(end - Duration::days(1)),
      day_without_idle(end),
    ];

    let summary = summarize_recent_idle(&days, end);

    assert_eq!(summary.idle_temperature_avg, None);
    assert_eq!(summary.sample_minutes, 0);
  }

  // ── comparability guard ──

  #[test]
  fn a_window_at_exactly_the_comparable_minutes_is_comparable() {
    let end = date(2026, 8, 20);
    let days = vec![day(end, 40.0, COOLING_BASELINE_COMPARABLE_IDLE_MINUTES)];

    assert!(summarize_recent_idle(&days, end).is_comparable());
  }

  #[test]
  fn a_window_one_minute_short_is_not_comparable() {
    let end = date(2026, 8, 20);
    let days = vec![day(end, 40.0, COOLING_BASELINE_COMPARABLE_IDLE_MINUTES - 1)];

    assert!(!summarize_recent_idle(&days, end).is_comparable());
  }

  #[test]
  fn an_empty_window_is_not_comparable() {
    assert!(!summarize_recent_idle(&[], date(2026, 8, 20)).is_comparable());
  }

  #[test]
  fn a_long_idle_history_that_stopped_before_the_window_is_not_comparable() {
    // An app that has not run for months must not present a months-old
    // reading as if it were recent.
    let days = qualifying_days(date(2026, 1, 1), 30, 40.0);

    let summary = summarize_recent_idle(&days, date(2026, 8, 20));

    assert_eq!(summary.sample_minutes, 0);
    assert!(!summary.is_comparable());
  }

  // ── derive_cooling_baseline ──

  #[test]
  fn derive_cooling_baseline_reports_state_and_recent_window_together() {
    let mut days = qualifying_days(
      date(2026, 8, 1),
      COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      42.0,
    );
    // More than the recent window's calendar bound after the baseline
    // days, so none of them is a recorded day of the recent window.
    let end = date(2026, 9, 20);
    days.push(day(end, 45.0, 120));

    let baseline = derive_cooling_baseline(&days, end);

    assert!(matches!(
      baseline.state,
      BaselineState::Established {
        idle_temperature_avg: 42.0,
        ..
      }
    ));
    assert_eq!(baseline.recent.idle_temperature_avg, Some(45.0));
    assert_eq!(baseline.recent.sample_minutes, 120);
    assert!(baseline.recent.is_comparable());
  }
}
