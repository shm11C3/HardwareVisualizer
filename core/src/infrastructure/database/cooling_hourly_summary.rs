//! `cooling_hourly_summary` reads and writes (#2023).
//!
//! Same `_from_pool` split as [`super::cooling_daily_summary`]: the public
//! `async fn`s resolve Core's process-wide pool via [`db::get_pool`] and
//! delegate to a variant taking an explicit `SqlitePool`, so the query
//! logic is testable against an in-memory database.
//!
//! `hour_start` is stored as the local wall-clock hour string
//! `cooling_hourly_rollup` formats, which sorts lexicographically as it
//! does chronologically and is prefix-compatible with
//! `cooling_daily_summary.date` - both the bounded range read below and
//! the retention delete compare it against plain `"%Y-%m-%d"` strings.

use super::db;
use crate::persistence::cooling_hourly_rollup::{
  HourlyCoolingSummary, format_hour_start, parse_hour_start,
};
use chrono::NaiveDate;
use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct HourlyCoolingSummaryRow {
  hour_start: String,
  cpu_usage_avg: Option<f64>,
  cpu_temperature_avg: Option<f64>,
  sample_minutes: i64,
}

impl HourlyCoolingSummaryRow {
  /// `None` for a row whose `hour_start` is not in the stored format. A
  /// hand-edited database drops that one row rather than failing the whole
  /// Explorer query.
  fn into_summary(self) -> Option<HourlyCoolingSummary> {
    Some(HourlyCoolingSummary {
      hour_start: parse_hour_start(&self.hour_start)?,
      cpu_usage_avg: self.cpu_usage_avg.map(|v| v as f32),
      cpu_temperature_avg: self.cpu_temperature_avg.map(|v| v as f32),
      // Same defensive clamp as `cooling_daily_summary`: the column is
      // `NOT NULL` and only ever written from a `u32`.
      sample_minutes: self.sample_minutes.max(0) as u32,
    })
  }
}

/// Every hourly row whose local day falls in `[start_date, end_date]`
/// (inclusive), oldest first.
///
/// Bounded rather than a whole-table read (unlike the daily rollup's
/// loader): this table holds 24x the rows for the same retention window,
/// and the Explorer only ever looks at two bounded windows.
pub async fn select_hours_in_date_range(
  start_date: NaiveDate,
  end_date: NaiveDate,
) -> Result<Vec<HourlyCoolingSummary>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_hours_in_date_range_from_pool(&pool, start_date, end_date).await
}

pub(crate) async fn select_hours_in_date_range_from_pool(
  pool: &SqlitePool,
  start_date: NaiveDate,
  end_date: NaiveDate,
) -> Result<Vec<HourlyCoolingSummary>, sqlx::Error> {
  // `>= "YYYY-MM-DD"` includes that day's 00:00 hour, and `<` the day
  // after `end_date` includes its 23:00 hour - the half-open upper bound
  // avoids needing a `"<= YYYY-MM-DD 23:00"` literal that would depend on
  // the hour format's exact width.
  let rows = sqlx::query_as::<_, HourlyCoolingSummaryRow>(
    "SELECT hour_start, cpu_usage_avg, cpu_temperature_avg, sample_minutes
     FROM cooling_hourly_summary
     WHERE hour_start >= $1 AND hour_start < $2
     ORDER BY hour_start ASC",
  )
  .bind(start_date.format("%Y-%m-%d").to_string())
  .bind(
    (end_date + chrono::Duration::days(1))
      .format("%Y-%m-%d")
      .to_string(),
  )
  .fetch_all(pool)
  .await?;

  Ok(
    rows
      .into_iter()
      .filter_map(HourlyCoolingSummaryRow::into_summary)
      .collect(),
  )
}

pub async fn upsert(summary: &HourlyCoolingSummary) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  upsert_from_pool(&pool, summary).await
}

