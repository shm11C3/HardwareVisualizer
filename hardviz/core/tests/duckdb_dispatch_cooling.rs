#![cfg(feature = "duckdb-archive")]
//! Dispatch routing (#2134) for Cooling: the six rolled-up projections,
//! written as one transaction, and both pinned baselines.
//!
//! Companion to `duckdb_dispatch.rs`, `duckdb_dispatch_raw_archive_families.rs`
//! and `duckdb_dispatch_storage_health.rs`: this file only proves that
//! `dispatch::cooling_rollup`, `dispatch::cooling_daily_summary` and friends,
//! and `dispatch::cooling_baseline`/`dispatch::cooling_delta_baseline` reach
//! `native_database::cooling_*` once selected and stay on SQLite until then -
//! including that the rollup write stays one transaction on both paths.
//! `duckdb_cooling.rs` already proves the native family reproduces SQLite bit
//! for bit, storage-class edge cases included.

mod native_support;

use chrono::NaiveDate;
use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AuthorityState, select_native_database,
};
use hardviz_core::persistence::cooling_baseline::EstablishedBaseline;
use hardviz_core::persistence::cooling_covariate_rollup::{
  CovariateDailySummary, CovariateDaySummary, FanCovariateDailySummary,
  PairedFitStatistics,
};
use hardviz_core::persistence::cooling_delta_baseline::EstablishedDeltaBaseline;
use hardviz_core::persistence::cooling_fan_rollup::FanDailySummary;
use hardviz_core::persistence::cooling_hourly_rollup::HourlyCoolingSummary;
use hardviz_core::persistence::cooling_rollup::{
  BandSummary, CpuLoadBand, DailyCoolingSummary, PowerSummary,
};
use hardviz_core::persistence::cooling_thermal_delta_rollup::ThermalDeltaDailySummary;
use native_support::{NativeFixture, app_native_schema};

fn day(text: &str) -> NaiveDate {
  text.parse().unwrap()
}

fn hour(text: &str) -> chrono::NaiveDateTime {
  chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
}

fn band(avg: f32, minutes: u32) -> BandSummary {
  BandSummary {
    avg: Some(avg),
    max: Some(avg + 2.0),
    min: Some(avg - 2.0),
    sample_minutes: minutes,
  }
}

fn empty_band() -> BandSummary {
  BandSummary {
    avg: None,
    max: None,
    min: None,
    sample_minutes: 0,
  }
}

fn power(avg: f32, minutes: u32) -> PowerSummary {
  PowerSummary {
    avg: Some(avg),
    max: Some(avg + 1.0),
    min: Some(avg - 1.0),
    sample_minutes: minutes,
  }
}

fn fit(n: u32) -> PairedFitStatistics {
  PairedFitStatistics {
    n,
    sum_x: 1.5,
    sum_y: 2.5,
    sum_xy: 3.5,
    sum_xx: 4.5,
    sum_yy: 5.5,
  }
}

/// One rolled-up day's worth of every projection `persist_day_rollup`
/// writes, for `date`.
#[allow(clippy::type_complexity)]
fn rollup_for(
  date: NaiveDate,
) -> (
  DailyCoolingSummary,
  Vec<HourlyCoolingSummary>,
  Vec<FanDailySummary>,
  Vec<ThermalDeltaDailySummary>,
  CovariateDaySummary,
) {
  let summary = DailyCoolingSummary {
    date,
    coverage_minutes: 90,
    idle: band(25.0, 30),
    low: band(35.0, 30),
    mid: empty_band(),
    high: empty_band(),
    power: power(8.0, 60),
  };
  let hours = vec![HourlyCoolingSummary {
    hour_start: hour(&format!("{date} 00:00:00")),
    cpu_usage_avg: Some(12.0),
    cpu_temperature_avg: Some(30.0),
    sample_minutes: 45,
  }];
  let fans = vec![FanDailySummary {
    date,
    source: "Fan 1".to_owned(),
    rpm_avg: 1200.0,
    rpm_max: 1300,
    rpm_min: 1100,
    sample_minutes: 60,
  }];
  let thermal_deltas = vec![ThermalDeltaDailySummary {
    date,
    source: "Sensor A".to_owned(),
    coverage_minutes: 60,
    idle: band(4.0, 20),
    low: band(6.0, 20),
    mid: empty_band(),
    high: empty_band(),
  }];
  let covariates = CovariateDaySummary {
    bands: vec![CovariateDailySummary {
      date,
      source: "Sensor A".to_owned(),
      band: CpuLoadBand::Idle,
      sample_minutes: 20,
      band_share: 0.5,
      ambient_temperature_median: 21.0,
      delta_minutes: 20,
      delta_temperature_median: Some(4.0),
      power_minutes: 20,
      cpu_power_median: Some(8.0),
      delta_per_watt: fit(20),
    }],
    fans: vec![FanCovariateDailySummary {
      date,
      source: "Sensor A".to_owned(),
      fan_source: "Fan 1".to_owned(),
      band: CpuLoadBand::Idle,
      rpm_minutes: 20,
      rpm_median: 1200.0,
      delta_per_rpm: fit(20),
    }],
  };
  (summary, hours, fans, thermal_deltas, covariates)
}

