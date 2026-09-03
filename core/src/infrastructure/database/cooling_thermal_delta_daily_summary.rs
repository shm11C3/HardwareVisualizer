//! `cooling_thermal_delta_daily_summary` reads and writes, plus the
//! paired-minute read the rollup folds it from (#2045, #2062).
//!
//! Follows the `_from_pool` split the other cooling query modules use: the
//! public `async fn`s resolve Core's process-wide pool, and delegate to a
//! `_from_pool` variant tests can drive against an in-memory database.
//!
//! The table is keyed by `(date, source)` rather than by `date` alone,
//! like `cooling_fan_daily_summary`: `AMBIENT_ARCHIVE` is row-per-source,
//! and which sensor a Thermal Delta was measured against is part of the
//! measurement, so each ambient Sensor Source Label gets its own row and a
//! source with no paired minute that day is simply absent.

use super::archive_queries::sqlite_epoch_milliseconds_of;
use super::cooling_daily_summary::{
  preserving_delete_sql, raw_timestamp_bound, sqlite_minute_key,
};
use super::db;
use crate::persistence::cooling_rollup::BandSummary;
use crate::persistence::cooling_thermal_delta_rollup::{
  ThermalDeltaDailySummary, ThermalDeltaMinuteSample,
};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

/// Every `(archived minute, ambient source)` pair inside `[start, end)`,
/// oldest first and grouped by source within a minute.
pub async fn select_thermal_delta_minutes_for_range(
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
) -> Result<Vec<ThermalDeltaMinuteSample>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_thermal_delta_minutes_for_range_from_pool(&pool, start, end).await
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct ThermalDeltaMinuteRow {
  timestamp: DateTime<Utc>,
  source: String,
  ambient_temperature: f64,
  cpu_avg: Option<f64>,
  cpu_temperature_avg: Option<f64>,
  cpu_temperature_max: Option<f64>,
  cpu_temperature_min: Option<f64>,
  cpu_power_avg: Option<f64>,
}

impl From<ThermalDeltaMinuteRow> for ThermalDeltaMinuteSample {
  fn from(row: ThermalDeltaMinuteRow) -> Self {
    Self {
      timestamp: row.timestamp,
      source: row.source,
      ambient_temperature: row.ambient_temperature as f32,
      cpu_usage_avg: row.cpu_avg.map(|v| v as f32),
      cpu_temperature_avg: row.cpu_temperature_avg.map(|v| v as f32),
      cpu_temperature_max: row.cpu_temperature_max.map(|v| v as f32),
      cpu_temperature_min: row.cpu_temperature_min.map(|v| v as f32),
      cpu_power_avg: row.cpu_power_avg.map(|v| v as f32),
    }
  }
}

pub(crate) async fn select_thermal_delta_minutes_for_range_from_pool(
  pool: &SqlitePool,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
) -> Result<Vec<ThermalDeltaMinuteSample>, sqlx::Error> {
  // Same range predicate as `cooling_daily_summary::select_archive_minutes_for_range`:
  // the exact epoch-millisecond comparison decides membership, and the
  // widened raw TEXT bound beside it only lets SQLite range-scan each
  // side's timestamp index instead of reading the whole table.
  let epoch_ms = sqlite_epoch_milliseconds_of("DATA_ARCHIVE.timestamp");
  let ambient_epoch_ms = sqlite_epoch_milliseconds_of("AMBIENT_ARCHIVE.timestamp");
  let minute_key = sqlite_minute_key(&epoch_ms);
  let ambient_minute_key = sqlite_minute_key(&ambient_epoch_ms);
  // The ambient side is collapsed to one value per *(minute, source)* and
  // then INNER JOINed on the minute, which is what makes #2045's "pair
  // samples first, aggregate second" rule structural: every row this
  // returns is one archived minute beside one sensor's reading for that
  // same minute, so no later step can subtract two summaries built over
  // different sample sets - or, since #2062, over different sensors. A
  // minute with no ambient row for a source yields no row for that
  // source, never an interpolated one.
  //
  // The per-source `AVG` is a formality: the archive writer refuses a
  // second row for a label it already wrote this minute, so the group is
  // one row wide unless the database was edited by hand. What it must
  // never do is average *across* sources, which the `GROUP BY` forbids.
  let sql = format!(
    "SELECT
       DATA_ARCHIVE.timestamp AS timestamp,
       ambient.source AS source,
       ambient.ambient_temperature AS ambient_temperature,
       CAST(DATA_ARCHIVE.cpu_avg AS REAL) AS cpu_avg,
       CAST(DATA_ARCHIVE.cpu_temperature_avg AS REAL) AS cpu_temperature_avg,
       CAST(DATA_ARCHIVE.cpu_temperature_max AS REAL) AS cpu_temperature_max,
       CAST(DATA_ARCHIVE.cpu_temperature_min AS REAL) AS cpu_temperature_min,
       CAST(DATA_ARCHIVE.cpu_power_avg AS REAL) AS cpu_power_avg
     FROM DATA_ARCHIVE
     JOIN (
       SELECT {ambient_minute_key} AS ambient_minute_key,
              AMBIENT_ARCHIVE.source AS source,
              AVG(CAST(AMBIENT_ARCHIVE.temperature AS REAL)) AS ambient_temperature
       FROM AMBIENT_ARCHIVE
       WHERE AMBIENT_ARCHIVE.timestamp >= $3 AND AMBIENT_ARCHIVE.timestamp < $4
         AND {ambient_epoch_ms} >= $1 AND {ambient_epoch_ms} < $2
       GROUP BY ambient_minute_key, AMBIENT_ARCHIVE.source
     ) AS ambient ON ambient.ambient_minute_key = {minute_key}
     WHERE DATA_ARCHIVE.timestamp >= $3 AND DATA_ARCHIVE.timestamp < $4
       AND {epoch_ms} >= $1 AND {epoch_ms} < $2
     ORDER BY DATA_ARCHIVE.timestamp ASC, ambient.source ASC"
  );
  let rows = sqlx::query_as::<_, ThermalDeltaMinuteRow>(&sql)
    .bind(start.timestamp_millis())
    .bind(end.timestamp_millis())
    .bind(raw_timestamp_bound(start, -1))
    .bind(raw_timestamp_bound(end, 1))
    .fetch_all(pool)
    .await?;

  Ok(
    rows
      .into_iter()
      .map(ThermalDeltaMinuteSample::from)
      .collect(),
  )
}

