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

use chrono::{Duration, NaiveDate};

use crate::persistence::cooling_baseline::{
  BaselineState, COOLING_BASELINE_RECENT_WINDOW_DAYS,
};
#[cfg(test)]
use crate::persistence::cooling_rollup::{AmbientDeltaSummary, PowerSummary};
use crate::persistence::cooling_rollup::{BandSummary, CpuLoadBand, DailyCoolingSummary};

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
/// Subtracting `recent.delta_avg` from `baseline.delta_avg` is legitimate
/// where subtracting a CPU summary from an ambient summary is not: both
/// sides here are already per-minute ΔT values that were paired before
/// aggregation, so this compares one period against another rather than
/// reconstructing a pairing that never happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientAdjustedBandComparison {
  pub baseline: BandDeltaWindowSummary,
  pub recent: BandDeltaWindowSummary,
  /// Whether both windows carry enough paired minutes for the
  /// ambient-adjusted reading to mean anything, on the same
  /// both-sides-or-nothing rule as [`BandComparison::comparable`].
  pub comparable: bool,
}

/// One [`CpuLoadBand`]'s baseline-vs-recent comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandComparison {
  pub band: CpuLoadBand,
  pub baseline: BandWindowSummary,
  pub recent: BandWindowSummary,
  /// Whether both windows carry enough evidence for `recent` minus
  /// `baseline` to mean anything. `false` means present "not comparable"
  /// rather than a number, even though both fields above still carry
  /// whatever (insufficient) data was found.
  pub comparable: bool,
  /// The ambient-adjusted reading of the same two windows (#2045), or
  /// `None` when neither window recorded a single ΔT minute for this
  /// band.
  ///
  /// `None` and `Some(.. { comparable: false, .. })` say different things
  /// on purpose. `None` means this machine has no ambient evidence here at
  /// all - the normal state on an install with no environmental sensor,
  /// and what keeps every ambient-unaware reading of this response exactly
  /// what it was before #2045. `Some` with `comparable: false` means
  /// ambient data exists but one window is too thin to compare, which is
  /// worth telling the user about because it will resolve on its own.
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
  },
}

