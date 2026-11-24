//! `cooling_covariate_daily_summary` and
//! `cooling_fan_covariate_daily_summary` reads and writes (#2068).
//!
//! Follows the `_from_pool` split the other cooling query modules use: the
//! public `async fn`s resolve Core's process-wide pool, and delegate to a
//! `_from_pool` variant tests can drive against an in-memory database.
//!
//! One module for both tables: they are two shapes of one projection,
//! folded from one read and written in one transaction, and nothing reads
//! one without the other. The band table is keyed by
//! `(date, source, band)` and the fan table by
//! `(date, source, fan_source, band)` - the ambient source is on every
//! row, as on `cooling_thermal_delta_daily_summary`, because which sensor
//! the Thermal Delta was measured against is part of the fit. A sensor
//! change can never blend two placements into one row.

use super::cooling_daily_summary::preserving_delete_sql;
use super::cooling_thermal_delta_daily_summary::pairable_ambient_cursor_sql;
use super::db;
use crate::persistence::cooling_covariate_rollup::{
  CovariateDailySummary, FanCovariateDailySummary, PairedFitStatistics,
};
use crate::persistence::cooling_rollup::CpuLoadBand;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

/// `MAX(date)` in `cooling_covariate_daily_summary` - the co-variate
/// projection's own catch-up cursor. A row exists only for a
/// `(day, source, band)` that saw a paired, classifiable minute, so this
/// is the latest day any source recorded one. Paired with
/// [`max_classifiable_pairable_ambient_archive_timestamp_before`] this is
/// how the rollup detects the projection is behind the archives (see
/// `cooling_rollup::covariate_rollup_is_behind`).
///
/// The fan table has no cursor of its own: a fan row needs a paired
/// classifiable minute to sit beside, so it never exists on a day the
/// band table has no row for.
pub async fn max_summarized_date() -> Result<Option<NaiveDate>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_summarized_date_from_pool(&pool).await
}

pub(crate) async fn max_summarized_date_from_pool(
  pool: &SqlitePool,
) -> Result<Option<NaiveDate>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<NaiveDate>>(
    "SELECT MAX(date) FROM cooling_covariate_daily_summary",
  )
  .fetch_one(pool)
  .await
}

/// The most recent ambient archive timestamp strictly before `before`
/// whose minute also has a `DATA_ARCHIVE` row carrying a CPU usage
/// reading.
///
/// The ΔT cursor's own query
/// (`cooling_thermal_delta_daily_summary::max_pairable_ambient_archive_timestamp_before`)
/// narrowed by one more predicate, because this rollup's row gate is one
/// step stricter than the ΔT rollup's: a paired minute with no usage
/// reading has no band to be filed under and yields no row here, so
/// counting it as evidence of a missed day would send the catch-up
/// chasing a day it can never fill. Same completed-days bound, same
/// pairing rule, same query plan.
pub async fn max_classifiable_pairable_ambient_archive_timestamp_before(
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_classifiable_pairable_ambient_archive_timestamp_before_from_pool(&pool, before)
    .await
}

/// The hardware-side clause that makes a paired minute classifiable.
const CLASSIFIABLE_PREDICATE: &str = "AND DATA_ARCHIVE.cpu_avg IS NOT NULL";

pub(crate) async fn max_classifiable_pairable_ambient_archive_timestamp_before_from_pool(
  pool: &SqlitePool,
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<DateTime<Utc>>>(&pairable_ambient_cursor_sql(
    CLASSIFIABLE_PREDICATE,
  ))
  .bind(before.timestamp_millis())
  .fetch_one(pool)
  .await
}

/// Every summarized source-band-day, oldest first and grouped by source
/// then band within a day. Reads the whole table for the same reason
/// `cooling_thermal_delta_daily_summary::select_all_thermal_delta_daily_summaries`
/// does: it holds at most one retention window of narrow rows per source
/// and band, and both windows the comparison reads are date ranges over
/// it.
pub async fn select_all_covariate_daily_summaries()
-> Result<Vec<CovariateDailySummary>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_all_covariate_daily_summaries_from_pool(&pool).await
}

fn band_from_key(key: &str) -> Result<CpuLoadBand, sqlx::Error> {
  CpuLoadBand::from_column_key(key).ok_or_else(|| {
    sqlx::Error::Decode(format!("unknown CPU-load band key {key:?}").into())
  })
}

