//! `FAN_ARCHIVE` writes and retention (#2022).
//!
//! Row-per-fan rather than fixed columns: how many fans a machine exposes
//! is configuration-dependent. Only a fan that actually reported an
//! archivable reading for the interval gets a row, so an unreadable fan
//! stays absent instead of being recorded as 0 RPM - which is a real
//! Inactive Fan Reading, not a missing one.
//!
//! The bucketed read the timeline lanes use lives with the other archive
//! series queries in [`super::archive_queries`]; this module owns the
//! writer side and the rollup's own per-day range read.

use super::db;
use crate::persistence::archive_data::FanArchiveRow;
use crate::persistence::cooling_fan_rollup::FanArchiveMinuteSample;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

pub async fn insert(rows: Vec<FanArchiveRow>) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  insert_from_pool(&pool, rows).await
}

pub(crate) async fn insert_from_pool(
  pool: &SqlitePool,
  rows: Vec<FanArchiveRow>,
) -> Result<(), sqlx::Error> {
  // One transaction for the interval's fans: a partially written minute
  // would show some fans dropping out of the lane for a single bucket
  // while the others kept going, which reads as a sensor glitch that
  // never happened.
  let mut tx = pool.begin().await?;
  let timestamp = Utc::now();

  for row in rows {
    sqlx::query("INSERT INTO FAN_ARCHIVE (source, rpm, timestamp) VALUES ($1, $2, $3)")
      .bind(row.source)
      .bind(row.rpm as i64)
      .bind(timestamp)
      .execute(&mut *tx)
      .await?;
  }

  tx.commit().await
}

pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  delete_old_data_from_pool(&pool, retention_days).await
}

pub(crate) async fn delete_old_data_from_pool(
  pool: &SqlitePool,
  retention_days: u32,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM FAN_ARCHIVE WHERE timestamp < $1")
    .bind(Utc::now() - chrono::Duration::days(retention_days as i64))
    .execute(pool)
    .await?;

  Ok(())
}

/// Every archived fan reading in `[start, end)`, oldest first, for the
/// daily rollup's own pass.
pub async fn select_fan_minutes_for_range(
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
) -> Result<Vec<FanArchiveMinuteSample>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_fan_minutes_for_range_from_pool(&pool, start, end).await
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct FanArchiveMinuteRow {
  source: String,
  rpm: i64,
}

pub(crate) async fn select_fan_minutes_for_range_from_pool(
  pool: &SqlitePool,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
) -> Result<Vec<FanArchiveMinuteSample>, sqlx::Error> {
  // Same epoch-millisecond comparison the cooling daily rollup uses
  // against `DATA_ARCHIVE`, and for the same reason: the bind value's
  // TEXT shape is not guaranteed to sort against the written column at
  // exact-second boundaries.
  let epoch_ms = super::archive_queries::sqlite_epoch_milliseconds();
  let sql = format!(
    "SELECT source, CAST(rpm AS INTEGER) AS rpm
     FROM FAN_ARCHIVE
     WHERE {epoch_ms} >= $1 AND {epoch_ms} < $2
     ORDER BY timestamp ASC, id ASC"
  );
  let rows = sqlx::query_as::<_, FanArchiveMinuteRow>(&sql)
    .bind(start.timestamp_millis())
    .bind(end.timestamp_millis())
    .fetch_all(pool)
    .await?;

  Ok(
    rows
      .into_iter()
      .map(|row| FanArchiveMinuteSample {
        source: row.source,
        // The column is only ever written from a `u32`; clamp rather
        // than wrap if a hand-edited database ever carries a negative.
        rpm: row.rpm.max(0) as u32,
      })
      .collect(),
  )
}

/// The most recent archived fan timestamp strictly before `before`.
///
/// `before` is the start of today in local time, so the answer only ever
/// names a *completed* day - see
/// `cooling_fan_rollup::fan_rollup_is_behind` for why today's readings
/// must not count as evidence that the rollup fell behind.
pub async fn max_fan_archive_timestamp_before(
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_fan_archive_timestamp_before_from_pool(&pool, before).await
}

