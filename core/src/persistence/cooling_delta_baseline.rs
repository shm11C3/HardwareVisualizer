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
//! **It is established from one ambient source and records which**
//! (#2062). The rollup it reads is row-per-source, and the rule runs per
//! source: a sensor change never lets two placements' days count toward
//! one window, because a ΔT measured against a sensor on the desk and one
//! measured against a sensor across the room are two different quantities.
//! Several sensors in one room were observed more than 2 K apart, wider
//! than the rise Cooling Insight calls sustained. The pinned row names its
//! source so a recent window is only ever measured against a reference of
//! the same source.
//!
//! **Every source gets its own reference**
//! ([`resolve_delta_baseline_for_window`], #2331). The pinned row is the
//! reference of the source that established first, and a recent window
//! read from that source compares against it. A recent window read from
//! any *other* source compares against that source's own baseline,
//! derived on read from its own first qualifying days by the same rule -
//! or reports that source's own establishing progress while it has too
//! few. The earlier design refused every other source outright, and on a
//! machine whose user rebinds the app to a different meter that is a
//! permanent refusal: the pin is write-once, so no amount of data the new
//! sensor collects could ever make the reading comparable again.
//!
//! Like the absolute baseline the first reference is **established by
//! derivation, then pinned** into a single-row table, for the same
//! reason: the `cooling_thermal_delta_daily_summary` rows it was derived
//! from are eventually cleaned up, and re-deriving forever would let "the
//! first N qualifying days" silently advance as they aged out. A
//! derived-on-read reference is kept just as stable by exempting every
//! source's first qualifying days from that cleanup instead
//! ([`retention_exempt_source_windows`]) - the rows before them are
//! non-qualifying by definition, so deleting those cannot change which
//! days are first, and the rows after them are never consulted.
//!
//! Why not pin one row per source? Because there is no native (DuckDB)
//! schema migration path in the 1.11.x line: `NATIVE_SCHEMA_VERSION` is
//! fixed at finalize time and the runtime refuses to open a file recorded
//! under any other version, so a second table or a widened one cannot
//! ship as a bugfix. Derive-on-read plus the retention exemption gives the
//! same answer without touching the schema; a per-source pinned table is
//! the shape to revisit once a migration path exists.
//!
//! It gets its *own* table rather than columns on `cooling_baseline`.
//! Pinning there is write-once (`INSERT OR IGNORE` against a
//! `CHECK (id = 1)` row), and that is precisely what makes an established
//! baseline undriftable. Two baselines that establish at different times
//! cannot share one write-once row: the second would have to arrive as an
//! `UPDATE ... WHERE <column> IS NULL`, a weaker rule that has to be got
//! right rather than being structurally impossible to get wrong. Separate
//! rows keep each one's insert-once invariant exactly as strong as it was.

use std::collections::BTreeMap;

use chrono::NaiveDate;

use crate::persistence::cooling_band_comparison::dominant_delta_source;
use crate::persistence::cooling_baseline::{
  BaselineWindow, COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS, DailyBaselineSample,
  derive_baseline_window, first_qualifying_days,
};
use crate::persistence::cooling_rollup::CpuLoadBand;
use crate::persistence::cooling_thermal_delta_rollup::ThermalDeltaDailySummary;

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
#[derive(Debug, Clone, PartialEq)]
pub enum DeltaBaselineState {
  Establishing {
    /// Qualifying days of whichever single source is furthest along -
    /// never a sum across sources, which is the mixing this module
    /// forbids.
    qualifying_days: u32,
    required_days: u32,
  },
  Established {
    /// The ambient Sensor Source Label every one of the window's paired
    /// minutes was measured against. A recent window is only ever
    /// compared against a baseline of its own source - see
    /// [`resolve_delta_baseline_for_window`] for how a window from
    /// another source gets one.
    source: String,
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
    match self {
      Self::Established {
        window_start_date,
        window_end_date,
        ..
      } => Some((*window_start_date, *window_end_date)),
      Self::Establishing { .. } => None,
    }
  }

  /// The ambient source this baseline was established from, or `None`
  /// while still establishing.
  pub fn source(&self) -> Option<&str> {
    match self {
      Self::Established { source, .. } => Some(source),
      Self::Establishing { .. } => None,
    }
  }
}