/// The column is `NOT NULL DEFAULT 0` and only ever written from a
/// `u32`; clamp rather than wrap if a hand-edited database ever carries a
/// negative count.
fn count(value: i64) -> u32 {
  value.max(0) as u32
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct CovariateDailySummaryRow {
  date: NaiveDate,
  source: String,
  band: String,
  sample_minutes: i64,
  band_share: f64,
  ambient_temperature_median: f64,
  delta_minutes: i64,
  delta_temperature_median: Option<f64>,
  power_minutes: i64,
  cpu_power_median: Option<f64>,
  power_fit_n: i64,
  power_fit_sum_x: f64,
  power_fit_sum_y: f64,
  power_fit_sum_xy: f64,
  power_fit_sum_xx: f64,
  power_fit_sum_yy: f64,
}

impl TryFrom<CovariateDailySummaryRow> for CovariateDailySummary {
  type Error = sqlx::Error;

  fn try_from(row: CovariateDailySummaryRow) -> Result<Self, sqlx::Error> {
    Ok(Self {
      date: row.date,
      source: row.source,
      band: band_from_key(&row.band)?,
      sample_minutes: count(row.sample_minutes),
      band_share: row.band_share as f32,
      ambient_temperature_median: row.ambient_temperature_median as f32,
      delta_minutes: count(row.delta_minutes),
      delta_temperature_median: row.delta_temperature_median.map(|v| v as f32),
      power_minutes: count(row.power_minutes),
      cpu_power_median: row.cpu_power_median.map(|v| v as f32),
      delta_per_watt: PairedFitStatistics {
        n: count(row.power_fit_n),
        sum_x: row.power_fit_sum_x,
        sum_y: row.power_fit_sum_y,
        sum_xy: row.power_fit_sum_xy,
        sum_xx: row.power_fit_sum_xx,
        sum_yy: row.power_fit_sum_yy,
      },
    })
  }
}

pub(crate) async fn select_all_covariate_daily_summaries_from_pool(
  pool: &SqlitePool,
) -> Result<Vec<CovariateDailySummary>, sqlx::Error> {
  // `date` is stored as "%Y-%m-%d", which sorts lexicographically the same
  // as chronologically (the same assumption `delete_old_data` makes).
  let rows = sqlx::query_as::<_, CovariateDailySummaryRow>(
    "SELECT date, source, band, sample_minutes, band_share, ambient_temperature_median,
       delta_minutes, delta_temperature_median, power_minutes, cpu_power_median,
       power_fit_n, power_fit_sum_x, power_fit_sum_y, power_fit_sum_xy, power_fit_sum_xx, power_fit_sum_yy
     FROM cooling_covariate_daily_summary
     ORDER BY date ASC, source ASC, band ASC",
  )
  .fetch_all(pool)
  .await?;

  rows
    .into_iter()
    .map(CovariateDailySummary::try_from)
    .collect()
}

/// Every summarized fan row, oldest first and grouped by source, fan and
/// band within a day.
pub async fn select_all_fan_covariate_daily_summaries()
-> Result<Vec<FanCovariateDailySummary>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_all_fan_covariate_daily_summaries_from_pool(&pool).await
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct FanCovariateDailySummaryRow {
  date: NaiveDate,
  source: String,
  fan_source: String,
  band: String,
  rpm_minutes: i64,
  rpm_median: f64,
  fit_n: i64,
  fit_sum_x: f64,
  fit_sum_y: f64,
  fit_sum_xy: f64,
  fit_sum_xx: f64,
  fit_sum_yy: f64,
}

impl TryFrom<FanCovariateDailySummaryRow> for FanCovariateDailySummary {
  type Error = sqlx::Error;

  fn try_from(row: FanCovariateDailySummaryRow) -> Result<Self, sqlx::Error> {
    Ok(Self {
      date: row.date,
      source: row.source,
      fan_source: row.fan_source,
      band: band_from_key(&row.band)?,
      rpm_minutes: count(row.rpm_minutes),
      rpm_median: row.rpm_median as f32,
      delta_per_rpm: PairedFitStatistics {
        n: count(row.fit_n),
        sum_x: row.fit_sum_x,
        sum_y: row.fit_sum_y,
        sum_xy: row.fit_sum_xy,
        sum_xx: row.fit_sum_xx,
        sum_yy: row.fit_sum_yy,
      },
    })
  }
}

pub(crate) async fn select_all_fan_covariate_daily_summaries_from_pool(
  pool: &SqlitePool,
) -> Result<Vec<FanCovariateDailySummary>, sqlx::Error> {
  let rows = sqlx::query_as::<_, FanCovariateDailySummaryRow>(
    "SELECT date, source, fan_source, band, rpm_minutes, rpm_median,
       fit_n, fit_sum_x, fit_sum_y, fit_sum_xy, fit_sum_xx, fit_sum_yy
     FROM cooling_fan_covariate_daily_summary
     ORDER BY date ASC, source ASC, fan_source ASC, band ASC",
  )
  .fetch_all(pool)
  .await?;

  rows
    .into_iter()
    .map(FanCovariateDailySummary::try_from)
    .collect()
}

/// Upsert one source-band-day against any executor, so a day's
/// projections share one transaction (see
/// `cooling_rollup::persist_day_rollup_from_pool`).
pub(crate) async fn upsert_with<'e, E>(
  executor: E,
  summary: &CovariateDailySummary,
) -> Result<(), sqlx::Error>
where
  E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
  let fit = &summary.delta_per_watt;
  sqlx::query(
    "INSERT INTO cooling_covariate_daily_summary (
       date, source, band, sample_minutes, band_share, ambient_temperature_median,
       delta_minutes, delta_temperature_median, power_minutes, cpu_power_median,
       power_fit_n, power_fit_sum_x, power_fit_sum_y, power_fit_sum_xy, power_fit_sum_xx, power_fit_sum_yy
     )
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
     ON CONFLICT(date, source, band) DO UPDATE SET
       sample_minutes = excluded.sample_minutes,
       band_share = excluded.band_share,
       ambient_temperature_median = excluded.ambient_temperature_median,
       delta_minutes = excluded.delta_minutes,
       delta_temperature_median = excluded.delta_temperature_median,
       power_minutes = excluded.power_minutes,
       cpu_power_median = excluded.cpu_power_median,
       power_fit_n = excluded.power_fit_n,
       power_fit_sum_x = excluded.power_fit_sum_x,
       power_fit_sum_y = excluded.power_fit_sum_y,
       power_fit_sum_xy = excluded.power_fit_sum_xy,
       power_fit_sum_xx = excluded.power_fit_sum_xx,
       power_fit_sum_yy = excluded.power_fit_sum_yy",
  )
  .bind(summary.date.format("%Y-%m-%d").to_string())
  .bind(&summary.source)
  .bind(summary.band.column_key())
  .bind(summary.sample_minutes as i64)
  .bind(summary.band_share)
  .bind(summary.ambient_temperature_median)
  .bind(summary.delta_minutes as i64)
  .bind(summary.delta_temperature_median)
  .bind(summary.power_minutes as i64)
  .bind(summary.cpu_power_median)
  .bind(fit.n as i64)
  .bind(fit.sum_x)
  .bind(fit.sum_y)
  .bind(fit.sum_xy)
  .bind(fit.sum_xx)
  .bind(fit.sum_yy)
  .execute(executor)
  .await?;

  Ok(())
}

