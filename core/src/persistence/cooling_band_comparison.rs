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
use crate::persistence::cooling_rollup::PowerSummary;
use crate::persistence::cooling_rollup::{BandSummary, CpuLoadBand, DailyCoolingSummary};

/// Minimum sample minutes a band's window must carry before that band's
/// comparison is meaningful. Applied independently to the baseline side
/// and the recent side of each band - a band comparable on one side but
/// not the other is still not comparable overall (DP-02: no delta
/// computed from a handful of minutes as if it were a measurement).
pub const COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES: u32 = 30;

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
    bands: [BandComparison; 4],
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
    }
  });

  CoolingBandComparison::Established {
    baseline_window_start_date: baseline_start,
    baseline_window_end_date: baseline_end,
    recent_window_start_date: recent_start,
    recent_window_end_date: window_end_date,
    bands,
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
          coverage_minutes INTEGER NOT NULL,
          cpu_power_avg REAL,
          cpu_power_max REAL,
          cpu_power_min REAL,
          power_sample_minutes INTEGER NOT NULL DEFAULT 0
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