/// The established ΔT baseline as pinned into `cooling_delta_baseline`.
#[derive(Debug, Clone, PartialEq)]
pub struct EstablishedDeltaBaseline {
  pub source: String,
  pub delta_temperature_avg: f32,
  pub window_start_date: NaiveDate,
  pub window_end_date: NaiveDate,
  pub sample_minutes: u32,
}

impl EstablishedDeltaBaseline {
  /// The record to pin for `state`, or `None` while still establishing.
  pub fn from_state(state: &DeltaBaselineState) -> Option<Self> {
    match state {
      DeltaBaselineState::Established {
        source,
        delta_temperature_avg,
        window_start_date,
        window_end_date,
        sample_minutes,
      } => Some(Self {
        source: source.clone(),
        delta_temperature_avg: *delta_temperature_avg,
        window_start_date: *window_start_date,
        window_end_date: *window_end_date,
        sample_minutes: *sample_minutes,
      }),
      DeltaBaselineState::Establishing { .. } => None,
    }
  }

  pub fn into_state(self) -> DeltaBaselineState {
    DeltaBaselineState::Established {
      source: self.source,
      delta_temperature_avg: self.delta_temperature_avg,
      window_start_date: self.window_start_date,
      window_end_date: self.window_end_date,
      sample_minutes: self.sample_minutes,
    }
  }
}

/// Establish the ΔT baseline from the first
/// [`COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS`] days on which *one*
/// ambient source's idle band recorded at least
/// [`COOLING_DELTA_BASELINE_QUALIFYING_MINUTES`] paired minutes.
///
/// The rule runs once per source, over that source's rows alone. Where
/// more than one source could establish, the one whose window completes
/// first wins - it is the reference that existed first, and "first N
/// qualifying days" is what makes the derived value stable enough to
/// pin. While no source has established, `qualifying_days` reports the
/// furthest-along source rather than a total, so a machine whose sensor
/// was swapped after four days honestly shows four, not eight.
///
/// `days` must be ordered by date ascending, as
/// `database::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries`
/// returns them.
pub fn derive_delta_baseline_state(
  days: &[ThermalDeltaDailySummary],
) -> DeltaBaselineState {
  let mut furthest_qualifying_days = 0;
  let mut established: Option<DeltaBaselineState> = None;
  // `BTreeMap` iteration is by source label, so a tie on `end_date` is
  // broken the same way on every run.
  for (source, samples) in &samples_by_source(days) {
    match source_baseline_window(samples) {
      window @ BaselineWindow::Established { end_date, .. } => {
        let completes_earlier = established
          .as_ref()
          .and_then(DeltaBaselineState::window)
          .is_none_or(|(_, current_end)| end_date < current_end);
        if completes_earlier {
          established = Some(state_from_window(source, window));
        }
      }
      BaselineWindow::Establishing { qualifying_days } => {
        furthest_qualifying_days = furthest_qualifying_days.max(qualifying_days);
      }
    }
  }

  established.unwrap_or(DeltaBaselineState::Establishing {
    qualifying_days: furthest_qualifying_days,
    required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
  })
}

/// Each ambient source's idle-ΔT series, in the order `days` came in
/// (date ascending), keyed by Sensor Source Label.
///
/// The grouping is the one place the module turns the row-per-source
/// rollup into per-source series, so every per-source reading - the
/// establishment rule, a single source's own baseline, the retention
/// exemption - reads the same projection of a row.
fn samples_by_source(
  days: &[ThermalDeltaDailySummary],
) -> BTreeMap<&str, Vec<DailyBaselineSample>> {
  let mut samples_by_source: BTreeMap<&str, Vec<DailyBaselineSample>> = BTreeMap::new();
  for day in days {
    samples_by_source
      .entry(day.source.as_str())
      .or_default()
      .push(idle_sample(day));
  }
  samples_by_source
}

/// One row projected into the shape the shared establishment rule reads:
/// the day's idle ΔT and the paired minutes behind it.
fn idle_sample(day: &ThermalDeltaDailySummary) -> DailyBaselineSample {
  let idle = day.band(CpuLoadBand::Idle);
  DailyBaselineSample {
    date: day.date,
    value: idle.avg,
    sample_minutes: idle.sample_minutes,
  }
}