/// Derive the load-band comparison from every completed day's rollup row
/// and the current baseline lifecycle state.
///
/// `window_end_date` is the most recent completed local day (yesterday),
/// matching [`crate::persistence::cooling_baseline::derive_cooling_baseline`].
pub fn derive_band_comparison(
  days: &[DailyCoolingSummary],
  baseline_state: BaselineState,
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
    window_end_date - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);

  let bands = [
    CpuLoadBand::Idle,
    CpuLoadBand::Low,
    CpuLoadBand::Mid,
    CpuLoadBand::High,
  ]
  .map(|band| {
    let baseline = band_window_summary(days, band, baseline_start, baseline_end);
    let recent = band_window_summary(days, band, recent_start, window_end_date);
    let comparable = baseline.is_comparable() && recent.is_comparable();
    BandComparison {
      band,
      baseline,
      recent,
      comparable,
      ambient_adjusted: ambient_adjusted_band_comparison(
        days,
        band,
        (baseline_start, baseline_end),
        (recent_start, window_end_date),
      ),
    }
  });

  CoolingBandComparison::Established {
    baseline_window_start_date: baseline_start,
    baseline_window_end_date: baseline_end,
    recent_window_start_date: recent_start,
    recent_window_end_date: window_end_date,
    bands: Box::new(bands),
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

/// [`band_window_summary`] over the day's ΔT bands instead of its absolute
/// temperature bands (#2045). Same sample-minute weighting - the ΔT band's
/// own `sample_minutes`, which counts only the minutes that carried both
/// readings, so a day whose ambient sensor dropped out for half the day
/// weighs exactly the half it observed.
pub(crate) fn band_delta_window_summary(
  days: &[DailyCoolingSummary],
  band: CpuLoadBand,
  start: NaiveDate,
  end: NaiveDate,
) -> BandDeltaWindowSummary {
  let mut weighted_sum = 0.0f64;
  let mut sample_minutes: u64 = 0;

  for day in days.iter().filter(|d| d.date >= start && d.date <= end) {
    let summary = day.ambient.band(band);
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

/// The ambient-adjusted reading of one band's two windows, or `None` when
/// neither window recorded a ΔT minute for this band.
///
/// The `None` case is the whole reason ambient stays optional: a machine
/// with no environmental sensor produces it for every band, and the
/// response then carries exactly the facts it carried before #2045.
fn ambient_adjusted_band_comparison(
  days: &[DailyCoolingSummary],
  band: CpuLoadBand,
  baseline_window: (NaiveDate, NaiveDate),
  recent_window: (NaiveDate, NaiveDate),
) -> Option<AmbientAdjustedBandComparison> {
  let baseline =
    band_delta_window_summary(days, band, baseline_window.0, baseline_window.1);
  let recent = band_delta_window_summary(days, band, recent_window.0, recent_window.1);

  if baseline.sample_minutes == 0 && recent.sample_minutes == 0 {
    return None;
  }

  Some(AmbientAdjustedBandComparison {
    comparable: baseline.is_comparable() && recent.is_comparable(),
    baseline,
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
  let yesterday = today - Duration::days(1);

  Ok(derive_band_comparison(&days, baseline_state, yesterday))
}

/// [`load_cooling_band_comparison_from_pool`] against Core's process-wide
/// pool.
pub async fn load_cooling_band_comparison() -> Result<CoolingBandComparison, sqlx::Error>
{
  let pool = crate::infrastructure::database::db::get_pool().await?;
  load_cooling_band_comparison_from_pool(&pool, chrono::Local::now().date_naive()).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::test_schema::{
    COOLING_BASELINE_DDL, COOLING_DAILY_SUMMARY_DDL, create_tables,
  };
  use crate::persistence::cooling_baseline::COOLING_BASELINE_QUALIFYING_IDLE_MINUTES;

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

  /// One day carrying only an idle temperature band and, optionally, an
  /// idle ΔT band on top of it (#2045).
  fn idle_day(
    date: NaiveDate,
    temperature: f32,
    minutes: u32,
    delta: Option<(f32, u32)>,
  ) -> DailyCoolingSummary {
    DailyCoolingSummary {
      date,
      coverage_minutes: 1440,
      idle: band(temperature, minutes),
      low: empty_band(),
      mid: empty_band(),
      high: empty_band(),
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
    let recent_end = date(2026, 8, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60, None),
      idle_day(recent_start, 50.0, 60, None),
    ];

    let result = derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_start),
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
    assert!(idle.comparable);
  }

  #[test]
  fn an_ambient_adjusted_reading_separates_a_warmer_room_from_worse_cooling() {
    // The whole point of the feature. The absolute idle temperature rose
    // 20 K between the windows, but the ΔT held flat: the room got
    // warmer, the cooling did not degrade.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 8, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60, Some((10.0, 60))),
      idle_day(recent_start, 50.0, 60, Some((10.0, 60))),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_start),
      recent_end,
    ));

    assert_eq!(idle.recent.temperature_avg, Some(50.0));
    assert_eq!(idle.baseline.temperature_avg, Some(30.0));
    let adjusted = idle.ambient_adjusted.expect("ambient data exists");
    assert_eq!(adjusted.baseline.delta_avg, Some(10.0));
    assert_eq!(adjusted.recent.delta_avg, Some(10.0));
    assert!(adjusted.comparable);
  }

  #[test]
  fn each_ambient_adjusted_window_is_weighted_by_its_own_delta_sample_minutes() {
    // The ΔT band's own sample minutes do the weighting, not the
    // temperature band's: a day whose ambient sensor covered only part of
    // the day must weigh exactly the part it observed.
    let baseline_start = date(2026, 8, 1);
    let baseline_end = date(2026, 8, 7);
    let recent_end = date(2026, 8, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      // 1440 temperature minutes but only 30 of them paired.
      idle_day(baseline_start, 30.0, 1440, Some((8.0, 30))),
      idle_day(baseline_end, 30.0, 1440, Some((12.0, 90))),
      idle_day(recent_start, 50.0, 1440, Some((20.0, 60))),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_end),
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
    let recent_end = date(2026, 8, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let short = COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES - 1;
    let days = vec![
      idle_day(baseline_start, 30.0, 60, Some((10.0, short))),
      idle_day(
        recent_start,
        50.0,
        60,
        Some((14.0, COOLING_AMBIENT_ADJUSTED_MINIMUM_SAMPLE_MINUTES)),
      ),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle
      .ambient_adjusted
      .expect("ambient evidence exists, it is merely thin");
    assert!(!adjusted.comparable);
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
    let recent_end = date(2026, 8, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let days = vec![
      idle_day(baseline_start, 30.0, 60, None),
      idle_day(recent_start, 50.0, 60, Some((14.0, 60))),
    ];

    let idle = idle_comparison(derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_start),
      recent_end,
    ));

    let adjusted = idle.ambient_adjusted.expect("the recent side has ambient");
    assert!(!adjusted.comparable);
    assert_eq!(adjusted.baseline.sample_minutes, 0);
    assert_eq!(adjusted.baseline.delta_avg, None);
    assert_eq!(adjusted.recent.delta_avg, Some(14.0));
  }

  #[test]
  fn an_ambient_adjusted_reading_stays_within_its_own_band() {
    // A ΔT recorded in the high band must not surface on the idle band's
    // ambient-adjusted reading.
    let baseline_start = date(2026, 8, 1);
    let recent_end = date(2026, 8, 20);
    let recent_start =
      recent_end - Duration::days(COOLING_BASELINE_RECENT_WINDOW_DAYS as i64 - 1);
    let high_only = |date: NaiveDate, delta: f32| DailyCoolingSummary {
      date,
      coverage_minutes: 1440,
      idle: empty_band(),
      low: empty_band(),
      mid: empty_band(),
      high: band(70.0, 60),
      power: PowerSummary::default(),
      ambient: AmbientDeltaSummary {
        coverage_minutes: 60,
        high: band(delta, 60),
        ..AmbientDeltaSummary::default()
      },
    };
    let days = vec![
      high_only(baseline_start, 40.0),
      high_only(recent_start, 45.0),
    ];

    let result = derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_start),
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
      BaselineState::Establishing {
        qualifying_days: 2,
        required_days: 7,
      },
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
    let recent_end = date(2026, 8, 20);
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
        ambient: AmbientDeltaSummary::default(),
      },
      DailyCoolingSummary {
        date: baseline_end,
        coverage_minutes: 1440,
        idle: band(32.0, 60),
        low: band(42.0, 90),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
        ambient: AmbientDeltaSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_start,
        coverage_minutes: 1440,
        idle: band(50.0, 60),
        low: empty_band(),
        mid: empty_band(),
        high: band(70.0, 40),
        power: PowerSummary::default(),
        ambient: AmbientDeltaSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_end,
        coverage_minutes: 1440,
        idle: band(52.0, 60),
        low: empty_band(),
        mid: empty_band(),
        high: band(72.0, 20),
        power: PowerSummary::default(),
        ambient: AmbientDeltaSummary::default(),
      },
    ];

    let result = derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_end),
      recent_end,
    );

    match result {
      CoolingBandComparison::Established {
        baseline_window_start_date,
        baseline_window_end_date,
        recent_window_start_date,
        recent_window_end_date,
        bands,
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
        assert!(idle.comparable);

        let low = bands.iter().find(|b| b.band == CpuLoadBand::Low).unwrap();
        assert_eq!(low.baseline.sample_minutes, 120);
        let expected_low = (40.0 * 30.0 + 42.0 * 90.0) / 120.0;
        assert!((low.baseline.temperature_avg.unwrap() - expected_low).abs() < 0.001);
        assert_eq!(low.recent.sample_minutes, 0);
        assert_eq!(low.recent.temperature_avg, None);
        assert!(!low.comparable, "no recent low-band evidence at all");

        let mid = bands.iter().find(|b| b.band == CpuLoadBand::Mid).unwrap();
        assert_eq!(mid.baseline.sample_minutes, 0);
        assert_eq!(mid.recent.sample_minutes, 0);
        assert!(!mid.comparable);

        let high = bands.iter().find(|b| b.band == CpuLoadBand::High).unwrap();
        assert_eq!(high.baseline.sample_minutes, 0);
        assert_eq!(high.recent.sample_minutes, 60);
        assert!(
          !high.comparable,
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
    let recent_end = date(2026, 8, 20);
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
        ambient: AmbientDeltaSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_start,
        coverage_minutes: 1440,
        idle: band(50.0, COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
        ambient: AmbientDeltaSummary::default(),
      },
    ];

    let result = derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_end),
      recent_end,
    );

    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };
    let idle = bands.iter().find(|b| b.band == CpuLoadBand::Idle).unwrap();
    assert!(idle.comparable);
  }

  #[test]
  fn a_band_one_minute_short_on_either_side_is_not_comparable() {
    let baseline_start = date(2026, 8, 1);
    let baseline_end = date(2026, 8, 1);
    let recent_end = date(2026, 8, 20);
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
        ambient: AmbientDeltaSummary::default(),
      },
      DailyCoolingSummary {
        date: recent_start,
        coverage_minutes: 1440,
        idle: band(50.0, COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
        ambient: AmbientDeltaSummary::default(),
      },
    ];

    let result = derive_band_comparison(
      &days,
      established_baseline(baseline_start, baseline_end),
      recent_end,
    );

    let CoolingBandComparison::Established { bands, .. } = result else {
      panic!("expected an established comparison");
    };
    let idle = bands.iter().find(|b| b.band == CpuLoadBand::Idle).unwrap();
    assert!(!idle.comparable);
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
      ambient: AmbientDeltaSummary::default(),
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
