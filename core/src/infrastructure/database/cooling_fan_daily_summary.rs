//! `cooling_fan_daily_summary` reads and writes (#2022).
//!
//! Follows the `_from_pool` split the other cooling query modules use: the
//! public `async fn`s resolve Core's process-wide pool, and delegate to a
//! `_from_pool` variant tests can drive against an in-memory database.
//!
//! The table is keyed by `(date, source)` rather than by `date` alone: how
//! many fans a machine exposes is configuration-dependent, so each fan gets
//! its own row and a fan with no reading that day is simply absent.

use super::db;
use crate::persistence::cooling_fan_rollup::FanDailySummary;
use chrono::NaiveDate;
use sqlx::SqlitePool;

/// `MAX(date)` in `cooling_fan_daily_summary` - the fan projection's own
/// catch-up cursor. Paired with
/// `fan_archive::max_fan_archive_timestamp_before` this is how the rollup
/// detects the fan summaries are behind the archive (see
/// `cooling_rollup::fan_rollup_is_behind`).
pub async fn max_summarized_date() -> Result<Option<NaiveDate>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_summarized_date_from_pool(&pool).await
}

pub(crate) async fn max_summarized_date_from_pool(
  pool: &SqlitePool,
) -> Result<Option<NaiveDate>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<NaiveDate>>(
    "SELECT MAX(date) FROM cooling_fan_daily_summary",
  )
  .fetch_one(pool)
  .await
}

/// Every summarized fan-day, oldest first and grouped by fan within a day.
/// Reads the whole table for the same reason
/// `cooling_daily_summary::select_all_daily_cooling_summaries` does: it
/// holds at most one retention window of narrow rows per fan.
pub async fn select_all_fan_daily_summaries() -> Result<Vec<FanDailySummary>, sqlx::Error>
{
  let pool = db::get_pool().await?;
  select_all_fan_daily_summaries_from_pool(&pool).await
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct FanDailySummaryRow {
  date: NaiveDate,
  source: String,
  rpm_avg: f64,
  rpm_max: i64,
  rpm_min: i64,
  sample_minutes: i64,
}

impl From<FanDailySummaryRow> for FanDailySummary {
  fn from(row: FanDailySummaryRow) -> Self {
    Self {
      date: row.date,
      source: row.source,
      rpm_avg: row.rpm_avg as f32,
      // These columns are `NOT NULL` and only ever written from a `u32`;
      // clamp rather than wrap if a hand-edited database carries a
      // negative value.
      rpm_max: row.rpm_max.max(0) as u32,
      rpm_min: row.rpm_min.max(0) as u32,
      sample_minutes: row.sample_minutes.max(0) as u32,
    }
  }
}

pub(crate) async fn select_all_fan_daily_summaries_from_pool(
  pool: &SqlitePool,
) -> Result<Vec<FanDailySummary>, sqlx::Error> {
  // `date` is stored as "%Y-%m-%d", which sorts lexicographically the same
  // as chronologically (the same assumption `delete_old_data` makes).
  let rows = sqlx::query_as::<_, FanDailySummaryRow>(
    "SELECT date, source, rpm_avg, rpm_max, rpm_min, sample_minutes
     FROM cooling_fan_daily_summary
     ORDER BY date ASC, source ASC",
  )
  .fetch_all(pool)
  .await?;

  Ok(rows.into_iter().map(FanDailySummary::from).collect())
}

/// Upsert one fan-day against any executor, so a day's daily, hourly and
/// fan writes share one transaction (see
/// `cooling_rollup::persist_day_rollup_from_pool`).
pub(crate) async fn upsert_with<'e, E>(
  executor: E,
  summary: &FanDailySummary,
) -> Result<(), sqlx::Error>
where
  E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
  sqlx::query(
    "INSERT INTO cooling_fan_daily_summary
       (date, source, rpm_avg, rpm_max, rpm_min, sample_minutes)
     VALUES ($1, $2, $3, $4, $5, $6)
     ON CONFLICT(date, source) DO UPDATE SET
       rpm_avg = excluded.rpm_avg,
       rpm_max = excluded.rpm_max,
       rpm_min = excluded.rpm_min,
       sample_minutes = excluded.sample_minutes",
  )
  .bind(summary.date.format("%Y-%m-%d").to_string())
  .bind(&summary.source)
  .bind(summary.rpm_avg)
  .bind(summary.rpm_max as i64)
  .bind(summary.rpm_min as i64)
  .bind(summary.sample_minutes as i64)
  .execute(executor)
  .await?;

  Ok(())
}

pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  delete_old_data_from_pool(&pool, retention_days).await
}