/// The shared establishment rule applied to one source's series, at the
/// ΔT baseline's own qualifying bar.
fn source_baseline_window(samples: &[DailyBaselineSample]) -> BaselineWindow {
  derive_baseline_window(
    samples,
    COOLING_DELTA_BASELINE_QUALIFYING_MINUTES,
    COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
  )
}

/// `source`'s establishment outcome as the lifecycle this module reports.
fn state_from_window(source: &str, window: BaselineWindow) -> DeltaBaselineState {
  match window {
    BaselineWindow::Established {
      value,
      start_date,
      end_date,
      sample_minutes,
    } => DeltaBaselineState::Established {
      source: source.to_string(),
      delta_temperature_avg: value,
      window_start_date: start_date,
      window_end_date: end_date,
      sample_minutes,
    },
    BaselineWindow::Establishing { qualifying_days } => {
      DeltaBaselineState::Establishing {
        qualifying_days,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    }
  }
}

/// `source`'s own ΔT baseline, derived from its rows alone by the same
/// rule [`derive_delta_baseline_state`] runs across every source: its
/// first [`COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS`] qualifying days,
/// or how many it has so far.
///
/// Nothing is pinned here. This is the reference for a source other than
/// the pinned one, and it stays stable without a pin because
/// [`retention_exempt_source_windows`] keeps the days it is derived from
/// out of the retention cleanup - see the module docs for why that is
/// equivalent, and why a per-source pin is not available to it.
pub fn derive_source_delta_baseline_state(
  days: &[ThermalDeltaDailySummary],
  source: &str,
) -> DeltaBaselineState {
  let samples: Vec<DailyBaselineSample> = days
    .iter()
    .filter(|day| day.source == source)
    .map(idle_sample)
    .collect();
  state_from_window(source, source_baseline_window(&samples))
}

/// The reference a recent window is measured against: the window's
/// dominant ambient source and the ΔT baseline of that same source.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaBaselineReference<'a> {
  /// The source that covered most of the recent window
  /// (`cooling_band_comparison::dominant_delta_source`), or `None` when
  /// no source paired a minute in it.
  pub recent_source: Option<&'a str>,
  /// The baseline the recent window is compared against. Whenever
  /// `recent_source` is `Some`, this is that source's own baseline -
  /// established or still establishing - never another source's.
  pub baseline: DeltaBaselineState,
}

/// Resolve which ΔT baseline the recent window `[recent_start,
/// recent_end]` is measured against.
///
/// `pinned` is the lifecycle [`resolve_delta_baseline_state`] answered
/// with: the pinned row's, or - before any source has established - the
/// derivation across every source. The rule, in order:
///
/// - The recent window's dominant source is the pinned one: the pinned
///   row is the reference, exactly as before.
/// - The recent window has a dominant source and it is any other one, or
///   nothing is pinned yet: the reference is that source's own baseline
///   ([`derive_source_delta_baseline_state`]), established from its own
///   first qualifying days or reporting its own count while establishing.
///   "Recent minus baseline" is then always one placement against itself,
///   which is the invariant #2062 exists for - the earlier gate that
///   refused the other source enforced the same invariant, but by making
///   the reading unavailable forever once the user changed sensors
///   (#2331).
/// - The recent window has no ambient source at all: there is nothing to
///   pick a source by, so the reference is `pinned` - the same answer the
///   reading gave before, with nothing on the recent side to compare.
///
/// Every consumer that compares a recent ΔT window against a baseline (the
/// load-band comparison, the baseline delta, the co-variate comparison)
/// goes through here so they cannot disagree on what the reference is.
pub fn resolve_delta_baseline_for_window<'a>(
  days: &'a [ThermalDeltaDailySummary],
  pinned: DeltaBaselineState,
  recent_start: NaiveDate,
  recent_end: NaiveDate,
) -> DeltaBaselineReference<'a> {
  let recent_source = dominant_delta_source(days, recent_start, recent_end);
  let baseline = match recent_source {
    Some(source) if pinned.source() != Some(source) => {
      derive_source_delta_baseline_state(days, source)
    }
    _ => pinned,
  };
  DeltaBaselineReference {
    recent_source,
    baseline,
  }
}