pub(crate) async fn upsert_from_pool(
  pool: &SqlitePool,
  summary: &HourlyCoolingSummary,
) -> Result<(), sqlx::Error> {
  sqlx::query(
    "INSERT INTO cooling_hourly_summary
       (hour_start, cpu_usage_avg, cpu_temperature_avg, sample_minutes)
     VALUES ($1, $2, $3, $4)
     ON CONFLICT(hour_start) DO UPDATE SET
       cpu_usage_avg = excluded.cpu_usage_avg,
       cpu_temperature_avg = excluded.cpu_temperature_avg,
       sample_minutes = excluded.sample_minutes",
  )
  .bind(format_hour_start(summary.hour_start))
  .bind(summary.cpu_usage_avg)
  .bind(summary.cpu_temperature_avg)
  .bind(summary.sample_minutes as i64)
  .execute(pool)
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
  // Same local-date cutoff as `cooling_daily_summary::delete_old_data`, so
  // both tables age out on exactly the same boundary. Comparing an hour
  // key against a bare date string is correct because every hour of a day
  // sorts after that day's date string.
  let cutoff = (chrono::Local::now().date_naive()
    - chrono::Duration::days(retention_days as i64))
  .format("%Y-%m-%d")
  .to_string();

  sqlx::query("DELETE FROM cooling_hourly_summary WHERE hour_start < $1")
    .bind(cutoff)
    .execute(pool)
    .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::NaiveDateTime;

  async fn setup(pool: &SqlitePool) {
    sqlx::query(
      "CREATE TABLE cooling_hourly_summary (
        hour_start TEXT PRIMARY KEY,
        cpu_usage_avg REAL,
        cpu_temperature_avg REAL,
        sample_minutes INTEGER NOT NULL
      )",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  fn naive(input: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M").unwrap()
  }

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  fn hour(hour_start: &str, usage: f32, temperature: f32) -> HourlyCoolingSummary {
    HourlyCoolingSummary {
      hour_start: naive(hour_start),
      cpu_usage_avg: Some(usage),
      cpu_temperature_avg: Some(temperature),
      sample_minutes: 60,
    }
  }

  #[tokio::test]
  async fn an_upserted_hour_reads_back_unchanged() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    let summary = hour("2026-08-15 09:00", 42.5, 61.25);
    upsert_from_pool(&pool, &summary).await.unwrap();

    let rows =
      select_hours_in_date_range_from_pool(&pool, date(2026, 8, 15), date(2026, 8, 15))
        .await
        .unwrap();

    assert_eq!(rows, vec![summary]);
  }

  #[tokio::test]
  async fn re_running_the_rollup_for_the_same_hour_does_not_duplicate_the_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    upsert_from_pool(&pool, &hour("2026-08-15 09:00", 10.0, 40.0))
      .await
      .unwrap();
    upsert_from_pool(&pool, &hour("2026-08-15 09:00", 20.0, 50.0))
      .await
      .unwrap();

    let rows =
      select_hours_in_date_range_from_pool(&pool, date(2026, 8, 15), date(2026, 8, 15))
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cpu_usage_avg, Some(20.0));
    assert_eq!(rows[0].cpu_temperature_avg, Some(50.0));
  }

  #[tokio::test]
  async fn the_range_read_includes_both_boundary_days_in_full() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;
    for hour_start in [
      "2026-08-14 23:00",
      "2026-08-15 00:00",
      "2026-08-16 23:00",
      "2026-08-17 00:00",
    ] {
      upsert_from_pool(&pool, &hour(hour_start, 10.0, 40.0))
        .await
        .unwrap();
    }

    let rows =
      select_hours_in_date_range_from_pool(&pool, date(2026, 8, 15), date(2026, 8, 16))
        .await
        .unwrap();

    assert_eq!(
      rows.iter().map(|r| r.hour_start).collect::<Vec<_>>(),
      vec![naive("2026-08-15 00:00"), naive("2026-08-16 23:00")],
      "the first hour of the start day and the last hour of the end day must both be inside the window"
    );
  }

  #[tokio::test]
  async fn a_row_with_an_unreadable_hour_start_is_skipped_rather_than_failing_the_query()
  {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;
    upsert_from_pool(&pool, &hour("2026-08-15 09:00", 10.0, 40.0))
      .await
      .unwrap();
    sqlx::query(
      "INSERT INTO cooling_hourly_summary
         (hour_start, cpu_usage_avg, cpu_temperature_avg, sample_minutes)
       VALUES ('2026-08-15 nonsense', 1.0, 1.0, 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows =
      select_hours_in_date_range_from_pool(&pool, date(2026, 8, 15), date(2026, 8, 15))
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].hour_start, naive("2026-08-15 09:00"));
  }

  #[tokio::test]
  async fn delete_old_data_removes_every_hour_strictly_before_the_cutoff_day() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;
    let cutoff_date = chrono::Local::now().date_naive() - chrono::Duration::days(400);
    let just_inside = cutoff_date;
    let just_outside = cutoff_date - chrono::Duration::days(1);

    for day in [just_inside, just_outside] {
      for hour_of_day in [0, 23] {
        upsert_from_pool(
          &pool,
          &HourlyCoolingSummary {
            hour_start: day.and_hms_opt(hour_of_day, 0, 0).unwrap(),
            cpu_usage_avg: Some(10.0),
            cpu_temperature_avg: Some(40.0),
            sample_minutes: 60,
          },
        )
        .await
        .unwrap();
      }
    }

    delete_old_data_from_pool(&pool, 400).await.unwrap();

    let remaining: Vec<String> = sqlx::query_scalar(
      "SELECT hour_start FROM cooling_hourly_summary ORDER BY hour_start",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
      remaining,
      vec![
        format_hour_start(just_inside.and_hms_opt(0, 0, 0).unwrap()),
        format_hour_start(just_inside.and_hms_opt(23, 0, 0).unwrap()),
      ],
      "the whole boundary day must survive, including its first hour; only the older day is deleted"
    );
  }
}