pub(crate) async fn delete_old_data_from_pool(
  pool: &SqlitePool,
  retention_days: u32,
) -> Result<(), sqlx::Error> {
  let cutoff = (chrono::Local::now().date_naive()
    - chrono::Duration::days(retention_days as i64))
  .format("%Y-%m-%d")
  .to_string();

  sqlx::query("DELETE FROM cooling_fan_daily_summary WHERE date < $1")
    .bind(cutoff)
    .execute(pool)
    .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  pub(crate) async fn setup_cooling_fan_daily_summary(pool: &SqlitePool) {
    sqlx::query(
      "CREATE TABLE cooling_fan_daily_summary (
        date TEXT NOT NULL,
        source TEXT NOT NULL,
        rpm_avg REAL NOT NULL,
        rpm_max INTEGER NOT NULL,
        rpm_min INTEGER NOT NULL,
        sample_minutes INTEGER NOT NULL,
        PRIMARY KEY (date, source)
      )",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn summary(d: NaiveDate, source: &str, avg: f32) -> FanDailySummary {
    FanDailySummary {
      date: d,
      source: source.to_string(),
      rpm_avg: avg,
      rpm_max: avg as u32 + 100,
      rpm_min: avg as u32,
      sample_minutes: 600,
    }
  }

  #[tokio::test]
  async fn max_summarized_date_is_none_for_an_empty_table() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_fan_daily_summary(&pool).await;

    assert_eq!(max_summarized_date_from_pool(&pool).await.unwrap(), None);
  }

  #[tokio::test]
  async fn every_fan_of_a_day_round_trips_as_its_own_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_fan_daily_summary(&pool).await;

    for entry in [
      summary(date(2026, 8, 15), "Fan 2", 1500.0),
      summary(date(2026, 8, 15), "Fan 1", 900.0),
      summary(date(2026, 8, 10), "Fan 1", 800.0),
    ] {
      upsert_with(&pool, &entry).await.unwrap();
    }

    let rows = select_all_fan_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(
      rows
        .iter()
        .map(|row| (row.date, row.source.as_str()))
        .collect::<Vec<_>>(),
      vec![
        (date(2026, 8, 10), "Fan 1"),
        (date(2026, 8, 15), "Fan 1"),
        (date(2026, 8, 15), "Fan 2"),
      ]
    );
    assert_eq!(rows[1].rpm_avg, 900.0);
    assert_eq!(rows[1].rpm_max, 1000);
    assert_eq!(rows[1].rpm_min, 900);
    assert_eq!(rows[1].sample_minutes, 600);
    assert_eq!(
      max_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 15))
    );
  }

  #[tokio::test]
  async fn an_inactive_fan_day_round_trips_as_a_real_zero() {
    // 0 RPM is an Inactive Fan Reading, not a missing one, so it must
    // survive the round trip as a row rather than being indistinguishable
    // from a fan the day never recorded.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_fan_daily_summary(&pool).await;

    upsert_with(
      &pool,
      &FanDailySummary {
        date: date(2026, 8, 15),
        source: "Fan 3".to_string(),
        rpm_avg: 0.0,
        rpm_max: 0,
        rpm_min: 0,
        sample_minutes: 1440,
      },
    )
    .await
    .unwrap();

    let rows = select_all_fan_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rpm_avg, 0.0);
    assert_eq!(rows[0].sample_minutes, 1440);
  }

  #[tokio::test]
  async fn upsert_is_idempotent_per_date_and_source() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_fan_daily_summary(&pool).await;

    let mut entry = summary(date(2026, 8, 15), "Fan 1", 900.0);
    upsert_with(&pool, &entry).await.unwrap();
    entry.rpm_avg = 1200.0;
    entry.sample_minutes = 1440;
    upsert_with(&pool, &entry).await.unwrap();

    let rows = select_all_fan_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(rows.len(), 1, "re-running a day must not duplicate the row");
    assert_eq!(rows[0].rpm_avg, 1200.0);
    assert_eq!(rows[0].sample_minutes, 1440);
  }

  #[tokio::test]
  async fn delete_old_data_removes_rows_strictly_before_the_cutoff() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_fan_daily_summary(&pool).await;
    let today = chrono::Local::now().date_naive();
    let just_inside = today - chrono::Duration::days(400);
    let just_outside = just_inside - chrono::Duration::days(1);

    for d in [just_inside, just_outside] {
      upsert_with(&pool, &summary(d, "Fan 1", 900.0))
        .await
        .unwrap();
    }

    delete_old_data_from_pool(&pool, 400).await.unwrap();

    let rows = select_all_fan_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(
      rows.iter().map(|row| row.date).collect::<Vec<_>>(),
      vec![just_inside],
      "the row exactly at the retention boundary must survive"
    );
  }
}