pub(crate) async fn max_fan_archive_timestamp_before_from_pool(
  pool: &SqlitePool,
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  let epoch_ms = super::archive_queries::sqlite_epoch_milliseconds();
  let sql = format!("SELECT MAX(timestamp) FROM FAN_ARCHIVE WHERE {epoch_ms} < $1");
  sqlx::query_scalar::<_, Option<DateTime<Utc>>>(&sql)
    .bind(before.timestamp_millis())
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  pub(crate) async fn setup_fan_archive(pool: &SqlitePool) {
    sqlx::query(
      "CREATE TABLE FAN_ARCHIVE (
        id INTEGER PRIMARY KEY,
        source TEXT NOT NULL,
        rpm INTEGER NOT NULL,
        timestamp DATETIME NOT NULL
      )",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  fn utc(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
      .unwrap()
      .with_timezone(&Utc)
  }

  async fn insert_row(pool: &SqlitePool, source: &str, rpm: i64, at: DateTime<Utc>) {
    sqlx::query("INSERT INTO FAN_ARCHIVE (source, rpm, timestamp) VALUES ($1, $2, $3)")
      .bind(source)
      .bind(rpm)
      .bind(at)
      .execute(pool)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn insert_writes_one_row_per_fan_sharing_a_timestamp() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;

    insert_from_pool(
      &pool,
      vec![
        FanArchiveRow {
          source: "Fan 1".to_string(),
          rpm: 900,
        },
        FanArchiveRow {
          source: "Fan 2".to_string(),
          rpm: 0,
        },
      ],
    )
    .await
    .unwrap();

    let rows: Vec<(String, i64, String)> =
      sqlx::query_as("SELECT source, rpm, timestamp FROM FAN_ARCHIVE ORDER BY source")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "Fan 1");
    assert_eq!(rows[0].1, 900);
    assert_eq!(rows[1].0, "Fan 2");
    assert_eq!(
      rows[1].1, 0,
      "an Inactive Fan Reading is stored as the real 0 RPM observation it is"
    );
    assert_eq!(
      rows[0].2, rows[1].2,
      "every fan in one interval shares the interval's timestamp"
    );
  }

  #[tokio::test]
  async fn select_fan_minutes_only_returns_rows_within_the_half_open_range() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    insert_row(&pool, "Fan 1", 100, utc("2026-08-14T23:59:59.999Z")).await;
    insert_row(&pool, "Fan 1", 200, utc("2026-08-15T00:00:00.000Z")).await;
    insert_row(&pool, "Fan 2", 300, utc("2026-08-15T23:59:59.999Z")).await;
    insert_row(&pool, "Fan 1", 400, utc("2026-08-16T00:00:00.000Z")).await;

    let rows = select_fan_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(
      rows,
      vec![
        FanArchiveMinuteSample {
          source: "Fan 1".to_string(),
          rpm: 200,
        },
        FanArchiveMinuteSample {
          source: "Fan 2".to_string(),
          rpm: 300,
        },
      ]
    );
  }

  #[tokio::test]
  async fn delete_old_data_removes_rows_before_the_cutoff() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    let now = Utc::now();
    insert_row(&pool, "Fan 1", 100, now - chrono::Duration::days(31)).await;
    insert_row(&pool, "Fan 1", 200, now - chrono::Duration::days(1)).await;

    delete_old_data_from_pool(&pool, 30).await.unwrap();

    let remaining: Vec<i64> = sqlx::query_scalar("SELECT rpm FROM FAN_ARCHIVE")
      .fetch_all(&pool)
      .await
      .unwrap();

    assert_eq!(remaining, vec![200]);
  }

  #[tokio::test]
  async fn max_fan_archive_timestamp_only_sees_rows_before_the_bound() {
    // The bound is the start of today: today's readings must not count, or
    // a machine recording fans right now would look permanently behind and
    // rewind the catch-up on every cycle.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    insert_row(&pool, "Fan 1", 100, utc("2026-08-19T23:59:59.000Z")).await;
    insert_row(&pool, "Fan 1", 200, utc("2026-08-20T00:00:00.000Z")).await;

    assert_eq!(
      max_fan_archive_timestamp_before_from_pool(&pool, &utc("2026-08-20T00:00:00.000Z"))
        .await
        .unwrap(),
      Some(utc("2026-08-19T23:59:59.000Z"))
    );
  }

  #[tokio::test]
  async fn max_fan_archive_timestamp_is_none_without_any_fan_source() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;

    assert_eq!(
      max_fan_archive_timestamp_before_from_pool(&pool, &utc("2026-08-20T00:00:00.000Z"))
        .await
        .unwrap(),
      None
    );
  }
}