/// `MAX(date)` in `cooling_thermal_delta_daily_summary` - the Thermal
/// Delta projection's own catch-up cursor. A row exists only for a
/// `(day, source)` that paired at least one minute, so this is exactly
/// the latest day that recorded any ambient coverage. Paired with
/// [`max_pairable_ambient_archive_timestamp_before`] this is how the
/// rollup detects the projection is behind the archives (see
/// `cooling_rollup::ambient_rollup_is_behind`).
pub async fn max_summarized_date() -> Result<Option<NaiveDate>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_summarized_date_from_pool(&pool).await
}

pub(crate) async fn max_summarized_date_from_pool(
  pool: &SqlitePool,
) -> Result<Option<NaiveDate>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<NaiveDate>>(
    "SELECT MAX(date) FROM cooling_thermal_delta_daily_summary",
  )
  .fetch_one(pool)
  .await
}

/// The most recent ambient archive timestamp strictly before `before`
/// whose minute also has a `DATA_ARCHIVE` row (#2045).
///
/// Both bounds matter, and both mirror
/// `cooling_daily_summary::max_powered_archive_timestamp_before`:
///
/// `before` is the start of today in local time, so the answer only ever
/// names a *completed* day. The rollup never summarizes today, so today's
/// ambient rows are not evidence that a day was missed - counting them
/// would make a machine that is recording ambient right now rewind the
/// catch-up on every cycle, forever.
///
/// The `EXISTS` clause matches the rollup's own coverage gate: a row is
/// written per *archived minute* that had an ambient pair, so an ambient
/// row whose minute has no hardware row can never become coverage however
/// often the day is re-rolled. Counting it would send the catch-up chasing
/// a day it can never fill.
pub async fn max_pairable_ambient_archive_timestamp_before(
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_pairable_ambient_archive_timestamp_before_from_pool(&pool, before).await
}

pub(crate) async fn max_pairable_ambient_archive_timestamp_before_from_pool(
  pool: &SqlitePool,
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  let ambient_epoch_ms = sqlite_epoch_milliseconds_of("AMBIENT_ARCHIVE.timestamp");
  let hardware_epoch_ms = sqlite_epoch_milliseconds_of("DATA_ARCHIVE.timestamp");
  let ambient_minute_key = sqlite_minute_key(&ambient_epoch_ms);
  let hardware_minute_key = sqlite_minute_key(&hardware_epoch_ms);
  // The `EXISTS` correlates on a computed minute key, which no index can
  // serve: on its own it re-scans all of `DATA_ARCHIVE` for every ambient
  // row. The raw bracket beside it is what lets SQLite range-scan the
  // timestamp index instead, and it is correlated to the ambient row's
  // own timestamp so the scan covers minutes rather than years.
  //
  // Two minutes of slack, for the same reason `raw_timestamp_bound` uses
  // a day: a genuine match is inside the same minute, so the bracket has
  // room to spare, and the minute-key equality remains what actually
  // decides. `strftime` normalizes whatever ISO 8601 shape the ambient
  // row carries into the same `%Y-%m-%dT%H:%M:%S` prefix the hardware
  // rows compare under.
  let sql = format!(
    "SELECT MAX(AMBIENT_ARCHIVE.timestamp) FROM AMBIENT_ARCHIVE
     WHERE {ambient_epoch_ms} < $1
       AND EXISTS (
         SELECT 1 FROM DATA_ARCHIVE
         WHERE DATA_ARCHIVE.timestamp >= strftime(
                 '%Y-%m-%dT%H:%M:%S', AMBIENT_ARCHIVE.timestamp, '-2 minutes')
           AND DATA_ARCHIVE.timestamp <= strftime(
                 '%Y-%m-%dT%H:%M:%S', AMBIENT_ARCHIVE.timestamp, '+2 minutes')
           AND {hardware_minute_key} = {ambient_minute_key}
       )"
  );
  sqlx::query_scalar::<_, Option<DateTime<Utc>>>(&sql)
    .bind(before.timestamp_millis())
    .fetch_one(pool)
    .await
}

