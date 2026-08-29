//! `cooling_daily_summary` reads and writes.
//!
//! Mirrors the `_from_pool` split used by `archive_queries`: the public
//! `async fn`s resolve Core's process-wide pool via [`db::get_pool`], and
//! delegate to a `_from_pool` variant that takes an explicit `SqlitePool`
//! so tests can exercise the query logic against an in-memory database
//! without touching the process-wide `db::init` `OnceLock`.

use super::archive_queries::sqlite_epoch_milliseconds;
use super::db;
use crate::persistence::cooling_baseline::DailyIdleSample;
use crate::persistence::cooling_rollup::{
  ArchiveMinuteSample, BandSummary, DailyCoolingSummary,
};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

pub async fn select_archive_minutes_for_range(
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
) -> Result<Vec<ArchiveMinuteSample>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_archive_minutes_for_range_from_pool(&pool, start, end).await
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::FromRow)]
struct ArchiveMinuteRow {
  cpu_avg: Option<f64>,
  cpu_temperature_avg: Option<f64>,
  cpu_temperature_max: Option<f64>,
  cpu_temperature_min: Option<f64>,
}

impl From<ArchiveMinuteRow> for ArchiveMinuteSample {
  fn from(row: ArchiveMinuteRow) -> Self {
    Self {
      cpu_usage_avg: row.cpu_avg.map(|v| v as f32),
      cpu_temperature_avg: row.cpu_temperature_avg.map(|v| v as f32),
      cpu_temperature_max: row.cpu_temperature_max.map(|v| v as f32),
      cpu_temperature_min: row.cpu_temperature_min.map(|v| v as f32),
    }
  }
}

async fn select_archive_minutes_for_range_from_pool(
  pool: &SqlitePool,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
) -> Result<Vec<ArchiveMinuteSample>, sqlx::Error> {
  // Compare via epoch milliseconds computed from `timestamp` in SQL,
  // rather than a raw TEXT comparison: `DATA_ARCHIVE.timestamp` is
  // written through sqlx's native `DateTime<Utc>` encoding (a `+00:00`
  // offset suffix, no fractional part when it is exactly zero), which is
  // not guaranteed to sort correctly against a differently-shaped bind
  // string at exact-second boundaries. `strftime` accepts every ISO 8601
  // shape chrono can produce, so this is robust regardless of writer.
  let epoch_ms = sqlite_epoch_milliseconds();
  let sql = format!(
    "SELECT
       CAST(cpu_avg AS REAL) AS cpu_avg,
       CAST(cpu_temperature_avg AS REAL) AS cpu_temperature_avg,
       CAST(cpu_temperature_max AS REAL) AS cpu_temperature_max,
       CAST(cpu_temperature_min AS REAL) AS cpu_temperature_min
     FROM DATA_ARCHIVE
     WHERE {epoch_ms} >= $1 AND {epoch_ms} < $2
     ORDER BY timestamp ASC"
  );
  let rows = sqlx::query_as::<_, ArchiveMinuteRow>(&sql)
    .bind(start.timestamp_millis())
    .bind(end.timestamp_millis())
    .fetch_all(pool)
    .await?;

  Ok(rows.into_iter().map(ArchiveMinuteSample::from).collect())
}

pub async fn max_summarized_date() -> Result<Option<NaiveDate>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_summarized_date_from_pool(&pool).await
}

async fn max_summarized_date_from_pool(
  pool: &SqlitePool,
) -> Result<Option<NaiveDate>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<NaiveDate>>(
    "SELECT MAX(date) FROM cooling_daily_summary",
  )
  .fetch_one(pool)
  .await
}

pub async fn earliest_archived_timestamp() -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  let pool = db::get_pool().await?;
  earliest_archived_timestamp_from_pool(&pool).await
}

