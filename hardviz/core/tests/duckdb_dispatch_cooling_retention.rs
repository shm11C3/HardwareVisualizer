#![cfg(feature = "duckdb-archive")]
//! The cooling rollup's retention cleanup through the dispatch boundary
//! (#2331): every ambient source's first qualifying ΔT days survive
//! `cooling_rollup::cleanup_old_data`, on SQLite and on the native database
//! alike.
//!
//! Companion to `duckdb_dispatch_cooling.rs`, which proves the cooling
//! family routes to the selected backend. This file proves the one cleanup
//! rule that family's reads depend on: only the first source to establish is
//! pinned, so a second source's reference is derived on read from its own
//! first qualifying days, and the cleanup must keep exactly those rows on
//! both engines while the rows around them age out. It lives in its own file
//! because the dispatch boundary is configured once per process.

mod native_support;

use chrono::{Duration, NaiveDate};
use hardviz_core::infrastructure::database::db;
use hardviz_core::infrastructure::database::dispatch;
use hardviz_core::infrastructure::database::native_database::{
  AuthorityState, select_native_database,
};
use hardviz_core::persistence::cooling_covariate_rollup::CovariateDaySummary;
use hardviz_core::persistence::cooling_delta_baseline::EstablishedDeltaBaseline;
use hardviz_core::persistence::cooling_rollup::{BandSummary, cleanup_old_data};
use hardviz_core::persistence::cooling_thermal_delta_rollup::ThermalDeltaDailySummary;
use native_support::{NativeFixture, app_native_schema};

/// `days` completed local days before today. The cleanup's cutoff is a
/// local date, so the fixture is laid out relative to it rather than on
/// fixed dates.
fn ago(days: i64) -> NaiveDate {
  chrono::Local::now().date_naive() - Duration::days(days)
}

/// A day on which `source` qualifies toward its own ΔT baseline.
fn qualifying(date: NaiveDate, source: &str, delta: f32) -> ThermalDeltaDailySummary {
  ThermalDeltaDailySummary {
    date,
    source: source.to_owned(),
    coverage_minutes: 900,
    idle: BandSummary {
      avg: Some(delta),
      max: Some(delta + 1.0),
      min: Some(delta - 1.0),
      sample_minutes: 600,
    },
    low: BandSummary::default(),
    mid: BandSummary::default(),
    high: BandSummary::default(),
  }
}

/// A day on which `source` had coverage but no idle ΔT at all.
fn non_qualifying(date: NaiveDate, source: &str) -> ThermalDeltaDailySummary {
  ThermalDeltaDailySummary {
    coverage_minutes: 1,
    idle: BandSummary::default(),
    ..qualifying(date, source, 0.0)
  }
}

/// Write `rows` as the ΔT projection of otherwise empty day rollups,
/// through whichever backend the boundary currently routes to.
async fn seed(rows: &[ThermalDeltaDailySummary]) {
  for row in rows {
    dispatch::cooling_rollup::persist_day_rollup(
      None,
      &[],
      &[],
      std::slice::from_ref(row),
      &CovariateDaySummary {
        bands: Vec::new(),
        fans: Vec::new(),
      },
    )
    .await
    .unwrap();
  }
}

async fn surviving_rows() -> Vec<ThermalDeltaDailySummary> {
  dispatch::cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries(
  )
  .await
  .unwrap()
}

#[tokio::test]
async fn cleanup_keeps_every_sources_first_qualifying_days_on_both_backends() {
  let fixture = NativeFixture::new();
  assert!(db::init(fixture.source.clone()));
  fixture.migrated_pool().await.close().await;

  // The pinned desk week, long past the 400-day cutoff.
  let desk_week: Vec<_> = (514..=520)
    .rev()
    .map(|n| qualifying(ago(n), "Desk", 12.0))
    .collect();
  // The living-room sensor the user switched to: two earlier days with
  // coverage but no idle ΔT, then its first seven qualifying days, then a
  // later qualifying day past the band-extension cap (#2333) that is not
  // part of its reference - all past the cutoff - and one recent day
  // inside retention. Each kept span runs from its first qualifying day to
  // 30 calendar days later (Desk ago(520)..ago(491), Living Room
  // ago(498)..ago(469)), so every deletable row sits outside both.
  let living_room_week: Vec<_> = (492..=498)
    .rev()
    .map(|n| qualifying(ago(n), "Living Room", 15.0))
    .collect();
  let deletable = vec![
    non_qualifying(ago(530), "Living Room"),
    non_qualifying(ago(529), "Living Room"),
    qualifying(ago(460), "Living Room", 15.5),
    non_qualifying(ago(455), "Hallway"),
  ];
  let recent = qualifying(ago(3), "Living Room", 16.0);
  let mut expected: Vec<_> = desk_week
    .iter()
    .chain(&living_room_week)
    .chain(std::iter::once(&recent))
    .cloned()
    .collect();
  expected.sort_by(|a, b| (a.date, &a.source).cmp(&(b.date, &b.source)));

  // --- Not selected: the cleanup runs against SQLite. ---
  seed(&desk_week).await;
  seed(&living_room_week).await;
  seed(&deletable).await;
  seed(std::slice::from_ref(&recent)).await;
  dispatch::cooling_delta_baseline::insert_established_delta_baseline(
    &EstablishedDeltaBaseline {
      source: "Desk".to_owned(),
      delta_temperature_avg: 12.0,
      window_start_date: ago(520),
      window_end_date: ago(514),
      sample_minutes: 4200,
    },
  )
  .await
  .unwrap();

  cleanup_old_data().await;

  assert_eq!(
    surviving_rows().await,
    expected,
    "SQLite: the pinned week, the second source's first seven qualifying days and the \
     recent row survive; its earlier non-qualifying days, its qualifying day past the \
     extension cap and the hallway row age out"
  );

  // --- Build, reconcile and select a native database from the cleaned
  // source, then tell the boundary to adopt it. ---
  fixture.finalize().await;
  let (_, verified) = fixture.try_reconcile().await.unwrap();
  let paths = fixture.authority_paths();
  assert!(dispatch::init(
    paths.clone(),
    app_native_schema::NATIVE_SCHEMA_VERSION
  ));
  select_native_database(paths, verified).await.unwrap();
  assert_eq!(
    dispatch::reobserve_authority().await.unwrap(),
    AuthorityState::NativeSelected
  );
  assert_eq!(
    surviving_rows().await,
    expected,
    "the native database starts from exactly the rows SQLite kept"
  );

  // --- Selected: the same deletable rows written natively must age out
  // the same way, and the same rows must survive. ---
  seed(&deletable).await;
  assert_eq!(
    surviving_rows().await.len(),
    expected.len() + deletable.len(),
    "the deletable rows were written to the native database"
  );

  cleanup_old_data().await;

  assert_eq!(
    surviving_rows().await,
    expected,
    "native: the cleanup keeps and removes exactly what SQLite did"
  );
}