fn baseline(window_start: NaiveDate, window_end: NaiveDate) -> EstablishedBaseline {
  EstablishedBaseline {
    idle_temperature_avg: 24.0,
    window_start_date: window_start,
    window_end_date: window_end,
    sample_minutes: 210,
  }
}

fn delta_baseline(
  window_start: NaiveDate,
  window_end: NaiveDate,
) -> EstablishedDeltaBaseline {
  EstablishedDeltaBaseline {
    source: "Sensor A".to_owned(),
    delta_temperature_avg: 4.0,
    window_start_date: window_start,
    window_end_date: window_end,
    sample_minutes: 140,
  }
}

#[tokio::test]
async fn dispatch_routes_cooling_rollup_and_baselines_to_the_selected_backend() {
  let fixture = NativeFixture::new();
  assert!(db::init(fixture.source.clone()));
  fixture.migrated_pool().await.close().await;

  let first_day = day("2026-09-01");
  let second_day = day("2026-09-02");
  let window_start = day("2026-08-25");
  let window_end = day("2026-08-31");

  // --- Not selected: the rollup write and the baseline pins land in
  // SQLite, and every cooling read answers from it. ---
  let (summary, hours, fans, thermal_deltas, covariates) = rollup_for(first_day);
  dispatch::cooling_rollup::persist_day_rollup(
    Some(&summary),
    &hours,
    &fans,
    &thermal_deltas,
    &covariates,
  )
  .await
  .unwrap();
  dispatch::cooling_baseline::insert_established_baseline(&baseline(
    window_start,
    window_end,
  ))
  .await
  .unwrap();
  dispatch::cooling_delta_baseline::insert_established_delta_baseline(&delta_baseline(
    window_start,
    window_end,
  ))
  .await
  .unwrap();

  let daily_before =
    dispatch::cooling_daily_summary::select_all_daily_cooling_summaries()
      .await
      .unwrap();
  assert_eq!(daily_before.len(), 1);
  assert_eq!(daily_before[0].date, first_day);
  let cursor_before = dispatch::cooling_daily_summary::max_summarized_date()
    .await
    .unwrap();
  assert_eq!(cursor_before, Some(first_day));
  let hourly_before =
    dispatch::cooling_hourly_summary::select_hours_in_date_range(first_day, first_day)
      .await
      .unwrap();
  assert_eq!(hourly_before.len(), 1);
  let fans_before = dispatch::cooling_fan_daily_summary::select_all_fan_daily_summaries()
    .await
    .unwrap();
  assert_eq!(fans_before.len(), 1);
  let thermal_before =
    dispatch::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries()
      .await
      .unwrap();
  assert_eq!(thermal_before.len(), 1);
  let covariate_before =
    dispatch::cooling_covariate_daily_summary::select_all_covariate_daily_summaries()
      .await
      .unwrap();
  assert_eq!(covariate_before.len(), 1);
  let fan_covariate_before =
    dispatch::cooling_covariate_daily_summary::select_all_fan_covariate_daily_summaries()
      .await
      .unwrap();
  assert_eq!(fan_covariate_before.len(), 1);
  let baseline_before = dispatch::cooling_baseline::select_established_baseline()
    .await
    .unwrap()
    .expect("the baseline was just pinned");
  assert_eq!(baseline_before.window_start_date, window_start);
  let delta_baseline_before =
    dispatch::cooling_delta_baseline::select_established_delta_baseline()
      .await
      .unwrap()
      .expect("the ΔT baseline was just pinned");
  assert_eq!(delta_baseline_before.source, "Sensor A");

  // --- Build, reconcile and select a native database from the same
  // source, then tell the boundary to adopt it. ---
  fixture.finalize().await;
  let (_, verified) = fixture.try_reconcile().await.unwrap();
  let paths = fixture.authority_paths();
  assert!(dispatch::init(
    paths.clone(),
    app_native_schema::NATIVE_SCHEMA_VERSION
  ));

  // --- The window while selection is being recorded: durably selected on
  // disk, but the boundary has not been told. Every cooling read must keep
  // answering from SQLite. ---
  select_native_database(paths, verified).await.unwrap();
  assert_eq!(fixture.authority_state(), AuthorityState::NativeSelected);
  let still_not_told =
    dispatch::cooling_daily_summary::select_all_daily_cooling_summaries()
      .await
      .unwrap();
  assert_eq!(still_not_told, daily_before);

  // --- Selected: every cooling read now answers what SQLite recorded, from
  // the native database. ---
  assert_eq!(
    dispatch::reobserve_authority().await.unwrap(),
    AuthorityState::NativeSelected
  );

  let daily_after = dispatch::cooling_daily_summary::select_all_daily_cooling_summaries()
    .await
    .unwrap();
  assert_eq!(daily_after, daily_before);
  assert_eq!(
    dispatch::cooling_daily_summary::max_summarized_date()
      .await
      .unwrap(),
    cursor_before
  );
  assert_eq!(
    dispatch::cooling_hourly_summary::select_hours_in_date_range(first_day, first_day)
      .await
      .unwrap(),
    hourly_before
  );
  assert_eq!(
    dispatch::cooling_fan_daily_summary::select_all_fan_daily_summaries()
      .await
      .unwrap(),
    fans_before
  );
  assert_eq!(
    dispatch::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries()
      .await
      .unwrap(),
    thermal_before
  );
  assert_eq!(
    dispatch::cooling_covariate_daily_summary::select_all_covariate_daily_summaries()
      .await
      .unwrap(),
    covariate_before
  );
  assert_eq!(
    dispatch::cooling_covariate_daily_summary::select_all_fan_covariate_daily_summaries()
      .await
      .unwrap(),
    fan_covariate_before
  );
  assert_eq!(
    dispatch::cooling_baseline::select_established_baseline()
      .await
      .unwrap(),
    Some(baseline_before)
  );
  assert_eq!(
    dispatch::cooling_delta_baseline::select_established_delta_baseline()
      .await
      .unwrap(),
    Some(delta_baseline_before)
  );

  // --- A second day's rollup, written through the boundary after
  // selection, lands in the native database as one transaction: the SQLite
  // tables must not grow, and every projection must show up together. ---
  let sqlite_pool = native_support::open_pool(&fixture.source, false).await;
  let sqlite_daily_count: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM cooling_daily_summary")
      .fetch_one(&sqlite_pool)
      .await
      .unwrap();
  assert_eq!(sqlite_daily_count, 1);

  let (summary2, hours2, fans2, thermal_deltas2, covariates2) = rollup_for(second_day);
  dispatch::cooling_rollup::persist_day_rollup(
    Some(&summary2),
    &hours2,
    &fans2,
    &thermal_deltas2,
    &covariates2,
  )
  .await
  .unwrap();

  let sqlite_daily_count_after: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM cooling_daily_summary")
      .fetch_one(&sqlite_pool)
      .await
      .unwrap();
  sqlite_pool.close().await;
  assert_eq!(
    sqlite_daily_count_after, 1,
    "a post-selection rollup write must land in the native database, not SQLite"
  );

  let daily_final = dispatch::cooling_daily_summary::select_all_daily_cooling_summaries()
    .await
    .unwrap();
  assert_eq!(
    daily_final.len(),
    2,
    "both days must be visible through dispatch"
  );
  let hourly_final =
    dispatch::cooling_hourly_summary::select_hours_in_date_range(first_day, second_day)
      .await
      .unwrap();
  assert_eq!(
    hourly_final.len(),
    2,
    "the second day's hour must be in the same transaction"
  );
  assert_eq!(
    dispatch::cooling_fan_daily_summary::select_all_fan_daily_summaries()
      .await
      .unwrap()
      .len(),
    2
  );
  assert_eq!(
    dispatch::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries()
      .await
      .unwrap()
      .len(),
    2
  );
  assert_eq!(
    dispatch::cooling_covariate_daily_summary::select_all_covariate_daily_summaries()
      .await
      .unwrap()
      .len(),
    2
  );
}