async fn earliest_archived_timestamp_from_pool(
  pool: &SqlitePool,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  // Decoded through sqlx's own chrono codec (not a manual string parse)
  // so this round-trips correctly regardless of the exact TEXT shape
  // `hardware_archive::insert`'s native `DateTime<Utc>` bind produced.
  sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
    "SELECT MIN(timestamp) FROM DATA_ARCHIVE",
  )
  .fetch_one(pool)
  .await
}

/// Every summarized day's idle-band facts, oldest first, for the cooling
/// baseline derivation (see
/// [`crate::persistence::cooling_baseline`]). Reads the whole table:
/// it holds at most `COOLING_DAILY_SUMMARY_RETENTION_DAYS` rows of three
/// narrow columns, and the baseline is defined over the *first*
/// qualifying days, so there is no useful `LIMIT` to push into SQL.
pub async fn select_daily_idle_samples() -> Result<Vec<DailyIdleSample>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_daily_idle_samples_from_pool(&pool).await
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::FromRow)]
struct DailyIdleRow {
  date: NaiveDate,
  idle_cpu_temperature_avg: Option<f64>,
  idle_sample_minutes: i64,
}

impl From<DailyIdleRow> for DailyIdleSample {
  fn from(row: DailyIdleRow) -> Self {
    Self {
      date: row.date,
      idle_temperature_avg: row.idle_cpu_temperature_avg.map(|v| v as f32),
      // The column is `NOT NULL DEFAULT 0` and only ever written from a
      // `u32`; clamp rather than wrap if a hand-edited database ever
      // carries a negative count.
      idle_sample_minutes: row.idle_sample_minutes.max(0) as u32,
    }
  }
}

async fn select_daily_idle_samples_from_pool(
  pool: &SqlitePool,
) -> Result<Vec<DailyIdleSample>, sqlx::Error> {
  // `date` is stored as "%Y-%m-%d", which sorts lexicographically the
  // same as chronologically (same assumption as `delete_old_data`).
  let rows = sqlx::query_as::<_, DailyIdleRow>(
    "SELECT date, idle_cpu_temperature_avg, idle_sample_minutes
     FROM cooling_daily_summary
     ORDER BY date ASC",
  )
  .fetch_all(pool)
  .await?;

  Ok(rows.into_iter().map(DailyIdleSample::from).collect())
}

pub async fn upsert(summary: &DailyCoolingSummary) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  upsert_from_pool(&pool, summary).await
}

async fn upsert_from_pool(
  pool: &SqlitePool,
  summary: &DailyCoolingSummary,
) -> Result<(), sqlx::Error> {
  fn minutes(band: &BandSummary) -> i64 {
    band.sample_minutes as i64
  }

  sqlx::query(
    r#"
    INSERT INTO cooling_daily_summary (
      date,
      idle_cpu_temperature_avg, idle_cpu_temperature_max, idle_cpu_temperature_min, idle_sample_minutes,
      low_cpu_temperature_avg, low_cpu_temperature_max, low_cpu_temperature_min, low_sample_minutes,
      mid_cpu_temperature_avg, mid_cpu_temperature_max, mid_cpu_temperature_min, mid_sample_minutes,
      high_cpu_temperature_avg, high_cpu_temperature_max, high_cpu_temperature_min, high_sample_minutes,
      coverage_minutes
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
    ON CONFLICT(date) DO UPDATE SET
      idle_cpu_temperature_avg = excluded.idle_cpu_temperature_avg,
      idle_cpu_temperature_max = excluded.idle_cpu_temperature_max,
      idle_cpu_temperature_min = excluded.idle_cpu_temperature_min,
      idle_sample_minutes = excluded.idle_sample_minutes,
      low_cpu_temperature_avg = excluded.low_cpu_temperature_avg,
      low_cpu_temperature_max = excluded.low_cpu_temperature_max,
      low_cpu_temperature_min = excluded.low_cpu_temperature_min,
      low_sample_minutes = excluded.low_sample_minutes,
      mid_cpu_temperature_avg = excluded.mid_cpu_temperature_avg,
      mid_cpu_temperature_max = excluded.mid_cpu_temperature_max,
      mid_cpu_temperature_min = excluded.mid_cpu_temperature_min,
      mid_sample_minutes = excluded.mid_sample_minutes,
      high_cpu_temperature_avg = excluded.high_cpu_temperature_avg,
      high_cpu_temperature_max = excluded.high_cpu_temperature_max,
      high_cpu_temperature_min = excluded.high_cpu_temperature_min,
      high_sample_minutes = excluded.high_sample_minutes,
      coverage_minutes = excluded.coverage_minutes
    "#,
  )
  .bind(summary.date.format("%Y-%m-%d").to_string())
  .bind(summary.idle.avg)
  .bind(summary.idle.max)
  .bind(summary.idle.min)
  .bind(minutes(&summary.idle))
  .bind(summary.low.avg)
  .bind(summary.low.max)
  .bind(summary.low.min)
  .bind(minutes(&summary.low))
  .bind(summary.mid.avg)
  .bind(summary.mid.max)
  .bind(summary.mid.min)
  .bind(minutes(&summary.mid))
  .bind(summary.high.avg)
  .bind(summary.high.max)
  .bind(summary.high.min)
  .bind(minutes(&summary.high))
  .bind(summary.coverage_minutes as i64)
  .execute(pool)
  .await?;

  Ok(())
}

pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  delete_old_data_from_pool(&pool, retention_days).await
}

