//! `AMBIENT_ARCHIVE` writes (#2043).
//!
//! Row-per-source: one archive minute holds one row per ambient Sensor
//! Source Label that produced a fresh reading, and none at all for a
//! minute with no usable reading. Nothing here fills a gap.
//!
//! Same `_from_pool` split the cooling tables use: the public `async fn`s
//! resolve Core's process-wide pool via [`db::get_pool`] and delegate to a
//! variant taking an explicit `SqlitePool`, so the SQL is testable against
//! an in-memory database.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use super::db;
use crate::persistence::archive_data::AmbientData;

pub async fn insert(
  rows: Vec<AmbientData>,
  timestamp: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
  if rows.is_empty() {
    return Ok(());
  }

  let pool = db::get_pool().await?;
  insert_from_pool(&pool, rows, timestamp).await
}

/// Every row of one archive minute shares the tick's `timestamp` rather
/// than each reading's own observation time, so an ambient row lines up
/// with the `DATA_ARCHIVE` row for the same minute the way every other
/// archive table does. The freshness window is what bounds how far the
/// observation may trail the tick.
pub(crate) async fn insert_from_pool(
  pool: &SqlitePool,
  rows: Vec<AmbientData>,
  timestamp: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
  let mut tx = pool.begin().await?;

  for row in rows {
    sqlx::query(
      "INSERT INTO AMBIENT_ARCHIVE (source, temperature, humidity, timestamp)
       VALUES ($1, $2, $3, $4)",
    )
    .bind(&row.source)
    .bind(row.temperature)
    .bind(row.humidity)
    .bind(timestamp)
    .execute(&mut *tx)
    .await?;
  }

  tx.commit().await?;
  Ok(())
}

/// Ambient rows age out on the same `hardwareArchive.retentionDays`
/// window as the hardware archive rows they explain, so a retained minute
/// never loses its ambient context while the CPU side survives.
pub async fn delete_old_data(retention_days: u32) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  delete_old_data_from_pool(&pool, retention_days).await
}

pub(crate) async fn delete_old_data_from_pool(
  pool: &SqlitePool,
  retention_days: u32,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM AMBIENT_ARCHIVE WHERE timestamp < $1")
    .bind(Utc::now() - chrono::Duration::days(retention_days as i64))
    .execute(pool)
    .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  async fn setup(pool: &SqlitePool) {
    sqlx::query(
      "CREATE TABLE AMBIENT_ARCHIVE (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source TEXT NOT NULL,
        temperature REAL NOT NULL,
        humidity REAL,
        timestamp DATETIME NOT NULL
      )",
    )
    .execute(pool)
    .await
    .unwrap();
  }

  fn at(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
      .unwrap()
      .with_timezone(&Utc)
  }

  fn row(source: &str, temperature: f32, humidity: Option<f32>) -> AmbientData {
    AmbientData {
      source: source.to_string(),
      temperature,
      humidity,
    }
  }

  async fn stored(pool: &SqlitePool) -> Vec<(String, f64, Option<f64>, DateTime<Utc>)> {
    sqlx::query_as(
      "SELECT source, temperature, humidity, timestamp
       FROM AMBIENT_ARCHIVE ORDER BY source",
    )
    .fetch_all(pool)
    .await
    .unwrap()
  }

  #[tokio::test]
  async fn each_ambient_source_gets_its_own_row_for_the_same_minute() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;
    let tick = at("2026-08-30T12:00:00Z");

    insert_from_pool(
      &pool,
      vec![
        row("Living Room", 24.5, Some(48.0)),
        row("Desk", 26.0, None),
      ],
      tick,
    )
    .await
    .unwrap();

    assert_eq!(
      stored(&pool).await,
      vec![
        ("Desk".to_string(), 26.0, None, tick),
        ("Living Room".to_string(), 24.5, Some(48.0), tick),
      ]
    );
  }

  #[tokio::test]
  async fn a_minute_with_no_usable_reading_writes_no_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    insert_from_pool(&pool, vec![], at("2026-08-30T12:00:00Z"))
      .await
      .unwrap();

    assert!(stored(&pool).await.is_empty());
  }

  #[tokio::test]
  async fn retention_deletes_rows_past_the_window_and_keeps_the_rest() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;
    let now = Utc::now();

    insert_from_pool(
      &pool,
      vec![row("Old", 20.0, None)],
      now - chrono::Duration::days(31),
    )
    .await
    .unwrap();
    insert_from_pool(
      &pool,
      vec![row("Recent", 25.0, None)],
      now - chrono::Duration::days(2),
    )
    .await
    .unwrap();

    delete_old_data_from_pool(&pool, 30).await.unwrap();

    let remaining = stored(&pool).await;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].0, "Recent");
  }
}