/// Every summarized source-day, oldest first and grouped by source within
/// a day. Reads the whole table for the same reason
/// `cooling_daily_summary::select_all_daily_cooling_summaries` does: it
/// holds at most one retention window of narrow rows per source, and the
/// baseline is defined over the *first* qualifying days, so there is no
/// useful `LIMIT` to push into SQL.
pub async fn select_all_thermal_delta_daily_summaries()
-> Result<Vec<ThermalDeltaDailySummary>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_all_thermal_delta_daily_summaries_from_pool(&pool).await
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct ThermalDeltaDailySummaryRow {
  date: NaiveDate,
  source: String,
  coverage_minutes: i64,
  idle_delta_temperature_avg: Option<f64>,
  idle_delta_temperature_max: Option<f64>,
  idle_delta_temperature_min: Option<f64>,
  idle_delta_sample_minutes: i64,
  low_delta_temperature_avg: Option<f64>,
  low_delta_temperature_max: Option<f64>,
  low_delta_temperature_min: Option<f64>,
  low_delta_sample_minutes: i64,
  mid_delta_temperature_avg: Option<f64>,
  mid_delta_temperature_max: Option<f64>,
  mid_delta_temperature_min: Option<f64>,
  mid_delta_sample_minutes: i64,
  high_delta_temperature_avg: Option<f64>,
  high_delta_temperature_max: Option<f64>,
  high_delta_temperature_min: Option<f64>,
  high_delta_sample_minutes: i64,
}

impl From<ThermalDeltaDailySummaryRow> for ThermalDeltaDailySummary {
  fn from(row: ThermalDeltaDailySummaryRow) -> Self {
    fn band(
      avg: Option<f64>,
      max: Option<f64>,
      min: Option<f64>,
      minutes: i64,
    ) -> BandSummary {
      BandSummary {
        avg: avg.map(|v| v as f32),
        max: max.map(|v| v as f32),
        min: min.map(|v| v as f32),
        // The column is `NOT NULL DEFAULT 0` and only ever written from
        // a `u32`; clamp rather than wrap if a hand-edited database ever
        // carries a negative count.
        sample_minutes: minutes.max(0) as u32,
      }
    }

    Self {
      date: row.date,
      source: row.source,
      coverage_minutes: row.coverage_minutes.max(0) as u32,
      idle: band(
        row.idle_delta_temperature_avg,
        row.idle_delta_temperature_max,
        row.idle_delta_temperature_min,
        row.idle_delta_sample_minutes,
      ),
      low: band(
        row.low_delta_temperature_avg,
        row.low_delta_temperature_max,
        row.low_delta_temperature_min,
        row.low_delta_sample_minutes,
      ),
      mid: band(
        row.mid_delta_temperature_avg,
        row.mid_delta_temperature_max,
        row.mid_delta_temperature_min,
        row.mid_delta_sample_minutes,
      ),
      high: band(
        row.high_delta_temperature_avg,
        row.high_delta_temperature_max,
        row.high_delta_temperature_min,
        row.high_delta_sample_minutes,
      ),
    }
  }
}

pub(crate) async fn select_all_thermal_delta_daily_summaries_from_pool(
  pool: &SqlitePool,
) -> Result<Vec<ThermalDeltaDailySummary>, sqlx::Error> {
  // `date` is stored as "%Y-%m-%d", which sorts lexicographically the same
  // as chronologically (the same assumption `delete_old_data` makes).
  let rows = sqlx::query_as::<_, ThermalDeltaDailySummaryRow>(
    "SELECT date, source, coverage_minutes,
       idle_delta_temperature_avg, idle_delta_temperature_max, idle_delta_temperature_min, idle_delta_sample_minutes,
       low_delta_temperature_avg, low_delta_temperature_max, low_delta_temperature_min, low_delta_sample_minutes,
       mid_delta_temperature_avg, mid_delta_temperature_max, mid_delta_temperature_min, mid_delta_sample_minutes,
       high_delta_temperature_avg, high_delta_temperature_max, high_delta_temperature_min, high_delta_sample_minutes
     FROM cooling_thermal_delta_daily_summary
     ORDER BY date ASC, source ASC",
  )
  .fetch_all(pool)
  .await?;

  Ok(
    rows
      .into_iter()
      .map(ThermalDeltaDailySummary::from)
      .collect(),
  )
}