async fn delete_old_data_from_pool(
  pool: &SqlitePool,
  retention_days: u32,
) -> Result<(), sqlx::Error> {
  // Same cutoff style as `storage_health::delete_old_data`: a local-date
  // TEXT comparison, since `date` is stored as "%Y-%m-%d" and compares
  // lexicographically the same as chronologically.
  let cutoff = (chrono::Local::now().date_naive()
    - chrono::Duration::days(retention_days as i64))
  .format("%Y-%m-%d")
  .to_string();

  sqlx::query("DELETE FROM cooling_daily_summary WHERE date < $1")
    .bind(cutoff)
    .execute(pool)
    .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::Row;

  async fn setup_data_archive(pool: &SqlitePool) {
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_avg REAL,
        cpu_temperature_avg REAL,
        cpu_temperature_max REAL,
        cpu_temperature_min REAL,
        timestamp DATETIME
      )",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  async fn setup_cooling_daily_summary(pool: &SqlitePool) {
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
        coverage_minutes INTEGER NOT NULL
      )",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  async fn insert_archive_row(
    pool: &SqlitePool,
    cpu_avg: Option<f64>,
    cpu_temperature_avg: Option<f64>,
    timestamp: DateTime<Utc>,
  ) {
    // Bind the native `DateTime<Utc>` type (not a manually formatted
    // string) so this matches exactly how the real archive writer
    // (`hardware_archive::insert`) writes the column - the range query
    // under test must be able to compare against sqlx's own encoding,
    // not just a hand-formatted literal.
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (cpu_avg, cpu_temperature_avg, cpu_temperature_max, cpu_temperature_min, timestamp)
       VALUES ($1, $2, $2, $2, $3)",
    )
    .bind(cpu_avg)
    .bind(cpu_temperature_avg)
    .bind(timestamp)
    .execute(pool)
    .await
    .unwrap();
  }

  fn utc(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
      .unwrap()
      .with_timezone(&Utc)
  }

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  #[tokio::test]
  async fn select_archive_minutes_only_returns_rows_within_the_half_open_range() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-14T23:59:59.000Z"),
    )
    .await;
    insert_archive_row(
      &pool,
      Some(6.0),
      Some(41.0),
      utc("2026-08-15T00:00:00.000Z"),
    )
    .await;
    insert_archive_row(
      &pool,
      Some(7.0),
      Some(42.0),
      utc("2026-08-15T23:59:59.999Z"),
    )
    .await;
    insert_archive_row(
      &pool,
      Some(8.0),
      Some(43.0),
      utc("2026-08-16T00:00:00.000Z"),
    )
    .await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cpu_usage_avg, Some(6.0));
    assert_eq!(rows[1].cpu_usage_avg, Some(7.0));
  }

  #[tokio::test]
  async fn select_archive_minutes_preserves_null_temperature_and_usage() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_archive_row(&pool, None, None, utc("2026-08-15T12:00:00.000Z")).await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cpu_usage_avg, None);
    assert_eq!(rows[0].cpu_temperature_avg, None);
  }

  #[tokio::test]
  async fn max_summarized_date_is_none_for_an_empty_table() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    assert_eq!(max_summarized_date_from_pool(&pool).await.unwrap(), None);
  }

  #[tokio::test]
  async fn max_summarized_date_returns_the_latest_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    for d in ["2026-08-10", "2026-08-20", "2026-08-15"] {
      sqlx::query(
        "INSERT INTO cooling_daily_summary (date, coverage_minutes) VALUES ($1, 0)",
      )
      .bind(d)
      .execute(&pool)
      .await
      .unwrap();
    }

    assert_eq!(
      max_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 20))
    );
  }

  #[tokio::test]
  async fn earliest_archived_timestamp_is_none_for_an_empty_archive() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;

    assert_eq!(
      earliest_archived_timestamp_from_pool(&pool).await.unwrap(),
      None
    );
  }

  async fn insert_idle_summary_row(
    pool: &SqlitePool,
    date: &str,
    idle_temperature_avg: Option<f64>,
    idle_sample_minutes: i64,
  ) {
    sqlx::query(
      "INSERT INTO cooling_daily_summary
         (date, idle_cpu_temperature_avg, idle_sample_minutes, coverage_minutes)
       VALUES ($1, $2, $3, 1440)",
    )
    .bind(date)
    .bind(idle_temperature_avg)
    .bind(idle_sample_minutes)
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn select_daily_idle_samples_returns_rows_in_ascending_date_order() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    // Inserted out of order on purpose: the baseline derivation takes the
    // *first* qualifying days, so it depends on this ordering rather than
    // sorting the rows itself.
    insert_idle_summary_row(&pool, "2026-08-20", Some(42.0), 120).await;
    insert_idle_summary_row(&pool, "2026-08-10", Some(40.0), 60).await;
    insert_idle_summary_row(&pool, "2026-08-15", Some(41.0), 90).await;

    let samples = select_daily_idle_samples_from_pool(&pool).await.unwrap();

    assert_eq!(
      samples.iter().map(|s| s.date).collect::<Vec<_>>(),
      vec![date(2026, 8, 10), date(2026, 8, 15), date(2026, 8, 20)]
    );
    assert_eq!(samples[0].idle_temperature_avg, Some(40.0));
    assert_eq!(samples[0].idle_sample_minutes, 60);
  }

  #[tokio::test]
  async fn select_daily_idle_samples_preserves_a_null_idle_temperature() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    // A day the machine ran but never spent in the idle band: the band
    // stays absent, never zero degrees.
    insert_idle_summary_row(&pool, "2026-08-15", None, 0).await;

    let samples = select_daily_idle_samples_from_pool(&pool).await.unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].idle_temperature_avg, None);
    assert_eq!(samples[0].idle_sample_minutes, 0);
  }

  #[tokio::test]
  async fn select_daily_idle_samples_is_empty_for_an_empty_rollup() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    assert_eq!(
      select_daily_idle_samples_from_pool(&pool).await.unwrap(),
      Vec::new()
    );
  }

  #[tokio::test]
  async fn earliest_archived_timestamp_returns_the_oldest_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_archive_row(&pool, Some(1.0), Some(1.0), utc("2026-08-15T00:00:00.000Z"))
      .await;
    insert_archive_row(&pool, Some(1.0), Some(1.0), utc("2026-08-01T00:00:00.000Z"))
      .await;
    insert_archive_row(&pool, Some(1.0), Some(1.0), utc("2026-08-10T00:00:00.000Z"))
      .await;

    assert_eq!(
      earliest_archived_timestamp_from_pool(&pool).await.unwrap(),
      Some(utc("2026-08-01T00:00:00.000Z"))
    );
  }

  fn full_band(value: f32, minutes: u32) -> BandSummary {
    BandSummary {
      avg: Some(value),
      max: Some(value + 1.0),
      min: Some(value - 1.0),
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

  #[tokio::test]
  async fn upsert_writes_all_four_bands_and_coverage() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    let summary = DailyCoolingSummary {
      date: date(2026, 8, 15),
      coverage_minutes: 1000,
      idle: full_band(30.0, 600),
      low: full_band(40.0, 300),
      mid: empty_band(),
      high: full_band(70.0, 100),
    };
    upsert_from_pool(&pool, &summary).await.unwrap();

    let row = sqlx::query(
      "SELECT idle_cpu_temperature_avg, idle_sample_minutes, mid_cpu_temperature_avg, mid_sample_minutes, coverage_minutes
       FROM cooling_daily_summary WHERE date = $1",
    )
    .bind("2026-08-15")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<f64, _>("idle_cpu_temperature_avg"), 30.0);
    assert_eq!(row.get::<i64, _>("idle_sample_minutes"), 600);
    assert_eq!(row.get::<Option<f64>, _>("mid_cpu_temperature_avg"), None);
    assert_eq!(row.get::<i64, _>("mid_sample_minutes"), 0);
    assert_eq!(row.get::<i64, _>("coverage_minutes"), 1000);
  }

  #[tokio::test]
  async fn upsert_is_idempotent_for_the_same_date() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    let mut summary = DailyCoolingSummary {
      date: date(2026, 8, 15),
      coverage_minutes: 500,
      idle: full_band(30.0, 500),
      low: empty_band(),
      mid: empty_band(),
      high: empty_band(),
    };
    upsert_from_pool(&pool, &summary).await.unwrap();

    summary.coverage_minutes = 1440;
    summary.idle.sample_minutes = 1440;
    upsert_from_pool(&pool, &summary).await.unwrap();

    let count: (i64,) =
      sqlx::query_as("SELECT COUNT(1) FROM cooling_daily_summary WHERE date = $1")
        .bind("2026-08-15")
        .fetch_one(&pool)
        .await
        .unwrap();
    let coverage: (i64,) = sqlx::query_as(
      "SELECT coverage_minutes FROM cooling_daily_summary WHERE date = $1",
    )
    .bind("2026-08-15")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
      count.0, 1,
      "re-running a rollup for the same day must not duplicate the row"
    );
    assert_eq!(coverage.0, 1440);
  }

  #[tokio::test]
  async fn delete_old_data_removes_rows_strictly_before_the_cutoff() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    let today = chrono::Local::now().date_naive();
    let cutoff_date = today - chrono::Duration::days(400);
    let just_inside = cutoff_date.format("%Y-%m-%d").to_string();
    let just_outside = (cutoff_date - chrono::Duration::days(1))
      .format("%Y-%m-%d")
      .to_string();

    for d in [&just_inside, &just_outside] {
      sqlx::query(
        "INSERT INTO cooling_daily_summary (date, coverage_minutes) VALUES ($1, 0)",
      )
      .bind(d)
      .execute(&pool)
      .await
      .unwrap();
    }

    delete_old_data_from_pool(&pool, 400).await.unwrap();

    let remaining: Vec<String> =
      sqlx::query_scalar("SELECT date FROM cooling_daily_summary")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(
      remaining,
      vec![just_inside],
      "the row exactly at the retention boundary must survive; only the older row is deleted"
    );
  }
}