/// The calendar span of every source's first
/// [`COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS`] qualifying days - the
/// rows a derived-on-read reference ([`derive_source_delta_baseline_state`])
/// is computed from - as the date ranges the retention cleanup of
/// `cooling_thermal_delta_daily_summary` must exempt.
///
/// This is what makes derive-on-read as stable as pinning (#2331). "First
/// N qualifying days" can only move if a row before the Nth is created or
/// deleted; the rows before the first qualifying day are non-qualifying by
/// definition, so deleting them cannot promote anything into the window,
/// and the rows after the Nth are never consulted. Keeping the span itself
/// therefore fixes the answer for as long as the table exists, the same
/// way the pinned row fixes it for the first source.
///
/// A source that has not established yet is exempted too, over the days
/// it has so far: otherwise its count could fall as they aged out, and the
/// window it eventually establishes would depend on when the cleanup ran
/// relative to its seventh day. A source with no qualifying day
/// contributes nothing. Non-qualifying rows *inside* a span survive with
/// it (the exemption is a date range, shared with the delete's other
/// windows), which keeps nothing a reader depends on but costs at most the
/// gaps between one source's qualifying days.
///
/// `days` must be ordered by date ascending, as
/// `select_all_thermal_delta_daily_summaries` returns them.
pub fn retention_exempt_source_windows(
  days: &[ThermalDeltaDailySummary],
) -> Vec<(NaiveDate, NaiveDate)> {
  samples_by_source(days)
    .values()
    .filter_map(|samples| {
      let window = first_qualifying_days(
        samples,
        COOLING_DELTA_BASELINE_QUALIFYING_MINUTES,
        COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      );
      Some((window.first()?.date, window.last()?.date))
    })
    .collect()
}