/// Upsert one source-day against any executor, so a day's daily, hourly,
/// fan and Thermal Delta writes share one transaction (see
/// `cooling_rollup::persist_day_rollup_from_pool`).
pub(crate) async fn upsert_with<'e, E>(
  executor: E,
  summary: &ThermalDeltaDailySummary,
) -> Result<(), sqlx::Error>
where
  E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
  fn minutes(band: &BandSummary) -> i64 {
    band.sample_minutes as i64
  }

  sqlx::query(
    "INSERT INTO cooling_thermal_delta_daily_summary (
       date, source, coverage_minutes,
       idle_delta_temperature_avg, idle_delta_temperature_max, idle_delta_temperature_min, idle_delta_sample_minutes,
       low_delta_temperature_avg, low_delta_temperature_max, low_delta_temperature_min, low_delta_sample_minutes,
       mid_delta_temperature_avg, mid_delta_temperature_max, mid_delta_temperature_min, mid_delta_sample_minutes,
       high_delta_temperature_avg, high_delta_temperature_max, high_delta_temperature_min, high_delta_sample_minutes
     )
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
     ON CONFLICT(date, source) DO UPDATE SET
       coverage_minutes = excluded.coverage_minutes,
       idle_delta_temperature_avg = excluded.idle_delta_temperature_avg,
       idle_delta_temperature_max = excluded.idle_delta_temperature_max,
       idle_delta_temperature_min = excluded.idle_delta_temperature_min,
       idle_delta_sample_minutes = excluded.idle_delta_sample_minutes,
       low_delta_temperature_avg = excluded.low_delta_temperature_avg,
       low_delta_temperature_max = excluded.low_delta_temperature_max,
       low_delta_temperature_min = excluded.low_delta_temperature_min,
       low_delta_sample_minutes = excluded.low_delta_sample_minutes,
       mid_delta_temperature_avg = excluded.mid_delta_temperature_avg,
       mid_delta_temperature_max = excluded.mid_delta_temperature_max,
       mid_delta_temperature_min = excluded.mid_delta_temperature_min,
       mid_delta_sample_minutes = excluded.mid_delta_sample_minutes,
       high_delta_temperature_avg = excluded.high_delta_temperature_avg,
       high_delta_temperature_max = excluded.high_delta_temperature_max,
       high_delta_temperature_min = excluded.high_delta_temperature_min,
       high_delta_sample_minutes = excluded.high_delta_sample_minutes",
  )
  .bind(summary.date.format("%Y-%m-%d").to_string())
  .bind(&summary.source)
  .bind(summary.coverage_minutes as i64)
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
  .execute(executor)
  .await?;

  Ok(())
}

pub async fn delete_old_data(
  retention_days: u32,
  preserved_windows: &[(NaiveDate, NaiveDate)],
) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  delete_old_data_from_pool(&pool, retention_days, preserved_windows).await
}