/// Upsert one fan row against any executor.
pub(crate) async fn upsert_fan_with<'e, E>(
  executor: E,
  summary: &FanCovariateDailySummary,
) -> Result<(), sqlx::Error>
where
  E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
  let fit = &summary.delta_per_rpm;
  sqlx::query(
    "INSERT INTO cooling_fan_covariate_daily_summary (
       date, source, fan_source, band, rpm_minutes, rpm_median,
       fit_n, fit_sum_x, fit_sum_y, fit_sum_xy, fit_sum_xx, fit_sum_yy
     )
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
     ON CONFLICT(date, source, fan_source, band) DO UPDATE SET
       rpm_minutes = excluded.rpm_minutes,
       rpm_median = excluded.rpm_median,
       fit_n = excluded.fit_n,
       fit_sum_x = excluded.fit_sum_x,
       fit_sum_y = excluded.fit_sum_y,
       fit_sum_xy = excluded.fit_sum_xy,
       fit_sum_xx = excluded.fit_sum_xx,
       fit_sum_yy = excluded.fit_sum_yy",
  )
  .bind(summary.date.format("%Y-%m-%d").to_string())
  .bind(&summary.source)
  .bind(&summary.fan_source)
  .bind(summary.band.column_key())
  .bind(summary.rpm_minutes as i64)
  .bind(summary.rpm_median)
  .bind(fit.n as i64)
  .bind(fit.sum_x)
  .bind(fit.sum_y)
  .bind(fit.sum_xy)
  .bind(fit.sum_xx)
  .bind(fit.sum_yy)
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