/// Resolve the ΔT baseline lifecycle: the pinned row if one exists,
/// otherwise derive it from `days` and pin it the moment it establishes.
///
/// Every loader that needs this state must go through here rather than
/// calling [`derive_delta_baseline_state`] directly, for the same reason
/// the absolute baseline insists on its resolver: otherwise it ignores
/// the pinned row and drifts as the rollup rows behind the original
/// establishment age out.
/// Test-only since #2134: [`resolve_delta_baseline_state`] is routed
/// through the dispatch boundary instead of an explicit pool.
#[cfg(test)]
pub(crate) async fn resolve_delta_baseline_state_from_pool(
  pool: &sqlx::SqlitePool,
  days: &[ThermalDeltaDailySummary],
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

/// [`resolve_delta_baseline_state_from_pool`], routed through the dispatch
/// boundary (#2134) instead of an explicit pool. Every production caller
/// (this module's own pinning, the band comparison, the baseline delta and
/// the co-variate comparison) uses this, not the `_from_pool` variant.
pub(crate) async fn resolve_delta_baseline_state(
  days: &[ThermalDeltaDailySummary],
) -> Result<DeltaBaselineState, crate::infrastructure::database::dispatch::DispatchError>
{
  use crate::infrastructure::database::dispatch;

  match dispatch::cooling_delta_baseline::select_established_delta_baseline().await? {
    Some(pinned) => Ok(pinned.into_state()),
    None => {
      let derived = derive_delta_baseline_state(days);
      if let Some(baseline) = EstablishedDeltaBaseline::from_state(&derived) {
        // #2271: same conversion-window guard as
        // `cooling_baseline::resolve_baseline_state` - see its comment for
        // why the permit must be held across the write itself, not just
        // checked beforehand.
        if let Some(_permit) = dispatch::acquire_write_permit().await {
          // Write-once bookkeeping, not part of the answer - a transient
          // failure must not turn a valid derivation into a read error.
          // Retried on the next resolution, same rule as the `_from_pool`
          // resolver above.
          if let Err(e) =
            dispatch::cooling_delta_baseline::insert_established_delta_baseline(&baseline)
              .await
          {
            crate::log_error!(
              "Failed to pin the established ΔT cooling baseline; retrying on the next resolution",
              "persistence::cooling_delta_baseline::resolve_delta_baseline_state",
              Some(e.to_string())
            );
          }
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
  use crate::infrastructure::database::dispatch;

  let resolve = async {
    let days =
      dispatch::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries()
        .await?;
    resolve_delta_baseline_state(&days).await
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
  use crate::persistence::cooling_rollup::BandSummary;

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

  /// One source's rollup row whose idle band carries `delta` over
  /// `minutes` paired minutes, or coverage with no idle ΔT at all when
  /// `delta` is `None`.
  pub(crate) fn day(
    date: NaiveDate,
    source: &str,
    delta: Option<(f32, u32)>,
  ) -> ThermalDeltaDailySummary {
    let (idle, coverage_minutes) = match delta {
      Some((avg, minutes)) => (band(avg, minutes), minutes),
      None => (BandSummary::default(), 1),
    };
    ThermalDeltaDailySummary {
      date,
      source: source.to_string(),
      coverage_minutes,
      idle,
      low: BandSummary::default(),
      mid: BandSummary::default(),
      high: BandSummary::default(),
    }
  }

  /// `count` consecutive qualifying days for `source` starting at `start`.
  fn qualifying_days(
    source: &str,
    start: NaiveDate,
    count: i64,
    delta: f32,
  ) -> Vec<ThermalDeltaDailySummary> {
    (0..count)
      .map(|offset| {
        day(
          start + chrono::Duration::days(offset),
          source,
          Some((delta, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
        )
      })
      .collect()
  }

  const REQUIRED: i64 = COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS as i64;

  #[test]
  fn a_machine_with_no_ambient_data_stays_establishing_at_zero() {
    assert_eq!(
      derive_delta_baseline_state(&[]),
      DeltaBaselineState::Establishing {
        qualifying_days: 0,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn coverage_without_an_idle_delta_does_not_qualify() {
    let days: Vec<_> = (0..30)
      .map(|offset| {
        day(
          date(2026, 8, 1) + chrono::Duration::days(offset),
          "Desk",
          None,
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
  fn the_required_number_of_qualifying_days_establishes_the_baseline() {
    let start = date(2026, 8, 1);
    let days = qualifying_days("Desk", start, REQUIRED, 12.0);

    let state = derive_delta_baseline_state(&days);

    assert_eq!(
      state,
      DeltaBaselineState::Established {
        source: "Desk".to_string(),
        delta_temperature_avg: 12.0,
        window_start_date: start,
        window_end_date: start + chrono::Duration::days(REQUIRED - 1),
        sample_minutes: COOLING_DELTA_BASELINE_QUALIFYING_MINUTES
          * COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn the_baseline_records_the_source_it_was_established_from() {
    let days = qualifying_days("Living Room", date(2026, 8, 1), REQUIRED, 12.0);

    assert_eq!(
      derive_delta_baseline_state(&days).source(),
      Some("Living Room")
    );
  }

  #[test]
  fn switching_sources_never_mixes_two_placements_into_one_baseline() {
    // The property #2062 exists for: four days against the desk sensor,
    // then the user switches to the one across the room for four more.
    // Eight qualifying days in total, but no single sensor has seven, so
    // nothing may establish - and the progress reported is one sensor's,
    // not the sum.
    let mut days = qualifying_days("Desk", date(2026, 8, 1), 4, 12.0);
    days.extend(qualifying_days("Living Room", date(2026, 8, 5), 4, 15.0));

    assert_eq!(
      derive_delta_baseline_state(&days),
      DeltaBaselineState::Establishing {
        qualifying_days: 4,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn two_sources_sharing_the_same_days_each_establish_from_their_own_rows() {
    // Both sensors archived every day of the same week: each source's
    // window is its own, and the value pinned is one sensor's ΔT rather
    // than the 13.5 a mixture would give.
    let start = date(2026, 8, 1);
    let mut days = Vec::new();
    for offset in 0..REQUIRED {
      let d = start + chrono::Duration::days(offset);
      days.push(day(
        d,
        "Desk",
        Some((12.0, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
      ));
      days.push(day(
        d,
        "Living Room",
        Some((15.0, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
      ));
    }

    let state = derive_delta_baseline_state(&days);

    // Both complete on the same day; the tie falls to the label that
    // sorts first, deterministically.
    assert_eq!(state.source(), Some("Desk"));
    let DeltaBaselineState::Established {
      delta_temperature_avg,
      ..
    } = state
    else {
      panic!("expected an established ΔT baseline");
    };
    assert_eq!(delta_temperature_avg, 12.0);
  }

  #[test]
  fn the_source_whose_window_completes_first_is_the_one_established() {
    // The desk sensor started later but the living-room one was only
    // reachable every other day, so the desk's seventh qualifying day
    // comes first.
    let mut days = Vec::new();
    for offset in 0..(REQUIRED * 2) {
      let d = date(2026, 8, 1) + chrono::Duration::days(offset);
      if offset % 2 == 0 {
        days.push(day(
          d,
          "Living Room",
          Some((15.0, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
        ));
      }
      if offset >= 3 {
        days.push(day(
          d,
          "Desk",
          Some((12.0, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
        ));
      }
    }

    let state = derive_delta_baseline_state(&days);

    assert_eq!(state.source(), Some("Desk"));
    assert_eq!(state.window(), Some((date(2026, 8, 4), date(2026, 8, 10))));
  }

  #[test]
  fn one_day_short_of_the_requirement_is_still_establishing() {
    let days = qualifying_days("Desk", date(2026, 8, 1), REQUIRED - 1, 12.0);

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
    let days: Vec<_> = (0..REQUIRED)
      .map(|offset| {
        day(
          date(2026, 8, 1) + chrono::Duration::days(offset),
          "Desk",
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
    for offset in 0..(REQUIRED * 2) {
      let d = start + chrono::Duration::days(offset);
      // Every other day has no idle ΔT at all.
      days.push(if offset % 2 == 0 {
        day(
          d,
          "Desk",
          Some((12.0, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
        )
      } else {
        day(d, "Desk", None)
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
    // from the sensor's own first days. With the rollup row-per-source
    // the ambient-free months simply have no rows at all.
    let ambient_start = date(2026, 8, 1);
    let days = qualifying_days("Desk", ambient_start, REQUIRED, 12.0);

    let state = derive_delta_baseline_state(&days);

    assert_eq!(
      state.window(),
      Some((
        ambient_start,
        ambient_start + chrono::Duration::days(REQUIRED - 1)
      )),
      "the ΔT window must begin at the first day ambient data actually exists"
    );
  }

  #[test]
  fn later_hotter_days_do_not_move_an_established_window() {
    // "First N qualifying days", never the most recent ones - the
    // property that makes the derived value stable enough to pin.
    let start = date(2026, 8, 1);
    let mut days = qualifying_days("Desk", start, REQUIRED, 12.0);
    days.extend(qualifying_days("Desk", date(2026, 10, 1), REQUIRED, 40.0));

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

  // ── the reference a recent window is measured against ──

  /// The pinned row as the resolver receives it: `Desk`, established over
  /// the first week of August at a ΔT the rows themselves no longer say,
  /// so a test can tell "used the pin" from "re-derived the same source".
  fn pinned_desk() -> DeltaBaselineState {
    DeltaBaselineState::Established {
      source: "Desk".to_string(),
      delta_temperature_avg: 10.0,
      window_start_date: date(2026, 8, 1),
      window_end_date: date(2026, 8, 7),
      sample_minutes: 210,
    }
  }

  /// The recent window the resolver is asked about: the week ending 9-20.
  const RECENT: (NaiveDate, NaiveDate) = (
    NaiveDate::from_ymd_opt(2026, 9, 14).unwrap(),
    NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
  );

  #[test]
  fn the_pinned_row_is_the_reference_for_a_recent_window_of_its_own_source() {
    // The rows would derive 12.0 for the desk sensor; the pin says 10.0.
    // The pin wins, as it always has: that is what makes it undriftable.
    let mut days = qualifying_days("Desk", date(2026, 8, 1), REQUIRED, 12.0);
    days.extend(qualifying_days("Desk", RECENT.0, REQUIRED, 13.0));

    let reference =
      resolve_delta_baseline_for_window(&days, pinned_desk(), RECENT.0, RECENT.1);

    assert_eq!(reference.recent_source, Some("Desk"));
    assert_eq!(reference.baseline, pinned_desk());
  }

  #[test]
  fn a_recent_window_from_another_source_is_measured_against_that_sources_own_baseline() {
    // The defect this resolver fixes (#2331), in the maintainer's shape: the
    // pin was established against one meter, then the app was rebound to
    // another, which has since collected weeks of data. The new meter's
    // first qualifying week is its reference - not "not comparable"
    // forever because the pin can never be rewritten.
    let mut days = qualifying_days("Desk", date(2026, 8, 1), REQUIRED, 12.0);
    let living_room_start = date(2026, 9, 1);
    days.extend(qualifying_days(
      "Living Room",
      living_room_start,
      REQUIRED,
      15.0,
    ));
    days.extend(qualifying_days("Living Room", RECENT.0, REQUIRED, 16.0));

    let reference =
      resolve_delta_baseline_for_window(&days, pinned_desk(), RECENT.0, RECENT.1);

    assert_eq!(reference.recent_source, Some("Living Room"));
    assert_eq!(
      reference.baseline,
      DeltaBaselineState::Established {
        source: "Living Room".to_string(),
        delta_temperature_avg: 15.0,
        window_start_date: living_room_start,
        window_end_date: living_room_start + chrono::Duration::days(REQUIRED - 1),
        sample_minutes: COOLING_DELTA_BASELINE_QUALIFYING_MINUTES
          * COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn a_recent_window_from_a_source_still_establishing_reports_that_sources_own_count() {
    // Three qualifying days against the new meter, all inside the recent
    // window. The progress reported is the new meter's three - not the
    // pinned sensor's established state, and not the furthest-along
    // source's count across the table.
    let mut days = qualifying_days("Desk", date(2026, 8, 1), REQUIRED, 12.0);
    days.extend(qualifying_days("Living Room", RECENT.0, 3, 15.0));

    let reference =
      resolve_delta_baseline_for_window(&days, pinned_desk(), RECENT.0, RECENT.1);

    assert_eq!(reference.recent_source, Some("Living Room"));
    assert_eq!(
      reference.baseline,
      DeltaBaselineState::Establishing {
        qualifying_days: 3,
        required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
      }
    );
  }

  #[test]
  fn a_recent_window_with_no_ambient_source_falls_back_to_the_pinned_row() {
    // The sensor was offline all week: there is no source to pick a
    // reference by, so the reading says what it said before - the pinned
    // baseline, with nothing on the recent side.
    let days = qualifying_days("Desk", date(2026, 8, 1), REQUIRED, 12.0);

    let reference =
      resolve_delta_baseline_for_window(&days, pinned_desk(), RECENT.0, RECENT.1);

    assert_eq!(reference.recent_source, None);
    assert_eq!(reference.baseline, pinned_desk());
  }

  #[test]
  fn before_any_pin_a_recent_window_is_measured_against_its_own_sources_baseline() {
    // Nothing pinned yet (the resolver was handed the cross-source
    // derivation, still establishing); the recent window's source has
    // its own week. Its own baseline is the reference, and the lifecycle
    // handed in is not what is reported.
    let mut days = qualifying_days("Living Room", date(2026, 9, 1), REQUIRED, 15.0);
    days.extend(qualifying_days("Living Room", RECENT.0, REQUIRED, 16.0));
    let establishing = DeltaBaselineState::Establishing {
      qualifying_days: 0,
      required_days: COOLING_BASELINE_REQUIRED_QUALIFYING_DAYS,
    };

    let reference =
      resolve_delta_baseline_for_window(&days, establishing, RECENT.0, RECENT.1);

    assert_eq!(reference.baseline.source(), Some("Living Room"));
    assert_eq!(
      reference.baseline.window(),
      Some((date(2026, 9, 1), date(2026, 9, 7)))
    );
  }

  // ── what retention must keep for a derived-on-read reference ──

  #[test]
  fn retention_exempts_each_sources_first_qualifying_days_whether_established_or_not() {
    // Desk: a consecutive first week, then more days that must not widen
    // the span. Living Room: non-qualifying days first (deletable), then
    // seven qualifying days on alternate dates (a 13-day span), then
    // later days that are not part of the reference. Bedroom: three
    // qualifying days so far, exempt over exactly those. Hallway: never a
    // qualifying day, so nothing to keep.
    let mut days = qualifying_days("Desk", date(2026, 8, 1), REQUIRED, 12.0);
    days.extend(qualifying_days("Desk", date(2026, 8, 20), 3, 13.0));
    for offset in 0..3 {
      days.push(day(
        date(2026, 9, 1) + chrono::Duration::days(offset),
        "Living Room",
        None,
      ));
    }
    for offset in 0..REQUIRED {
      days.push(day(
        date(2026, 9, 4) + chrono::Duration::days(offset * 2),
        "Living Room",
        Some((15.0, COOLING_DELTA_BASELINE_QUALIFYING_MINUTES)),
      ));
    }
    days.extend(qualifying_days("Living Room", date(2026, 10, 1), 2, 15.0));
    days.extend(qualifying_days("Bedroom", date(2026, 9, 10), 3, 14.0));
    days.push(day(date(2026, 9, 12), "Hallway", None));
    days.sort_by_key(|row| row.date);

    let windows = retention_exempt_source_windows(&days);

    assert_eq!(
      windows,
      vec![
        // Sources come out in label order.
        (date(2026, 9, 10), date(2026, 9, 12)),
        (date(2026, 8, 1), date(2026, 8, 7)),
        (date(2026, 9, 4), date(2026, 9, 16)),
      ]
    );
  }

  #[test]
  fn a_table_with_no_qualifying_day_exempts_nothing() {
    let days: Vec<_> = (0..5)
      .map(|offset| {
        day(
          date(2026, 8, 1) + chrono::Duration::days(offset),
          "Desk",
          None,
        )
      })
      .collect();

    assert_eq!(retention_exempt_source_windows(&days), Vec::new());
    assert_eq!(retention_exempt_source_windows(&[]), Vec::new());
  }

  /// #2271: same conversion-window guard as
  /// `cooling_baseline`'s own `a_read_during_conversion_does_not_pin_and_pins_normally_once_the_window_closes`
  /// - see that test's doc for why this exercises the resolver's
  /// select/derive/gate/insert shape directly against an isolated pool
  /// instead of driving `resolve_delta_baseline_state` itself.
  #[cfg(feature = "duckdb-archive")]
  #[tokio::test]
  async fn a_read_during_conversion_does_not_pin_the_delta_baseline_and_pins_normally_after()
   {
    use crate::infrastructure::database;
    use crate::infrastructure::database::dispatch;
    use crate::infrastructure::database::test_schema::{
      COOLING_DELTA_BASELINE_DDL, create_tables,
    };
    use sqlx::SqlitePool;

    let _guard = dispatch::test_support::lock_write_gate().await;
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    create_tables(&pool, &[COOLING_DELTA_BASELINE_DDL]).await;
    let days = qualifying_days("Desk", date(2026, 8, 1), REQUIRED, 12.0);
    let derived = derive_delta_baseline_state(&days);
    let established =
      EstablishedDeltaBaseline::from_state(&derived).expect("this window must establish");

    // The App marks the lifecycle owner `Converting`: the gate the
    // resolver must hold a permit from before writing closes.
    dispatch::set_writable(false).await;
    if let Some(_permit) = dispatch::acquire_write_permit().await {
      database::cooling_delta_baseline::insert_established_delta_baseline_from_pool(
        &pool,
        &established,
        chrono::Utc::now(),
      )
      .await
      .unwrap();
    }
    assert_eq!(
      database::cooling_delta_baseline::select_established_delta_baseline_from_pool(
        &pool
      )
      .await
      .unwrap(),
      None,
      "the ΔT pin must not be written while the gate reports Converting"
    );

    // The conversion resolves and the owner leaves `Converting`: the gate
    // reopens, and the very next resolution's write-back succeeds.
    dispatch::set_writable(true).await;
    if let Some(_permit) = dispatch::acquire_write_permit().await {
      database::cooling_delta_baseline::insert_established_delta_baseline_from_pool(
        &pool,
        &established,
        chrono::Utc::now(),
      )
      .await
      .unwrap();
    }
    assert_eq!(
      database::cooling_delta_baseline::select_established_delta_baseline_from_pool(
        &pool
      )
      .await
      .unwrap(),
      Some(established),
      "the ΔT pin establishes normally once the conversion window closes"
    );
  }
}