/// Delete rows older than `retention_days`, except those inside any of
/// `preserved_windows`.
///
/// `preserved_windows` is the pinned ΔT baseline's calendar window. Once
/// it ages past the retention cutoff, deleting its rows would leave every
/// baseline-side ΔT comparison permanently empty while the pinned
/// baseline still claims that period as the reference, so it is exempt -
/// the same rule `cooling_daily_summary` applies to the absolute
/// baseline's window.
pub(crate) async fn delete_old_data_from_pool(
  pool: &SqlitePool,
  retention_days: u32,
  preserved_windows: &[(NaiveDate, NaiveDate)],
) -> Result<(), sqlx::Error> {
  let cutoff = (chrono::Local::now().date_naive()
    - chrono::Duration::days(retention_days as i64))
  .format("%Y-%m-%d")
  .to_string();

  let sql = preserving_delete_sql(
    "cooling_thermal_delta_daily_summary",
    "date",
    preserved_windows.len(),
  );
  let mut query = sqlx::query(&sql).bind(cutoff);
  for (start, end) in preserved_windows {
    query = query
      .bind(start.format("%Y-%m-%d").to_string())
      .bind(end.format("%Y-%m-%d").to_string());
  }
  query.execute(pool).await?;

  Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
  use super::super::test_schema::{
    AMBIENT_ARCHIVE_DDL, AMBIENT_ARCHIVE_TIMESTAMP_INDEX_DDL,
    COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL, DATA_ARCHIVE_DDL,
    DATA_ARCHIVE_TIMESTAMP_INDEX_DDL, create_tables,
  };
  use super::*;
  use crate::persistence::cooling_thermal_delta_rollup::summarize_thermal_delta_day;

  pub(crate) async fn setup_thermal_delta_daily_summary(pool: &SqlitePool) {
    create_tables(pool, &[COOLING_THERMAL_DELTA_DAILY_SUMMARY_DDL]).await;
  }

  /// The hardware archive plus the ambient archive the pairing query
  /// joins against, with the indexes the shipped schema carries.
  async fn setup_archives(pool: &SqlitePool) {
    create_tables(
      pool,
      &[
        DATA_ARCHIVE_DDL,
        DATA_ARCHIVE_TIMESTAMP_INDEX_DDL,
        AMBIENT_ARCHIVE_DDL,
        AMBIENT_ARCHIVE_TIMESTAMP_INDEX_DDL,
      ],
    )
    .await;
  }

  async fn insert_archive_row(
    pool: &SqlitePool,
    cpu_avg: Option<f64>,
    cpu_temperature_avg: Option<f64>,
    timestamp: DateTime<Utc>,
  ) {
    // Bind the native `DateTime<Utc>` so this matches exactly how the
    // real archive writer writes the column.
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

  async fn insert_ambient_row(
    pool: &SqlitePool,
    source: &str,
    temperature: f64,
    timestamp: DateTime<Utc>,
  ) {
    sqlx::query(
      "INSERT INTO AMBIENT_ARCHIVE (source, temperature, timestamp)
       VALUES ($1, $2, $3)",
    )
    .bind(source)
    .bind(temperature)
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

  fn band(avg: f32, minutes: u32) -> BandSummary {
    BandSummary {
      avg: Some(avg),
      max: Some(avg + 1.0),
      min: Some(avg - 1.0),
      sample_minutes: minutes,
    }
  }

  fn summary(d: NaiveDate, source: &str, idle_delta: f32) -> ThermalDeltaDailySummary {
    ThermalDeltaDailySummary {
      date: d,
      source: source.to_string(),
      coverage_minutes: 900,
      idle: band(idle_delta, 600),
      low: BandSummary::default(),
      mid: BandSummary::default(),
      high: BandSummary::default(),
    }
  }

  async fn paired_minutes_of_the_day(pool: &SqlitePool) -> Vec<ThermalDeltaMinuteSample> {
    select_thermal_delta_minutes_for_range_from_pool(
      pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap()
  }

  // ── pairing at the read boundary ──

  #[tokio::test]
  async fn a_minute_is_paired_with_the_ambient_row_stamped_to_the_same_minute() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    let tick = utc("2026-08-15T12:00:00.000Z");
    insert_archive_row(&pool, Some(5.0), Some(40.0), tick).await;
    insert_ambient_row(&pool, "Living Room", 25.0, tick).await;

    let rows = paired_minutes_of_the_day(&pool).await;

    assert_eq!(
      rows,
      vec![ThermalDeltaMinuteSample {
        timestamp: tick,
        source: "Living Room".to_string(),
        ambient_temperature: 25.0,
        cpu_usage_avg: Some(5.0),
        cpu_temperature_avg: Some(40.0),
        cpu_temperature_max: Some(40.0),
        cpu_temperature_min: Some(40.0),
        cpu_power_avg: None,
      }]
    );
  }

  #[tokio::test]
  async fn two_sources_in_one_minute_pair_as_two_samples_rather_than_one_mean() {
    // `AMBIENT_ARCHIVE` is row-per-source, and so is this read: a room
    // with two sensors yields two samples for one archived minute, each
    // carrying its own sensor's reading. The 26.0 a per-minute mean would
    // have produced appears nowhere.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    let tick = utc("2026-08-15T12:00:00.000Z");
    insert_archive_row(&pool, Some(5.0), Some(40.0), tick).await;
    insert_ambient_row(&pool, "Living Room", 24.0, tick).await;
    insert_ambient_row(&pool, "Desk", 28.0, tick).await;

    let rows = paired_minutes_of_the_day(&pool).await;

    assert_eq!(
      rows
        .iter()
        .map(|row| (row.source.as_str(), row.ambient_temperature))
        .collect::<Vec<_>>(),
      vec![("Desk", 28.0), ("Living Room", 24.0)]
    );
  }

  #[tokio::test]
  async fn a_sensor_that_went_quiet_leaves_the_minute_without_a_sample() {
    // The pairing is per minute. An ambient row an hour away must not
    // fill the gap - that is exactly the interpolation #2045 forbids.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:00.000Z"),
    )
    .await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(50.0),
      utc("2026-08-15T13:00:00.000Z"),
    )
    .await;
    insert_ambient_row(&pool, "Living Room", 25.0, utc("2026-08-15T12:00:00.000Z")).await;

    let rows = paired_minutes_of_the_day(&pool).await;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cpu_temperature_avg, Some(40.0));
  }

  #[tokio::test]
  async fn an_ambient_row_within_the_same_minute_still_pairs_across_seconds() {
    // The archive tick stamps both sides with one instant, but the join
    // is defined on the minute rather than on exact equality, so a
    // writer that ever drifted by seconds would still pair correctly.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:00.000Z"),
    )
    .await;
    insert_ambient_row(&pool, "Living Room", 25.0, utc("2026-08-15T12:00:59.900Z")).await;

    let rows = paired_minutes_of_the_day(&pool).await;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].ambient_temperature, 25.0);
  }

  #[tokio::test]
  async fn rows_split_across_a_minute_boundary_do_not_pair() {
    // Why the write cycle must stamp every table from one shared instant
    // rather than reading the clock per insert: one second of drift
    // across a minute boundary puts the two rows in different minutes,
    // and the pairing is defined on the minute.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:59.900Z"),
    )
    .await;
    insert_ambient_row(&pool, "Living Room", 25.0, utc("2026-08-15T12:01:00.100Z")).await;

    assert_eq!(
      paired_minutes_of_the_day(&pool).await,
      Vec::new(),
      "a reading from the next minute must not stand in for this one"
    );
  }

  #[tokio::test]
  async fn an_ambient_row_whose_minute_was_never_archived_yields_no_sample() {
    // The join is inner on purpose: a reading with no hardware minute to
    // pair with has nothing to subtract from and no coverage to record.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    insert_ambient_row(&pool, "Living Room", 25.0, utc("2026-08-15T12:00:00.000Z")).await;

    assert_eq!(paired_minutes_of_the_day(&pool).await, Vec::new());
  }

  #[tokio::test]
  async fn a_paired_minute_keeps_a_null_cpu_reading_rather_than_dropping_the_pair() {
    // A machine with an ambient sensor and no CPU temperature sensor
    // still pairs: the sample is what tells the rollup the minute had
    // coverage, even though no ΔT can come of it.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    let tick = utc("2026-08-15T12:00:00.000Z");
    insert_archive_row(&pool, Some(5.0), None, tick).await;
    insert_ambient_row(&pool, "Living Room", 25.0, tick).await;

    let rows = paired_minutes_of_the_day(&pool).await;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cpu_temperature_avg, None);
    assert_eq!(rows[0].cpu_usage_avg, Some(5.0));
  }

  #[tokio::test]
  async fn the_pairing_query_only_returns_minutes_within_the_half_open_range() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    for (tick, cpu) in [
      ("2026-08-14T23:59:59.000Z", 39.0),
      ("2026-08-15T00:00:00.000Z", 40.0),
      ("2026-08-15T23:59:59.999Z", 41.0),
      ("2026-08-16T00:00:00.000Z", 42.0),
    ] {
      insert_archive_row(&pool, Some(5.0), Some(cpu), utc(tick)).await;
      insert_ambient_row(&pool, "Living Room", 25.0, utc(tick)).await;
    }

    let rows = paired_minutes_of_the_day(&pool).await;

    assert_eq!(
      rows
        .iter()
        .map(|row| row.cpu_temperature_avg)
        .collect::<Vec<_>>(),
      vec![Some(40.0), Some(41.0)]
    );
  }

  // ── pairing query plan ──

  async fn query_plan(pool: &SqlitePool, sql: &str) -> String {
    let rows: Vec<(i64, i64, i64, String)> =
      sqlx::query_as(&format!("EXPLAIN QUERY PLAN {sql}"))
        .fetch_all(pool)
        .await
        .unwrap();
    rows
      .into_iter()
      .map(|(_, _, _, detail)| detail)
      .collect::<Vec<_>>()
      .join("\n")
  }

  #[tokio::test]
  async fn the_pairing_query_searches_both_timestamp_indexes_instead_of_scanning() {
    // Asserted on `EXPLAIN QUERY PLAN` rather than on timings: `SCAN
    // <table>` is SQLite's own word for reading every row, so it is the
    // exact thing to forbid. The SQL is restated with its binds
    // substituted so the planner sees exactly what production runs.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    let (start, end) = (
      utc("2026-08-15T00:00:00.000Z"),
      utc("2026-08-16T00:00:00.000Z"),
    );
    let epoch_ms = sqlite_epoch_milliseconds_of("DATA_ARCHIVE.timestamp");
    let ambient_epoch_ms = sqlite_epoch_milliseconds_of("AMBIENT_ARCHIVE.timestamp");
    let minute_key = sqlite_minute_key(&epoch_ms);
    let ambient_minute_key = sqlite_minute_key(&ambient_epoch_ms);
    let (lower, upper) = (
      raw_timestamp_bound(&start, -1),
      raw_timestamp_bound(&end, 1),
    );
    let (start_ms, end_ms) = (start.timestamp_millis(), end.timestamp_millis());
    let sql = format!(
      "SELECT ambient.source, ambient.ambient_temperature, DATA_ARCHIVE.cpu_avg
       FROM DATA_ARCHIVE
       JOIN (
         SELECT {ambient_minute_key} AS ambient_minute_key,
                AMBIENT_ARCHIVE.source AS source,
                AVG(CAST(AMBIENT_ARCHIVE.temperature AS REAL)) AS ambient_temperature
         FROM AMBIENT_ARCHIVE
         WHERE AMBIENT_ARCHIVE.timestamp >= '{lower}' AND AMBIENT_ARCHIVE.timestamp < '{upper}'
           AND {ambient_epoch_ms} >= {start_ms} AND {ambient_epoch_ms} < {end_ms}
         GROUP BY ambient_minute_key, AMBIENT_ARCHIVE.source
       ) AS ambient ON ambient.ambient_minute_key = {minute_key}
       WHERE DATA_ARCHIVE.timestamp >= '{lower}' AND DATA_ARCHIVE.timestamp < '{upper}'
         AND {epoch_ms} >= {start_ms} AND {epoch_ms} < {end_ms}
       ORDER BY DATA_ARCHIVE.timestamp ASC, ambient.source ASC"
    );

    let plan = query_plan(&pool, &sql).await;

    assert!(
      !plan.contains("SCAN DATA_ARCHIVE"),
      "the hardware side must not be a full table scan; plan was:\n{plan}"
    );
    assert!(
      !plan.contains("SCAN AMBIENT_ARCHIVE"),
      "the ambient side must not be a full table scan either; plan was:\n{plan}"
    );
  }

  #[tokio::test]
  async fn the_pairable_ambient_cursor_searches_the_timestamp_index() {
    // The correlated EXISTS is the worst case: without the raw bracket
    // it re-reads all of `DATA_ARCHIVE` once per ambient row.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    let ambient_epoch_ms = sqlite_epoch_milliseconds_of("AMBIENT_ARCHIVE.timestamp");
    let hardware_epoch_ms = sqlite_epoch_milliseconds_of("DATA_ARCHIVE.timestamp");
    let ambient_minute_key = sqlite_minute_key(&ambient_epoch_ms);
    let hardware_minute_key = sqlite_minute_key(&hardware_epoch_ms);
    let sql = format!(
      "SELECT MAX(AMBIENT_ARCHIVE.timestamp) FROM AMBIENT_ARCHIVE
       WHERE {ambient_epoch_ms} < 1
         AND EXISTS (
           SELECT 1 FROM DATA_ARCHIVE
           WHERE DATA_ARCHIVE.timestamp >= strftime(
                   '%Y-%m-%dT%H:%M:%S', AMBIENT_ARCHIVE.timestamp, '-2 minutes')
             AND DATA_ARCHIVE.timestamp <= strftime(
                   '%Y-%m-%dT%H:%M:%S', AMBIENT_ARCHIVE.timestamp, '+2 minutes')
             AND {hardware_minute_key} = {ambient_minute_key}
         )"
    );

    let plan = query_plan(&pool, &sql).await;

    assert!(
      plan.contains("idx_data_archive_timestamp"),
      "the correlated subquery must search the index; plan was:\n{plan}"
    );
    assert!(
      !plan.contains("SCAN DATA_ARCHIVE"),
      "the correlated subquery must not scan the archive per ambient row; plan was:\n{plan}"
    );
  }

  // ── backfill cursor ──

  #[tokio::test]
  async fn max_summarized_date_is_none_for_an_empty_table() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_thermal_delta_daily_summary(&pool).await;

    assert_eq!(max_summarized_date_from_pool(&pool).await.unwrap(), None);
  }

  #[tokio::test]
  async fn max_pairable_ambient_archive_timestamp_only_sees_rows_before_the_bound() {
    // The bound is the start of today: today's ambient rows must not
    // count, or a machine collecting ambient right now would look
    // permanently behind and rewind the catch-up on every cycle.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    for tick in [
      utc("2026-08-19T23:59:00.000Z"),
      utc("2026-08-20T00:00:00.000Z"),
    ] {
      insert_archive_row(&pool, Some(5.0), Some(40.0), tick).await;
      insert_ambient_row(&pool, "Living Room", 25.0, tick).await;
    }

    let latest = max_pairable_ambient_archive_timestamp_before_from_pool(
      &pool,
      &utc("2026-08-20T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(latest, Some(utc("2026-08-19T23:59:00.000Z")));
  }

  #[tokio::test]
  async fn max_pairable_ambient_archive_timestamp_is_none_without_an_ambient_sensor() {
    // What stops the backfill check from firing forever on a machine
    // that has no environmental sensor at all.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-19T10:00:00.000Z"),
    )
    .await;

    assert_eq!(
      max_pairable_ambient_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-20T00:00:00.000Z")
      )
      .await
      .unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn max_pairable_ambient_archive_timestamp_skips_an_unpairable_ambient_row() {
    // The rollup writes a row per *archived minute* that paired, so an
    // ambient row whose minute has no hardware row can never become
    // coverage however often the day is re-rolled. Counting it would
    // send the catch-up chasing a day it can never fill.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-18T10:00:00.000Z"),
    )
    .await;
    insert_ambient_row(&pool, "Living Room", 25.0, utc("2026-08-18T10:00:00.000Z")).await;
    // Later, but with no hardware row for its minute.
    insert_ambient_row(&pool, "Living Room", 26.0, utc("2026-08-19T10:00:00.000Z")).await;

    assert_eq!(
      max_pairable_ambient_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-20T00:00:00.000Z")
      )
      .await
      .unwrap(),
      Some(utc("2026-08-18T10:00:00.000Z"))
    );
  }

  // ── round trips ──

  #[tokio::test]
  async fn every_source_of_a_day_round_trips_as_its_own_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_thermal_delta_daily_summary(&pool).await;

    for entry in [
      summary(date(2026, 8, 15), "Living Room", 12.0),
      summary(date(2026, 8, 15), "Desk", 15.0),
      summary(date(2026, 8, 10), "Desk", 14.0),
    ] {
      upsert_with(&pool, &entry).await.unwrap();
    }

    let rows = select_all_thermal_delta_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(
      rows
        .iter()
        .map(|row| (row.date, row.source.as_str(), row.idle.avg))
        .collect::<Vec<_>>(),
      vec![
        (date(2026, 8, 10), "Desk", Some(14.0)),
        (date(2026, 8, 15), "Desk", Some(15.0)),
        (date(2026, 8, 15), "Living Room", Some(12.0)),
      ]
    );
    assert_eq!(rows[2], summary(date(2026, 8, 15), "Living Room", 12.0));
    assert_eq!(
      max_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 15))
    );
  }

  #[tokio::test]
  async fn a_band_with_no_paired_minute_round_trips_as_absent_rather_than_zero() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_thermal_delta_daily_summary(&pool).await;
    let coverage_only = ThermalDeltaDailySummary {
      date: date(2026, 8, 15),
      source: "Desk".to_string(),
      coverage_minutes: 300,
      idle: BandSummary::default(),
      low: BandSummary::default(),
      mid: BandSummary::default(),
      high: BandSummary::default(),
    };

    upsert_with(&pool, &coverage_only).await.unwrap();

    let rows = select_all_thermal_delta_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    assert_eq!(rows, vec![coverage_only]);
  }

  #[tokio::test]
  async fn upsert_is_idempotent_per_date_and_source() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_thermal_delta_daily_summary(&pool).await;

    let mut entry = summary(date(2026, 8, 15), "Desk", 12.0);
    upsert_with(&pool, &entry).await.unwrap();
    entry.idle = band(20.0, 1200);
    entry.coverage_minutes = 1440;
    upsert_with(&pool, &entry).await.unwrap();

    let rows = select_all_thermal_delta_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(rows.len(), 1, "re-running a day must not duplicate the row");
    assert_eq!(rows[0].idle.avg, Some(20.0));
    assert_eq!(rows[0].coverage_minutes, 1440);
  }

  #[tokio::test]
  async fn delete_old_data_removes_rows_strictly_before_the_cutoff() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_thermal_delta_daily_summary(&pool).await;
    let today = chrono::Local::now().date_naive();
    let just_inside = today - chrono::Duration::days(400);
    let just_outside = just_inside - chrono::Duration::days(1);

    for d in [just_inside, just_outside] {
      upsert_with(&pool, &summary(d, "Desk", 12.0)).await.unwrap();
    }

    delete_old_data_from_pool(&pool, 400, &[]).await.unwrap();

    let rows = select_all_thermal_delta_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    assert_eq!(
      rows.iter().map(|row| row.date).collect::<Vec<_>>(),
      vec![just_inside],
      "the row exactly at the retention boundary must survive"
    );
  }

  #[tokio::test]
  async fn delete_old_data_keeps_the_pinned_delta_baseline_window_past_the_cutoff() {
    // The pinned ΔT baseline never expires, so once its window ages past
    // the retention cutoff its rows must still survive - otherwise every
    // baseline-side ΔT comparison goes permanently empty while the
    // baseline still names that period as the reference.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_thermal_delta_daily_summary(&pool).await;
    let today = chrono::Local::now().date_naive();
    let window_start = today - chrono::Duration::days(500);
    let window_end = window_start + chrono::Duration::days(6);
    let outside_window = window_start - chrono::Duration::days(1);

    for d in [outside_window, window_start, window_end] {
      upsert_with(&pool, &summary(d, "Desk", 12.0)).await.unwrap();
    }

    delete_old_data_from_pool(&pool, 400, &[(window_start, window_end)])
      .await
      .unwrap();

    let rows = select_all_thermal_delta_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    assert_eq!(
      rows.iter().map(|row| row.date).collect::<Vec<_>>(),
      vec![window_start, window_end],
      "both window edges must survive, and the day just outside it must not"
    );
  }

  // ── archives to daily summary, end to end ──

  #[tokio::test]
  async fn a_paired_ambient_archive_reaches_a_persisted_row_per_source() {
    // The whole path in one test: real archive rows on both sides, the
    // real join, the real fold, the real write, the real read-back. The
    // unit tests above each cover one hop; this is what catches a column
    // name that only some of the hops agree on.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    setup_thermal_delta_daily_summary(&pool).await;

    // Three idle minutes: CPU 40/44/48 against a Living Room sensor at
    // 20/24/28, so every minute's delta is exactly 20 K even though both
    // series are climbing - a rise the room explains leaves the delta
    // flat. A Desk sensor 2 K warmer shares the first two minutes only.
    for (minute, cpu, ambient) in [(0, 40.0, 20.0), (1, 44.0, 24.0), (2, 48.0, 28.0)] {
      let tick = utc(&format!("2026-08-15T12:0{minute}:00.000Z"));
      insert_archive_row(&pool, Some(5.0), Some(cpu), tick).await;
      insert_ambient_row(&pool, "Living Room", ambient, tick).await;
      if minute < 2 {
        insert_ambient_row(&pool, "Desk", ambient + 2.0, tick).await;
      }
    }
    // A fourth minute both sensors missed: it belongs to the absolute
    // rollup alone and must not touch either source's row.
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(100.0),
      utc("2026-08-15T12:03:00.000Z"),
    )
    .await;

    let minutes = paired_minutes_of_the_day(&pool).await;
    for row in summarize_thermal_delta_day(date(2026, 8, 15), &minutes) {
      upsert_with(&pool, &row).await.unwrap();
    }

    let rows = select_all_thermal_delta_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(
      rows
        .iter()
        .map(|row| {
          (
            row.source.as_str(),
            row.coverage_minutes,
            row.idle.avg,
            row.idle.sample_minutes,
          )
        })
        .collect::<Vec<_>>(),
      vec![
        ("Desk", 2, Some(18.0), 2),
        ("Living Room", 3, Some(20.0), 3)
      ]
    );
    // And the backfill cursor now agrees the projection is current.
    assert_eq!(
      max_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 15))
    );
  }

  #[tokio::test]
  async fn an_archive_without_ambient_persists_no_thermal_delta_row() {
    // The same path on a machine with no environmental sensor: no row
    // rather than a 0 K one, and the backfill cursor reports nothing to
    // catch up.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    setup_thermal_delta_daily_summary(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:00.000Z"),
    )
    .await;

    let minutes = paired_minutes_of_the_day(&pool).await;
    for row in summarize_thermal_delta_day(date(2026, 8, 15), &minutes) {
      upsert_with(&pool, &row).await.unwrap();
    }

    assert_eq!(
      select_all_thermal_delta_daily_summaries_from_pool(&pool)
        .await
        .unwrap(),
      Vec::new()
    );
    assert_eq!(max_summarized_date_from_pool(&pool).await.unwrap(), None);
    assert_eq!(
      max_pairable_ambient_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-16T00:00:00.000Z")
      )
      .await
      .unwrap(),
      None,
      "neither side has ambient, so the catch-up must never claim to be behind"
    );
  }
}
