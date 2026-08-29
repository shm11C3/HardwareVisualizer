//! `cooling_baseline` reads and writes: the single pinned row holding the
//! established cooling baseline.
//!
//! The table exists so the baseline outlives the `cooling_daily_summary`
//! rows it was derived from, which are cleaned up under their own
//! retention window - see [`crate::persistence::cooling_baseline`] for
//! why re-deriving forever would drift.
//!
//! Follows the `_from_pool` split used by the other query modules: the
//! public `async fn`s resolve Core's process-wide pool via
//! [`db::get_pool`], and delegate to a `_from_pool` variant taking an
//! explicit `SqlitePool` so tests can exercise the query logic against an
//! in-memory database.

use super::db;
use crate::persistence::cooling_baseline::EstablishedBaseline;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

pub async fn select_established_baseline()
-> Result<Option<EstablishedBaseline>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_established_baseline_from_pool(&pool).await
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::FromRow)]
struct BaselineRow {
  window_start_date: NaiveDate,
  window_end_date: NaiveDate,
  idle_temperature_avg: f64,
  sample_minutes: i64,
}

impl From<BaselineRow> for EstablishedBaseline {
  fn from(row: BaselineRow) -> Self {
    Self {
      idle_temperature_avg: row.idle_temperature_avg as f32,
      window_start_date: row.window_start_date,
      window_end_date: row.window_end_date,
      // The column is written from a `u32`; clamp rather than wrap if a
      // hand-edited database ever carries a negative count.
      sample_minutes: row.sample_minutes.max(0) as u32,
    }
  }
}

pub(crate) async fn select_established_baseline_from_pool(
  pool: &SqlitePool,
) -> Result<Option<EstablishedBaseline>, sqlx::Error> {
  let row = sqlx::query_as::<_, BaselineRow>(
    "SELECT window_start_date, window_end_date, idle_temperature_avg, sample_minutes
     FROM cooling_baseline
     WHERE id = 1",
  )
  .fetch_optional(pool)
  .await?;

  Ok(row.map(EstablishedBaseline::from))
}

pub async fn insert_established_baseline(
  baseline: &EstablishedBaseline,
) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  insert_established_baseline_from_pool(&pool, baseline, Utc::now()).await
}

pub(crate) async fn insert_established_baseline_from_pool(
  pool: &SqlitePool,
  baseline: &EstablishedBaseline,
  established_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
  // `OR IGNORE` against the fixed `id = 1` is what makes establishment
  // write-once: a concurrent reader that derived the same establishment,
  // or any later read, must never replace the pinned value - replacing it
  // is exactly the drift this table exists to prevent.
  sqlx::query(
    "INSERT OR IGNORE INTO cooling_baseline
       (id, window_start_date, window_end_date, idle_temperature_avg, sample_minutes, established_at)
     VALUES (1, $1, $2, $3, $4, $5)",
  )
  .bind(baseline.window_start_date.format("%Y-%m-%d").to_string())
  .bind(baseline.window_end_date.format("%Y-%m-%d").to_string())
  .bind(baseline.idle_temperature_avg)
  .bind(baseline.sample_minutes as i64)
  .bind(established_at)
  .execute(pool)
  .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  async fn setup_cooling_baseline(pool: &SqlitePool) {
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

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn baseline(temperature: f32) -> EstablishedBaseline {
    EstablishedBaseline {
      idle_temperature_avg: temperature,
      window_start_date: date(2026, 8, 1),
      window_end_date: date(2026, 8, 7),
      sample_minutes: 210,
    }
  }

  #[tokio::test]
  async fn select_established_baseline_is_none_before_anything_is_pinned() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_baseline(&pool).await;

    assert_eq!(
      select_established_baseline_from_pool(&pool).await.unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn insert_then_select_round_trips_the_established_baseline() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_baseline(&pool).await;

    insert_established_baseline_from_pool(&pool, &baseline(42.5), Utc::now())
      .await
      .unwrap();

    assert_eq!(
      select_established_baseline_from_pool(&pool).await.unwrap(),
      Some(baseline(42.5))
    );
  }

  #[tokio::test]
  async fn a_second_insert_never_replaces_the_pinned_baseline() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_baseline(&pool).await;
    insert_established_baseline_from_pool(&pool, &baseline(42.5), Utc::now())
      .await
      .unwrap();

    // A later establishment - derived once the original rollup rows aged
    // out - must be ignored, not overwrite the reference.
    insert_established_baseline_from_pool(&pool, &baseline(70.0), Utc::now())
      .await
      .unwrap();

    assert_eq!(
      select_established_baseline_from_pool(&pool).await.unwrap(),
      Some(baseline(42.5))
    );
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM cooling_baseline")
      .fetch_one(&pool)
      .await
      .unwrap();
    assert_eq!(rows, 1, "the baseline table holds exactly one row");
  }
}