/// Delete rows of both tables older than `retention_days`, except those
/// inside any of `preserved_windows`.
///
/// `preserved_windows` is the pinned ΔT baseline's calendar window - the
/// same exemption `cooling_thermal_delta_daily_summary` applies, because
/// the co-variate comparison reads its baseline side from exactly that
/// window and would go permanently empty once it aged past the cutoff.
pub(crate) async fn delete_old_data_from_pool(
  pool: &SqlitePool,
  retention_days: u32,
  preserved_windows: &[(NaiveDate, NaiveDate)],
) -> Result<(), sqlx::Error> {
  let cutoff = (chrono::Local::now().date_naive()
    - chrono::Duration::days(retention_days as i64))
  .format("%Y-%m-%d")
  .to_string();

  for table in [
    "cooling_covariate_daily_summary",
    "cooling_fan_covariate_daily_summary",
  ] {
    let sql = preserving_delete_sql(table, "date", preserved_windows.len());
    let mut query = sqlx::query(&sql).bind(cutoff.as_str());
    for (start, end) in preserved_windows {
      query = query
        .bind(start.format("%Y-%m-%d").to_string())
        .bind(end.format("%Y-%m-%d").to_string());
    }
    query.execute(pool).await?;
  }

  Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
  use super::super::test_schema::{
    AMBIENT_ARCHIVE_DDL, AMBIENT_ARCHIVE_TIMESTAMP_INDEX_DDL,
    COOLING_COVARIATE_DAILY_SUMMARY_DDL, COOLING_FAN_COVARIATE_DAILY_SUMMARY_DDL,
    DATA_ARCHIVE_DDL, DATA_ARCHIVE_TIMESTAMP_INDEX_DDL, create_tables,
  };
  use super::*;

  pub(crate) async fn setup_covariate_daily_summaries(pool: &SqlitePool) {
    create_tables(
      pool,
      &[
        COOLING_COVARIATE_DAILY_SUMMARY_DDL,
        COOLING_FAN_COVARIATE_DAILY_SUMMARY_DDL,
      ],
    )
    .await;
  }

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

  fn fit(n: u32) -> PairedFitStatistics {
    PairedFitStatistics {
      n,
      sum_x: 10.0 * n as f64,
      sum_y: 15.0 * n as f64,
      sum_xy: 150.0 * n as f64,
      sum_xx: 100.0 * n as f64,
      sum_yy: 225.0 * n as f64,
    }
  }

  fn summary(
    d: NaiveDate,
    source: &str,
    band: CpuLoadBand,
    delta_median: f32,
  ) -> CovariateDailySummary {
    CovariateDailySummary {
      date: d,
      source: source.to_string(),
      band,
      sample_minutes: 600,
      band_share: 0.8,
      ambient_temperature_median: 25.0,
      delta_minutes: 590,
      delta_temperature_median: Some(delta_median),
      power_minutes: 580,
      cpu_power_median: Some(12.5),
      delta_per_watt: fit(570),
    }
  }

  fn fan_summary(
    d: NaiveDate,
    source: &str,
    fan_source: &str,
    band: CpuLoadBand,
  ) -> FanCovariateDailySummary {
    FanCovariateDailySummary {
      date: d,
      source: source.to_string(),
      fan_source: fan_source.to_string(),
      band,
      rpm_minutes: 600,
      rpm_median: 950.0,
      delta_per_rpm: fit(590),
    }
  }

  // ── backfill cursor ──

  #[tokio::test]
  async fn max_summarized_date_is_none_for_an_empty_table() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;

    assert_eq!(max_summarized_date_from_pool(&pool).await.unwrap(), None);
  }

  #[tokio::test]
  async fn the_classifiable_cursor_only_sees_rows_before_the_bound() {
    // Same completed-days bound as the ΔT cursor: today's rows must not
    // count, or a machine collecting right now would rewind every cycle.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    for tick in [
      utc("2026-08-19T23:59:00.000Z"),
      utc("2026-08-20T00:00:00.000Z"),
    ] {
      insert_archive_row(&pool, Some(5.0), Some(40.0), tick).await;
      insert_ambient_row(&pool, "Living Room", 25.0, tick).await;
    }

    let latest = max_classifiable_pairable_ambient_archive_timestamp_before_from_pool(
      &pool,
      &utc("2026-08-20T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(latest, Some(utc("2026-08-19T23:59:00.000Z")));
  }

  #[tokio::test]
  async fn the_classifiable_cursor_is_none_without_an_ambient_sensor() {
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
      max_classifiable_pairable_ambient_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-20T00:00:00.000Z")
      )
      .await
      .unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn the_classifiable_cursor_skips_a_paired_minute_without_a_usage_reading() {
    // The one way this cursor differs from the ΔT one: a paired minute
    // with no usage reading has no band, so the rollup writes no row for
    // it, and counting it would send the catch-up chasing a day it can
    // never fill.
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
    insert_archive_row(&pool, None, Some(40.0), utc("2026-08-19T10:00:00.000Z")).await;
    insert_ambient_row(&pool, "Living Room", 26.0, utc("2026-08-19T10:00:00.000Z")).await;

    assert_eq!(
      max_classifiable_pairable_ambient_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-20T00:00:00.000Z")
      )
      .await
      .unwrap(),
      Some(utc("2026-08-18T10:00:00.000Z"))
    );
  }

  #[tokio::test]
  async fn the_classifiable_cursor_searches_the_timestamp_index() {
    // The same plan the ΔT cursor is held to: the extra predicate must
    // not turn the correlated subquery back into a per-row scan.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_archives(&pool).await;
    let sql = pairable_ambient_cursor_sql(CLASSIFIABLE_PREDICATE).replace("$1", "1");

    let rows: Vec<(i64, i64, i64, String)> =
      sqlx::query_as(&format!("EXPLAIN QUERY PLAN {sql}"))
        .fetch_all(&pool)
        .await
        .unwrap();
    let plan = rows
      .into_iter()
      .map(|(_, _, _, detail)| detail)
      .collect::<Vec<_>>()
      .join("\n");

    assert!(
      plan.contains("idx_data_archive_timestamp"),
      "the correlated subquery must search the index; plan was:\n{plan}"
    );
    assert!(
      !plan.contains("SCAN DATA_ARCHIVE"),
      "the correlated subquery must not scan the archive per ambient row; plan was:\n{plan}"
    );
  }

  // ── round trips ──

  #[tokio::test]
  async fn every_source_and_band_of_a_day_round_trips_as_its_own_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;

    for entry in [
      summary(date(2026, 8, 15), "Living Room", CpuLoadBand::Idle, 12.0),
      summary(date(2026, 8, 15), "Desk", CpuLoadBand::High, 30.0),
      summary(date(2026, 8, 15), "Desk", CpuLoadBand::Idle, 15.0),
      summary(date(2026, 8, 10), "Desk", CpuLoadBand::Idle, 14.0),
    ] {
      upsert_with(&pool, &entry).await.unwrap();
    }

    let rows = select_all_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(
      rows
        .iter()
        .map(|row| (row.date, row.source.as_str(), row.band))
        .collect::<Vec<_>>(),
      vec![
        (date(2026, 8, 10), "Desk", CpuLoadBand::Idle),
        (date(2026, 8, 15), "Desk", CpuLoadBand::High),
        (date(2026, 8, 15), "Desk", CpuLoadBand::Idle),
        (date(2026, 8, 15), "Living Room", CpuLoadBand::Idle),
      ]
    );
    assert_eq!(
      rows[3],
      summary(date(2026, 8, 15), "Living Room", CpuLoadBand::Idle, 12.0)
    );
    assert_eq!(
      max_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 15))
    );
  }

  #[tokio::test]
  async fn every_fan_of_a_source_band_day_round_trips_as_its_own_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;

    for entry in [
      fan_summary(date(2026, 8, 15), "Desk", "Fan 2", CpuLoadBand::Idle),
      fan_summary(date(2026, 8, 15), "Desk", "Fan 1", CpuLoadBand::Idle),
      fan_summary(date(2026, 8, 15), "Living Room", "Fan 1", CpuLoadBand::Idle),
    ] {
      upsert_fan_with(&pool, &entry).await.unwrap();
    }

    let rows = select_all_fan_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(
      rows
        .iter()
        .map(|row| (row.source.as_str(), row.fan_source.as_str()))
        .collect::<Vec<_>>(),
      vec![
        ("Desk", "Fan 1"),
        ("Desk", "Fan 2"),
        ("Living Room", "Fan 1")
      ]
    );
    assert_eq!(
      rows[0],
      fan_summary(date(2026, 8, 15), "Desk", "Fan 1", CpuLoadBand::Idle)
    );
  }

  #[tokio::test]
  async fn an_absent_reading_round_trips_as_absent_rather_than_zero() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;
    let ambient_only = CovariateDailySummary {
      delta_minutes: 0,
      delta_temperature_median: None,
      power_minutes: 0,
      cpu_power_median: None,
      delta_per_watt: PairedFitStatistics::default(),
      ..summary(date(2026, 8, 15), "Desk", CpuLoadBand::Idle, 0.0)
    };

    upsert_with(&pool, &ambient_only).await.unwrap();

    let rows = select_all_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    assert_eq!(rows, vec![ambient_only]);
  }

  #[tokio::test]
  async fn upsert_is_idempotent_per_date_source_and_band() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;

    let mut entry = summary(date(2026, 8, 15), "Desk", CpuLoadBand::Idle, 12.0);
    upsert_with(&pool, &entry).await.unwrap();
    entry.delta_temperature_median = Some(20.0);
    entry.delta_per_watt = fit(1200);
    upsert_with(&pool, &entry).await.unwrap();
    let mut fan = fan_summary(date(2026, 8, 15), "Desk", "Fan 1", CpuLoadBand::Idle);
    upsert_fan_with(&pool, &fan).await.unwrap();
    fan.rpm_median = 1200.0;
    upsert_fan_with(&pool, &fan).await.unwrap();

    let rows = select_all_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    let fans = select_all_fan_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(rows.len(), 1, "re-running a day must not duplicate the row");
    assert_eq!(rows[0].delta_temperature_median, Some(20.0));
    assert_eq!(rows[0].delta_per_watt.n, 1200);
    assert_eq!(fans.len(), 1);
    assert_eq!(fans[0].rpm_median, 1200.0);
  }

  #[tokio::test]
  async fn a_row_with_a_band_key_no_band_ever_wrote_is_a_decode_error() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;
    sqlx::query(
      "INSERT INTO cooling_covariate_daily_summary
         (date, source, band, sample_minutes, band_share, ambient_temperature_median)
       VALUES ('2026-08-15', 'Desk', 'turbo', 1, 1.0, 25.0)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = select_all_covariate_daily_summaries_from_pool(&pool).await;

    assert!(matches!(result, Err(sqlx::Error::Decode(_))));
  }

  #[tokio::test]
  async fn delete_old_data_removes_rows_of_both_tables_strictly_before_the_cutoff() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;
    let today = chrono::Local::now().date_naive();
    let just_inside = today - chrono::Duration::days(400);
    let just_outside = just_inside - chrono::Duration::days(1);

    for d in [just_inside, just_outside] {
      upsert_with(&pool, &summary(d, "Desk", CpuLoadBand::Idle, 12.0))
        .await
        .unwrap();
      upsert_fan_with(&pool, &fan_summary(d, "Desk", "Fan 1", CpuLoadBand::Idle))
        .await
        .unwrap();
    }

    delete_old_data_from_pool(&pool, 400, &[]).await.unwrap();

    let rows = select_all_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    let fans = select_all_fan_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    assert_eq!(
      rows.iter().map(|row| row.date).collect::<Vec<_>>(),
      vec![just_inside],
      "the row exactly at the retention boundary must survive"
    );
    assert_eq!(
      fans.iter().map(|row| row.date).collect::<Vec<_>>(),
      vec![just_inside]
    );
  }

  #[tokio::test]
  async fn delete_old_data_keeps_the_pinned_delta_baseline_window_past_the_cutoff() {
    // The comparison's baseline side is read from the ΔT baseline's own
    // window, so those rows must outlive the cutoff exactly as the ΔT
    // table's do.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_covariate_daily_summaries(&pool).await;
    let today = chrono::Local::now().date_naive();
    let window_start = today - chrono::Duration::days(500);
    let window_end = window_start + chrono::Duration::days(6);
    let outside_window = window_start - chrono::Duration::days(1);

    for d in [outside_window, window_start, window_end] {
      upsert_with(&pool, &summary(d, "Desk", CpuLoadBand::Idle, 12.0))
        .await
        .unwrap();
      upsert_fan_with(&pool, &fan_summary(d, "Desk", "Fan 1", CpuLoadBand::Idle))
        .await
        .unwrap();
    }

    delete_old_data_from_pool(&pool, 400, &[(window_start, window_end)])
      .await
      .unwrap();

    let rows = select_all_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    let fans = select_all_fan_covariate_daily_summaries_from_pool(&pool)
      .await
      .unwrap();
    assert_eq!(
      rows.iter().map(|row| row.date).collect::<Vec<_>>(),
      vec![window_start, window_end],
      "both window edges must survive, and the day just outside it must not"
    );
    assert_eq!(
      fans.iter().map(|row| row.date).collect::<Vec<_>>(),
      vec![window_start, window_end]
    );
  }
}
