//! `cooling_delta_baseline` reads and writes: the single pinned row
//! holding the established ambient-normalized (ΔT) cooling baseline
//! (#2045) and the ambient source it was established from (#2062).
//!
//! Deliberately its own table rather than columns on `cooling_baseline` -
//! see [`crate::persistence::cooling_delta_baseline`] for why two
//! baselines that establish at different times cannot share one
//! write-once row.
//!
//! Follows the same `_from_pool` split as every other query module here.

use super::db;
use crate::persistence::cooling_delta_baseline::EstablishedDeltaBaseline;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

pub async fn select_established_delta_baseline()
-> Result<Option<EstablishedDeltaBaseline>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_established_delta_baseline_from_pool(&pool).await
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct DeltaBaselineRow {
  source: String,
  window_start_date: NaiveDate,
  window_end_date: NaiveDate,
  delta_temperature_avg: f64,
  sample_minutes: i64,
}

impl From<DeltaBaselineRow> for EstablishedDeltaBaseline {
  fn from(row: DeltaBaselineRow) -> Self {
    Self {
      source: row.source,
      delta_temperature_avg: row.delta_temperature_avg as f32,
      window_start_date: row.window_start_date,
      window_end_date: row.window_end_date,
      // The column is written from a `u32`; clamp rather than wrap if a
      // hand-edited database ever carries a negative count.
      sample_minutes: row.sample_minutes.max(0) as u32,
    }
  }
}

pub(crate) async fn select_established_delta_baseline_from_pool(
  pool: &SqlitePool,
) -> Result<Option<EstablishedDeltaBaseline>, sqlx::Error> {
  let row = sqlx::query_as::<_, DeltaBaselineRow>(
    "SELECT source, window_start_date, window_end_date, delta_temperature_avg, sample_minutes
     FROM cooling_delta_baseline
     WHERE id = 1",
  )
  .fetch_optional(pool)
  .await?;

  Ok(row.map(EstablishedDeltaBaseline::from))
}

pub(crate) async fn insert_established_delta_baseline_from_pool(
  pool: &SqlitePool,
  baseline: &EstablishedDeltaBaseline,
  established_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
  // `OR IGNORE` against the fixed `id = 1` makes establishment
  // write-once, exactly as it does for the absolute baseline: any later
  // read must never replace the pinned value, since replacing it is the
  // drift this table exists to prevent.
  sqlx::query(
    "INSERT OR IGNORE INTO cooling_delta_baseline
       (id, source, window_start_date, window_end_date, delta_temperature_avg, sample_minutes, established_at)
     VALUES (1, $1, $2, $3, $4, $5, $6)",
  )
  .bind(&baseline.source)
  .bind(baseline.window_start_date.format("%Y-%m-%d").to_string())
  .bind(baseline.window_end_date.format("%Y-%m-%d").to_string())
  .bind(baseline.delta_temperature_avg)
  .bind(baseline.sample_minutes as i64)
  .bind(established_at)
  .execute(pool)
  .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::test_schema::{
    COOLING_DELTA_BASELINE_DDL, create_tables,
  };

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn baseline() -> EstablishedDeltaBaseline {
    EstablishedDeltaBaseline {
      source: "Living Room".to_string(),
      delta_temperature_avg: 12.5,
      window_start_date: date(2026, 8, 1),
      window_end_date: date(2026, 8, 7),
      sample_minutes: 420,
    }
  }

  #[tokio::test]
  async fn an_unestablished_delta_baseline_reads_back_absent() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    create_tables(&pool, &[COOLING_DELTA_BASELINE_DDL]).await;

    assert_eq!(
      select_established_delta_baseline_from_pool(&pool)
        .await
        .unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn a_pinned_delta_baseline_round_trips_with_its_source() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    create_tables(&pool, &[COOLING_DELTA_BASELINE_DDL]).await;

    insert_established_delta_baseline_from_pool(&pool, &baseline(), Utc::now())
      .await
      .unwrap();

    assert_eq!(
      select_established_delta_baseline_from_pool(&pool)
        .await
        .unwrap(),
      Some(baseline())
    );
  }

  #[tokio::test]
  async fn a_second_establishment_cannot_replace_the_pinned_one() {
    // The invariant the whole table exists for. A later read that
    // re-derives a different value - or from a different sensor - must
    // not be able to overwrite the reference every delta is measured
    // against.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    create_tables(&pool, &[COOLING_DELTA_BASELINE_DDL]).await;
    insert_established_delta_baseline_from_pool(&pool, &baseline(), Utc::now())
      .await
      .unwrap();

    insert_established_delta_baseline_from_pool(
      &pool,
      &EstablishedDeltaBaseline {
        source: "Desk".to_string(),
        delta_temperature_avg: 40.0,
        window_start_date: date(2027, 1, 1),
        window_end_date: date(2027, 1, 7),
        sample_minutes: 999,
      },
      Utc::now(),
    )
    .await
    .unwrap();

    assert_eq!(
      select_established_delta_baseline_from_pool(&pool)
        .await
        .unwrap(),
      Some(baseline()),
      "the pinned ΔT baseline must survive a competing establishment"
    );
  }

  #[tokio::test]
  async fn the_delta_baseline_is_independent_of_the_absolute_one() {
    // Two tables, two write-once rows: pinning one says nothing about
    // the other, which is what lets them establish at different times.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    create_tables(
      &pool,
      &[
        COOLING_DELTA_BASELINE_DDL,
        crate::infrastructure::database::test_schema::COOLING_BASELINE_DDL,
      ],
    )
    .await;

    insert_established_delta_baseline_from_pool(&pool, &baseline(), Utc::now())
      .await
      .unwrap();

    assert!(
      crate::infrastructure::database::cooling_baseline::select_established_baseline_from_pool(&pool)
        .await
        .unwrap()
        .is_none(),
      "pinning the ΔT baseline must not look like pinning the absolute one"
    );
  }
}
