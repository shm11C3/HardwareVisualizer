//! `cooling_daily_summary` reads and writes.
//!
//! Mirrors the `_from_pool` split used by `archive_queries`: the public
//! `async fn`s resolve Core's process-wide pool via [`db::get_pool`], and
//! delegate to a `_from_pool` variant that takes an explicit `SqlitePool`
//! so tests can exercise the query logic against an in-memory database
//! without touching the process-wide `db::init` `OnceLock`.

use super::archive_queries::{sqlite_epoch_milliseconds, sqlite_epoch_milliseconds_of};
use super::db;
use crate::persistence::cooling_baseline::DailyIdleSample;
use crate::persistence::cooling_rollup::{
  AmbientDeltaSummary, ArchiveMinuteSample, BandSummary, DailyCoolingSummary,
  PowerSummary,
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
  // Decoded through sqlx's own chrono codec for the same reason
  // `earliest_archived_timestamp` does: the column's exact TEXT shape is
  // whatever `hardware_archive::insert`'s native `DateTime<Utc>` bind
  // produced.
  timestamp: DateTime<Utc>,
  cpu_avg: Option<f64>,
  cpu_temperature_avg: Option<f64>,
  cpu_temperature_max: Option<f64>,
  cpu_temperature_min: Option<f64>,
  // The CPU package-domain power columns (#2021). `cpu_power_*` is the
  // column `PowerDraw::cpu_watts` is archived into on every platform that
  // publishes one, so the cooling power lane needs no per-platform branch.
  cpu_power_avg: Option<f64>,
  cpu_power_max: Option<f64>,
  cpu_power_min: Option<f64>,
  // This minute's paired ambient temperature (#2045), or NULL when the
  // minute has no ambient row at all. See
  // `select_archive_minutes_for_range_from_pool` for how the pairing is
  // resolved.
  ambient_temperature_avg: Option<f64>,
}

impl From<ArchiveMinuteRow> for ArchiveMinuteSample {
  fn from(row: ArchiveMinuteRow) -> Self {
    Self {
      timestamp: row.timestamp,
      cpu_usage_avg: row.cpu_avg.map(|v| v as f32),
      cpu_temperature_avg: row.cpu_temperature_avg.map(|v| v as f32),
      cpu_temperature_max: row.cpu_temperature_max.map(|v| v as f32),
      cpu_temperature_min: row.cpu_temperature_min.map(|v| v as f32),
      cpu_power_avg: row.cpu_power_avg.map(|v| v as f32),
      cpu_power_max: row.cpu_power_max.map(|v| v as f32),
      cpu_power_min: row.cpu_power_min.map(|v| v as f32),
      ambient_temperature_avg: row.ambient_temperature_avg.map(|v| v as f32),
    }
  }
}

/// SQL fragment bucketing an epoch-millisecond expression to the minute it
/// falls in. The archive tick stamps a hardware row and every ambient row
/// of the same write cycle with one identical instant, so exact equality
/// would very nearly work - but bucketing to the minute is what the join
/// actually means, and it keeps the pairing correct if a future writer
/// ever stamps the two sides a few milliseconds apart.
fn sqlite_minute_key(epoch_milliseconds: &str) -> String {
  format!("({epoch_milliseconds} / 60000)")
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
  let epoch_ms = sqlite_epoch_milliseconds_of("DATA_ARCHIVE.timestamp");
  let ambient_epoch_ms = sqlite_epoch_milliseconds_of("AMBIENT_ARCHIVE.timestamp");
  let minute_key = sqlite_minute_key(&epoch_ms);
  let ambient_minute_key = sqlite_minute_key(&ambient_epoch_ms);
  // The ambient side is pre-aggregated *within each minute* and then
  // LEFT JOINed, which is what makes #2045's "pair samples first,
  // aggregate second" rule structural: the row this returns carries one
  // ambient value belonging to the same minute as its CPU readings, so no
  // later step is able to subtract two summaries built over different
  // sample sets. A minute with no ambient row keeps a NULL here and
  // produces no ΔT downstream - never an interpolated one.
  //
  // `AVG(temperature)` across the minute's sources is the multi-source
  // rule: `AMBIENT_ARCHIVE` is row-per-source, so a room with two sensors
  // contributes two rows for one minute. An unweighted mean is the
  // neutral choice for the MVP - Core has no ranking, calibration
  // confidence, or "primary sensor" preference to justify favouring one
  // label over another, and inventing one would be a product decision
  // this rollup is not the place to make. With a single source (the
  // common case) it degenerates to that source's own reading.
  let sql = format!(
    "SELECT
       DATA_ARCHIVE.timestamp AS timestamp,
       CAST(DATA_ARCHIVE.cpu_avg AS REAL) AS cpu_avg,
       CAST(DATA_ARCHIVE.cpu_temperature_avg AS REAL) AS cpu_temperature_avg,
       CAST(DATA_ARCHIVE.cpu_temperature_max AS REAL) AS cpu_temperature_max,
       CAST(DATA_ARCHIVE.cpu_temperature_min AS REAL) AS cpu_temperature_min,
       CAST(DATA_ARCHIVE.cpu_power_avg AS REAL) AS cpu_power_avg,
       CAST(DATA_ARCHIVE.cpu_power_max AS REAL) AS cpu_power_max,
       CAST(DATA_ARCHIVE.cpu_power_min AS REAL) AS cpu_power_min,
       ambient.ambient_temperature_avg AS ambient_temperature_avg
     FROM DATA_ARCHIVE
     LEFT JOIN (
       SELECT {ambient_minute_key} AS ambient_minute_key,
              AVG(CAST(AMBIENT_ARCHIVE.temperature AS REAL)) AS ambient_temperature_avg
       FROM AMBIENT_ARCHIVE
       WHERE {ambient_epoch_ms} >= $1 AND {ambient_epoch_ms} < $2
       GROUP BY ambient_minute_key
     ) AS ambient ON ambient.ambient_minute_key = {minute_key}
     WHERE {epoch_ms} >= $1 AND {epoch_ms} < $2
     ORDER BY DATA_ARCHIVE.timestamp ASC"
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

pub(crate) async fn select_daily_idle_samples_from_pool(
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

/// Every summarized day's full band breakdown, oldest first, for Cooling
/// Insight's long-range trend and load-band comparison queries (#2017).
/// Reads the whole table - see [`select_daily_idle_samples`] for why that
/// is cheap enough not to need a bounded `WHERE date >= ...` clause.
pub async fn select_all_daily_cooling_summaries()
-> Result<Vec<DailyCoolingSummary>, sqlx::Error> {
  let pool = db::get_pool().await?;
  select_all_daily_cooling_summaries_from_pool(&pool).await
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::FromRow)]
struct DailyCoolingSummaryRow {
  date: NaiveDate,
  idle_cpu_temperature_avg: Option<f64>,
  idle_cpu_temperature_max: Option<f64>,
  idle_cpu_temperature_min: Option<f64>,
  idle_sample_minutes: i64,
  low_cpu_temperature_avg: Option<f64>,
  low_cpu_temperature_max: Option<f64>,
  low_cpu_temperature_min: Option<f64>,
  low_sample_minutes: i64,
  mid_cpu_temperature_avg: Option<f64>,
  mid_cpu_temperature_max: Option<f64>,
  mid_cpu_temperature_min: Option<f64>,
  mid_sample_minutes: i64,
  high_cpu_temperature_avg: Option<f64>,
  high_cpu_temperature_max: Option<f64>,
  high_cpu_temperature_min: Option<f64>,
  high_sample_minutes: i64,
  coverage_minutes: i64,
  cpu_power_avg: Option<f64>,
  cpu_power_max: Option<f64>,
  cpu_power_min: Option<f64>,
  power_sample_minutes: i64,
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
  ambient_coverage_minutes: i64,
}

impl From<DailyCoolingSummaryRow> for DailyCoolingSummary {
  fn from(row: DailyCoolingSummaryRow) -> Self {
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
        // Same defensive clamp as `select_daily_idle_samples`: the
        // column is `NOT NULL DEFAULT 0` and only ever written from a
        // `u32`.
        sample_minutes: minutes.max(0) as u32,
      }
    }

    Self {
      date: row.date,
      coverage_minutes: row.coverage_minutes.max(0) as u32,
      idle: band(
        row.idle_cpu_temperature_avg,
        row.idle_cpu_temperature_max,
        row.idle_cpu_temperature_min,
        row.idle_sample_minutes,
      ),
      low: band(
        row.low_cpu_temperature_avg,
        row.low_cpu_temperature_max,
        row.low_cpu_temperature_min,
        row.low_sample_minutes,
      ),
      mid: band(
        row.mid_cpu_temperature_avg,
        row.mid_cpu_temperature_max,
        row.mid_cpu_temperature_min,
        row.mid_sample_minutes,
      ),
      high: band(
        row.high_cpu_temperature_avg,
        row.high_cpu_temperature_max,
        row.high_cpu_temperature_min,
        row.high_sample_minutes,
      ),
      power: PowerSummary {
        avg: row.cpu_power_avg.map(|v| v as f32),
        max: row.cpu_power_max.map(|v| v as f32),
        min: row.cpu_power_min.map(|v| v as f32),
        sample_minutes: row.power_sample_minutes.max(0) as u32,
      },
      ambient: AmbientDeltaSummary {
        coverage_minutes: row.ambient_coverage_minutes.max(0) as u32,
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
      },
    }
  }
}

pub(crate) async fn select_all_daily_cooling_summaries_from_pool(
  pool: &SqlitePool,
) -> Result<Vec<DailyCoolingSummary>, sqlx::Error> {
  let rows = sqlx::query_as::<_, DailyCoolingSummaryRow>(
    "SELECT date,
       idle_cpu_temperature_avg, idle_cpu_temperature_max, idle_cpu_temperature_min, idle_sample_minutes,
       low_cpu_temperature_avg, low_cpu_temperature_max, low_cpu_temperature_min, low_sample_minutes,
       mid_cpu_temperature_avg, mid_cpu_temperature_max, mid_cpu_temperature_min, mid_sample_minutes,
       high_cpu_temperature_avg, high_cpu_temperature_max, high_cpu_temperature_min, high_sample_minutes,
       coverage_minutes,
       cpu_power_avg, cpu_power_max, cpu_power_min, power_sample_minutes,
       idle_delta_temperature_avg, idle_delta_temperature_max, idle_delta_temperature_min, idle_delta_sample_minutes,
       low_delta_temperature_avg, low_delta_temperature_max, low_delta_temperature_min, low_delta_sample_minutes,
       mid_delta_temperature_avg, mid_delta_temperature_max, mid_delta_temperature_min, mid_delta_sample_minutes,
       high_delta_temperature_avg, high_delta_temperature_max, high_delta_temperature_min, high_delta_sample_minutes,
       ambient_coverage_minutes
     FROM cooling_daily_summary
     ORDER BY date ASC",
  )
  .fetch_all(pool)
  .await?;

  Ok(rows.into_iter().map(DailyCoolingSummary::from).collect())
}

pub async fn upsert(summary: &DailyCoolingSummary) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  upsert_from_pool(&pool, summary).await
}

async fn upsert_from_pool(
  pool: &SqlitePool,
  summary: &DailyCoolingSummary,
) -> Result<(), sqlx::Error> {
  upsert_with(pool, summary).await
}

/// [`upsert`] against any executor, so the daily and hourly writes for one
/// rolled-up day can share a transaction (see
/// `cooling_rollup::persist_day_rollup_from_pool`). A half-written day -
/// daily present, hourly missing - would otherwise be invisible to the
/// catch-up cursor's consistency check.
pub(crate) async fn upsert_with<'e, E>(
  executor: E,
  summary: &DailyCoolingSummary,
) -> Result<(), sqlx::Error>
where
  E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
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
      coverage_minutes,
      cpu_power_avg, cpu_power_max, cpu_power_min, power_sample_minutes,
      idle_delta_temperature_avg, idle_delta_temperature_max, idle_delta_temperature_min, idle_delta_sample_minutes,
      low_delta_temperature_avg, low_delta_temperature_max, low_delta_temperature_min, low_delta_sample_minutes,
      mid_delta_temperature_avg, mid_delta_temperature_max, mid_delta_temperature_min, mid_delta_sample_minutes,
      high_delta_temperature_avg, high_delta_temperature_max, high_delta_temperature_min, high_delta_sample_minutes,
      ambient_coverage_minutes
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22,
            $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39)
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
      coverage_minutes = excluded.coverage_minutes,
      cpu_power_avg = excluded.cpu_power_avg,
      cpu_power_max = excluded.cpu_power_max,
      cpu_power_min = excluded.cpu_power_min,
      power_sample_minutes = excluded.power_sample_minutes,
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
      high_delta_sample_minutes = excluded.high_delta_sample_minutes,
      ambient_coverage_minutes = excluded.ambient_coverage_minutes
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
  .bind(summary.power.avg)
  .bind(summary.power.max)
  .bind(summary.power.min)
  .bind(summary.power.sample_minutes as i64)
  .bind(summary.ambient.idle.avg)
  .bind(summary.ambient.idle.max)
  .bind(summary.ambient.idle.min)
  .bind(minutes(&summary.ambient.idle))
  .bind(summary.ambient.low.avg)
  .bind(summary.ambient.low.max)
  .bind(summary.ambient.low.min)
  .bind(minutes(&summary.ambient.low))
  .bind(summary.ambient.mid.avg)
  .bind(summary.ambient.mid.max)
  .bind(summary.ambient.mid.min)
  .bind(minutes(&summary.ambient.mid))
  .bind(summary.ambient.high.avg)
  .bind(summary.ambient.high.max)
  .bind(summary.ambient.high.min)
  .bind(minutes(&summary.ambient.high))
  .bind(summary.ambient.coverage_minutes as i64)
  .execute(executor)
  .await?;

  Ok(())
}

/// The latest summarized day that recorded at least one minute carrying
/// both a CPU usage and a CPU temperature reading.
///
/// By `summarize_day`'s own rule a band only accrues `sample_minutes` for
/// such a minute, so a day with any band samples is exactly a day the
/// hourly rollup must also have produced a row for. That equivalence is
/// what lets the catch-up cursor tell "hourly is behind" apart from
/// "there was never anything for hourly to record" - see
/// `cooling_rollup::rollup_catch_up_cursor`.
pub async fn max_pairable_summarized_date() -> Result<Option<NaiveDate>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_pairable_summarized_date_from_pool(&pool).await
}

pub(crate) async fn max_pairable_summarized_date_from_pool(
  pool: &SqlitePool,
) -> Result<Option<NaiveDate>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<NaiveDate>>(
    "SELECT MAX(date) FROM cooling_daily_summary
     WHERE idle_sample_minutes + low_sample_minutes
         + mid_sample_minutes + high_sample_minutes > 0",
  )
  .fetch_one(pool)
  .await
}

/// The latest summarized day that recorded any CPU package power (#2021).
///
/// Paired with [`max_powered_archive_timestamp_before`] this is how the
/// catch-up detects that the daily rollup's power columns are behind the
/// archive - see `cooling_rollup::power_rollup_is_behind`.
pub async fn max_powered_summarized_date() -> Result<Option<NaiveDate>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_powered_summarized_date_from_pool(&pool).await
}

pub(crate) async fn max_powered_summarized_date_from_pool(
  pool: &SqlitePool,
) -> Result<Option<NaiveDate>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<NaiveDate>>(
    "SELECT MAX(date) FROM cooling_daily_summary WHERE power_sample_minutes > 0",
  )
  .fetch_one(pool)
  .await
}

/// The most recent archived timestamp strictly before `before` whose row
/// carries a full CPU package power triple (#2021).
///
/// `before` is the start of today in local time, so the answer only ever
/// names a *completed* day. The rollup never summarizes today, so today's
/// archived power is not evidence that a day was missed - counting it
/// would make a machine that is recording power right now rewind the
/// catch-up on every cycle, forever.
///
/// All three columns are required, matching `summarize_day`'s own power
/// gate: a partial triple contributes nothing there, so it must not count
/// as power the rollup failed to pick up either.
pub async fn max_powered_archive_timestamp_before(
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_powered_archive_timestamp_before_from_pool(&pool, before).await
}

pub(crate) async fn max_powered_archive_timestamp_before_from_pool(
  pool: &SqlitePool,
  before: &DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
  // Same epoch-millisecond comparison as
  // `select_archive_minutes_for_range_from_pool`, and for the same reason:
  // the bind value's TEXT shape is not guaranteed to sort against the
  // written column at exact-second boundaries.
  let epoch_ms = sqlite_epoch_milliseconds();
  let sql = format!(
    "SELECT MAX(timestamp) FROM DATA_ARCHIVE
     WHERE {epoch_ms} < $1
       AND cpu_power_avg IS NOT NULL
       AND cpu_power_max IS NOT NULL
       AND cpu_power_min IS NOT NULL"
  );
  sqlx::query_scalar::<_, Option<DateTime<Utc>>>(&sql)
    .bind(before.timestamp_millis())
    .fetch_one(pool)
    .await
}

/// The latest summarized day that recorded any ambient coverage (#2045).
///
/// Paired with [`max_pairable_ambient_archive_timestamp_before`] this is
/// how the catch-up detects that the daily rollup's ambient delta columns
/// are behind the archives - see
/// `cooling_rollup::ambient_rollup_is_behind`.
pub async fn max_ambient_summarized_date() -> Result<Option<NaiveDate>, sqlx::Error> {
  let pool = db::get_pool().await?;
  max_ambient_summarized_date_from_pool(&pool).await
}

pub(crate) async fn max_ambient_summarized_date_from_pool(
  pool: &SqlitePool,
) -> Result<Option<NaiveDate>, sqlx::Error> {
  sqlx::query_scalar::<_, Option<NaiveDate>>(
    "SELECT MAX(date) FROM cooling_daily_summary WHERE ambient_coverage_minutes > 0",
  )
  .fetch_one(pool)
  .await
}

/// The most recent ambient archive timestamp strictly before `before`
/// whose minute also has a `DATA_ARCHIVE` row (#2045).
///
/// Both bounds matter, and both mirror
/// [`max_powered_archive_timestamp_before`]:
///
/// `before` is the start of today in local time, so the answer only ever
/// names a *completed* day. The rollup never summarizes today, so today's
/// ambient rows are not evidence that a day was missed - counting them
/// would make a machine that is recording ambient right now rewind the
/// catch-up on every cycle, forever.
///
/// The `EXISTS` clause matches `summarize_day`'s own ambient-coverage
/// gate: coverage is counted per *archived minute* that had an ambient
/// pair, so an ambient row whose minute has no hardware row can never
/// become coverage however often the day is re-rolled. Counting it would
/// send the catch-up chasing a day it can never fill.
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
  let sql = format!(
    "SELECT MAX(AMBIENT_ARCHIVE.timestamp) FROM AMBIENT_ARCHIVE
     WHERE {ambient_epoch_ms} < $1
       AND EXISTS (
         SELECT 1 FROM DATA_ARCHIVE
         WHERE {hardware_minute_key} = {ambient_minute_key}
       )"
  );
  sqlx::query_scalar::<_, Option<DateTime<Utc>>>(&sql)
    .bind(before.timestamp_millis())
    .fetch_one(pool)
    .await
}

pub async fn delete_old_data(
  retention_days: u32,
  preserved_window: Option<(NaiveDate, NaiveDate)>,
) -> Result<(), sqlx::Error> {
  let pool = db::get_pool().await?;
  delete_old_data_from_pool(&pool, retention_days, preserved_window).await
}

/// Delete rows older than `retention_days`, except those inside
/// `preserved_window`.
///
/// `preserved_window` is the pinned baseline's calendar window. Once that
/// window ages past the retention cutoff, deleting its rows would leave
/// every baseline-side comparison permanently empty while the pinned
/// baseline itself still claims that period as the reference - so the
/// window is exempt. It is at most a week of rows, kept for as long as the
/// baseline it backs.
pub(crate) async fn delete_old_data_from_pool(
  pool: &SqlitePool,
  retention_days: u32,
  preserved_window: Option<(NaiveDate, NaiveDate)>,
) -> Result<(), sqlx::Error> {
  // Same cutoff style as `storage_health::delete_old_data`: a local-date
  // TEXT comparison, since `date` is stored as "%Y-%m-%d" and compares
  // lexicographically the same as chronologically.
  let cutoff = (chrono::Local::now().date_naive()
    - chrono::Duration::days(retention_days as i64))
  .format("%Y-%m-%d")
  .to_string();

  match preserved_window {
    Some((start, end)) => {
      sqlx::query(
        "DELETE FROM cooling_daily_summary
         WHERE date < $1 AND NOT (date >= $2 AND date <= $3)",
      )
      .bind(cutoff)
      .bind(start.format("%Y-%m-%d").to_string())
      .bind(end.format("%Y-%m-%d").to_string())
      .execute(pool)
      .await?;
    }
    None => {
      sqlx::query("DELETE FROM cooling_daily_summary WHERE date < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::Row;

  use super::super::test_schema::{
    AMBIENT_ARCHIVE_DDL, COOLING_DAILY_SUMMARY_DDL, DATA_ARCHIVE_DDL, create_tables,
  };

  /// The hardware archive plus the ambient archive the minute query
  /// LEFT JOINs against (#2045). Both are created together because the
  /// join names `AMBIENT_ARCHIVE` unconditionally: an install with no
  /// environmental sensor has an empty table, never a missing one.
  async fn setup_data_archive(pool: &SqlitePool) {
    create_tables(pool, &[DATA_ARCHIVE_DDL, AMBIENT_ARCHIVE_DDL]).await;
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

  async fn setup_cooling_daily_summary(pool: &SqlitePool) {
    create_tables(pool, &[COOLING_DAILY_SUMMARY_DDL]).await;
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
  async fn select_archive_minutes_carries_each_rows_own_timestamp() {
    // The hourly rollup buckets these rows by their instant, so a lost or
    // truncated timestamp would silently collapse a day into one hour.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_archive_row(
      &pool,
      Some(6.0),
      Some(41.0),
      utc("2026-08-15T09:17:00.000Z"),
    )
    .await;
    insert_archive_row(
      &pool,
      Some(7.0),
      Some(42.0),
      utc("2026-08-15T22:43:00.000Z"),
    )
    .await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(
      rows.iter().map(|r| r.timestamp).collect::<Vec<_>>(),
      vec![
        utc("2026-08-15T09:17:00.000Z"),
        utc("2026-08-15T22:43:00.000Z")
      ]
    );
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

  async fn insert_full_summary_row(
    pool: &SqlitePool,
    date: &str,
    idle: BandSummary,
    low: BandSummary,
    mid: BandSummary,
    high: BandSummary,
    coverage_minutes: i64,
  ) {
    sqlx::query(
      "INSERT INTO cooling_daily_summary (
         date,
         idle_cpu_temperature_avg, idle_cpu_temperature_max, idle_cpu_temperature_min, idle_sample_minutes,
         low_cpu_temperature_avg, low_cpu_temperature_max, low_cpu_temperature_min, low_sample_minutes,
         mid_cpu_temperature_avg, mid_cpu_temperature_max, mid_cpu_temperature_min, mid_sample_minutes,
         high_cpu_temperature_avg, high_cpu_temperature_max, high_cpu_temperature_min, high_sample_minutes,
         coverage_minutes
       ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
    )
    .bind(date)
    .bind(idle.avg)
    .bind(idle.max)
    .bind(idle.min)
    .bind(idle.sample_minutes as i64)
    .bind(low.avg)
    .bind(low.max)
    .bind(low.min)
    .bind(low.sample_minutes as i64)
    .bind(mid.avg)
    .bind(mid.max)
    .bind(mid.min)
    .bind(mid.sample_minutes as i64)
    .bind(high.avg)
    .bind(high.max)
    .bind(high.min)
    .bind(high.sample_minutes as i64)
    .bind(coverage_minutes)
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn select_all_daily_cooling_summaries_returns_every_band_in_ascending_date_order()
  {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    // Inserted out of order to verify the `ORDER BY date ASC` clause.
    insert_full_summary_row(
      &pool,
      "2026-08-20",
      full_band(30.0, 600),
      full_band(40.0, 300),
      empty_band(),
      full_band(70.0, 100),
      1000,
    )
    .await;
    insert_full_summary_row(
      &pool,
      "2026-08-10",
      full_band(28.0, 1440),
      empty_band(),
      empty_band(),
      empty_band(),
      1440,
    )
    .await;

    let summaries = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(
      summaries.iter().map(|s| s.date).collect::<Vec<_>>(),
      vec![date(2026, 8, 10), date(2026, 8, 20)]
    );
    assert_eq!(summaries[1].idle, full_band(30.0, 600));
    assert_eq!(summaries[1].low, full_band(40.0, 300));
    assert_eq!(summaries[1].mid, empty_band());
    assert_eq!(summaries[1].high, full_band(70.0, 100));
    assert_eq!(summaries[1].coverage_minutes, 1000);
  }

  #[tokio::test]
  async fn select_all_daily_cooling_summaries_is_empty_for_an_empty_rollup() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    assert_eq!(
      select_all_daily_cooling_summaries_from_pool(&pool)
        .await
        .unwrap(),
      Vec::new()
    );
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
      power: PowerSummary {
        avg: Some(18.5),
        max: Some(42.0),
        min: Some(4.5),
        sample_minutes: 950,
      },
      ambient: AmbientDeltaSummary::default(),
    };
    upsert_from_pool(&pool, &summary).await.unwrap();

    let row = sqlx::query(
      "SELECT idle_cpu_temperature_avg, idle_sample_minutes, mid_cpu_temperature_avg, mid_sample_minutes, coverage_minutes,
              cpu_power_avg, cpu_power_max, cpu_power_min, power_sample_minutes
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
    assert_eq!(row.get::<f64, _>("cpu_power_avg"), 18.5);
    assert_eq!(row.get::<f64, _>("cpu_power_max"), 42.0);
    assert_eq!(row.get::<f64, _>("cpu_power_min"), 4.5);
    assert_eq!(row.get::<i64, _>("power_sample_minutes"), 950);
  }

  #[tokio::test]
  async fn a_day_without_power_readings_reads_back_absent_rather_than_zero() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    upsert_from_pool(
      &pool,
      &DailyCoolingSummary {
        date: date(2026, 8, 15),
        coverage_minutes: 1000,
        idle: full_band(30.0, 600),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
        ambient: AmbientDeltaSummary::default(),
      },
    )
    .await
    .unwrap();

    let days = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(days.len(), 1);
    assert_eq!(days[0].power, PowerSummary::default());
    assert_eq!(days[0].power.avg, None);
  }

  #[tokio::test]
  async fn a_persisted_power_summary_round_trips_through_the_daily_read() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    let power = PowerSummary {
      avg: Some(21.5),
      max: Some(55.0),
      min: Some(3.25),
      sample_minutes: 1200,
    };
    upsert_from_pool(
      &pool,
      &DailyCoolingSummary {
        date: date(2026, 8, 15),
        coverage_minutes: 1440,
        // A machine that reports power but has no temperature sensor
        // keeps its power series: the two capabilities are independent.
        idle: empty_band(),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power,
        ambient: AmbientDeltaSummary::default(),
      },
    )
    .await
    .unwrap();

    let days = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(days[0].power, power);
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
      power: PowerSummary::default(),
      ambient: AmbientDeltaSummary::default(),
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

    delete_old_data_from_pool(&pool, 400, None).await.unwrap();

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

  #[tokio::test]
  async fn delete_old_data_keeps_the_pinned_baseline_window_past_the_cutoff() {
    // The pinned baseline never expires, so once its window ages past the
    // retention cutoff its rows must still survive - otherwise every
    // baseline-side comparison goes permanently empty while the baseline
    // still names that period as the reference.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    let today = chrono::Local::now().date_naive();
    let window_start = today - chrono::Duration::days(500);
    let window_end = window_start + chrono::Duration::days(6);
    let outside_window = window_start - chrono::Duration::days(1);

    for d in [outside_window, window_start, window_end] {
      sqlx::query(
        "INSERT INTO cooling_daily_summary (date, coverage_minutes) VALUES ($1, 0)",
      )
      .bind(d.format("%Y-%m-%d").to_string())
      .execute(&pool)
      .await
      .unwrap();
    }

    delete_old_data_from_pool(&pool, 400, Some((window_start, window_end)))
      .await
      .unwrap();

    let remaining: Vec<String> =
      sqlx::query_scalar("SELECT date FROM cooling_daily_summary ORDER BY date")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(
      remaining,
      vec![
        window_start.format("%Y-%m-%d").to_string(),
        window_end.format("%Y-%m-%d").to_string(),
      ],
      "both window edges must survive, and the day just outside it must not"
    );
  }

  #[tokio::test]
  async fn max_pairable_summarized_date_ignores_days_that_recorded_no_pair() {
    // A machine with no CPU temperature sensor accrues coverage but no
    // band samples. Those days are not evidence that the hourly rollup
    // fell behind.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    insert_full_summary_row(
      &pool,
      "2026-08-10",
      full_band(30.0, 120),
      empty_band(),
      empty_band(),
      empty_band(),
      1440,
    )
    .await;
    // Later day, recorded but with no usable pair in any band.
    sqlx::query(
      "INSERT INTO cooling_daily_summary (date, coverage_minutes) VALUES ('2026-08-20', 1440)",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
      max_pairable_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 10))
    );
    assert_eq!(
      max_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 20)),
      "the plain cursor still tracks the latest summarized day"
    );
  }

  #[tokio::test]
  async fn max_pairable_summarized_date_is_none_when_no_day_ever_paired() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    sqlx::query(
      "INSERT INTO cooling_daily_summary (date, coverage_minutes) VALUES ('2026-08-20', 1440)",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
      max_pairable_summarized_date_from_pool(&pool).await.unwrap(),
      None
    );
  }

  // ── CPU package power backfill cursor (#2021) ──

  /// One archived minute carrying a full CPU package power triple, or -
  /// with `power` as `None` - one recorded with no power reading at all.
  async fn insert_powered_archive_row(
    pool: &SqlitePool,
    power: Option<f64>,
    timestamp: DateTime<Utc>,
  ) {
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE
         (cpu_avg, cpu_temperature_avg, cpu_temperature_max, cpu_temperature_min,
          cpu_power_avg, cpu_power_max, cpu_power_min, timestamp)
       VALUES (5.0, 40.0, 41.0, 39.0, $1, $1, $1, $2)",
    )
    .bind(power)
    .bind(timestamp)
    .execute(pool)
    .await
    .unwrap();
  }

  #[tokio::test]
  async fn max_powered_summarized_date_ignores_days_that_recorded_no_power() {
    // The state migration 14 leaves behind: rows exist, their power
    // columns are NULL, and `power_sample_minutes` defaulted to 0.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    upsert_from_pool(
      &pool,
      &DailyCoolingSummary {
        date: date(2026, 8, 10),
        coverage_minutes: 1440,
        idle: full_band(30.0, 600),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary {
          avg: Some(18.0),
          max: Some(30.0),
          min: Some(5.0),
          sample_minutes: 900,
        },
        ambient: AmbientDeltaSummary::default(),
      },
    )
    .await
    .unwrap();
    insert_full_summary_row(
      &pool,
      "2026-08-20",
      full_band(30.0, 600),
      empty_band(),
      empty_band(),
      empty_band(),
      1440,
    )
    .await;

    assert_eq!(
      max_powered_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 10)),
      "a NULL-power row must not advance the power cursor past the last real reading"
    );
  }

  #[tokio::test]
  async fn max_powered_summarized_date_is_none_when_no_day_recorded_power() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    insert_full_summary_row(
      &pool,
      "2026-08-20",
      full_band(30.0, 600),
      empty_band(),
      empty_band(),
      empty_band(),
      1440,
    )
    .await;

    assert_eq!(
      max_powered_summarized_date_from_pool(&pool).await.unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn max_powered_archive_timestamp_only_sees_rows_before_the_bound() {
    // The bound is the start of today: today's archived power must not
    // count, or a machine recording power right now would look
    // permanently behind and rewind the catch-up on every cycle.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_powered_archive_row(&pool, Some(18.0), utc("2026-08-19T23:59:59.000Z")).await;
    insert_powered_archive_row(&pool, Some(22.0), utc("2026-08-20T00:00:00.000Z")).await;

    let latest = max_powered_archive_timestamp_before_from_pool(
      &pool,
      &utc("2026-08-20T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(latest, Some(utc("2026-08-19T23:59:59.000Z")));
  }

  #[tokio::test]
  async fn max_powered_archive_timestamp_is_none_without_a_power_source() {
    // Rows were archived, but the platform publishes no CPU power. This
    // is what stops the backfill check from firing forever.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_powered_archive_row(&pool, None, utc("2026-08-19T10:00:00.000Z")).await;

    assert_eq!(
      max_powered_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-20T00:00:00.000Z")
      )
      .await
      .unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn max_powered_archive_timestamp_skips_an_incomplete_power_triple() {
    // `summarize_day` folds nothing from a partial triple, so it must not
    // count as power the rollup failed to pick up either - otherwise the
    // catch-up would rewind chasing a day it can never fill.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (cpu_avg, cpu_power_avg, timestamp)
       VALUES (5.0, 18.0, $1)",
    )
    .bind(utc("2026-08-19T10:00:00.000Z"))
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
      max_powered_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-20T00:00:00.000Z")
      )
      .await
      .unwrap(),
      None
    );
  }
  // ── ambient pairing at the read boundary (#2045) ──

  #[tokio::test]
  async fn a_minute_is_paired_with_the_ambient_row_stamped_to_the_same_minute() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    let tick = utc("2026-08-15T12:00:00.000Z");
    insert_archive_row(&pool, Some(5.0), Some(40.0), tick).await;
    insert_ambient_row(&pool, "Living Room", 25.0, tick).await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].ambient_temperature_avg, Some(25.0));
  }

  #[tokio::test]
  async fn several_ambient_sources_in_one_minute_average_unweighted() {
    // `AMBIENT_ARCHIVE` is row-per-source, so a room with two sensors
    // contributes two rows for one minute. The MVP rule is a plain mean:
    // Core has no ranking that would justify preferring one label.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    let tick = utc("2026-08-15T12:00:00.000Z");
    insert_archive_row(&pool, Some(5.0), Some(40.0), tick).await;
    insert_ambient_row(&pool, "Living Room", 24.0, tick).await;
    insert_ambient_row(&pool, "Desk", 28.0, tick).await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows[0].ambient_temperature_avg, Some(26.0));
  }

  #[tokio::test]
  async fn a_minute_with_no_ambient_row_stays_unpaired_rather_than_borrowing_another() {
    // The pairing is per minute. An ambient row an hour away must not
    // fill the gap - that is exactly the interpolation #2045 forbids.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
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

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].ambient_temperature_avg, Some(25.0));
    assert_eq!(rows[1].ambient_temperature_avg, None);
  }

  #[tokio::test]
  async fn an_ambient_row_within_the_same_minute_still_pairs_across_seconds() {
    // The archive tick stamps both sides with one instant, but the join
    // is defined on the minute rather than on exact equality, so a
    // writer that ever drifted by seconds would still pair correctly.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:00.000Z"),
    )
    .await;
    insert_ambient_row(&pool, "Living Room", 25.0, utc("2026-08-15T12:00:59.900Z")).await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows[0].ambient_temperature_avg, Some(25.0));
  }

  #[tokio::test]
  async fn rows_split_across_a_minute_boundary_do_not_pair() {
    // Why the write cycle must stamp every table from one shared instant
    // rather than reading the clock per insert: one second of drift
    // across a minute boundary puts the two rows in different minutes,
    // and the pairing is defined on the minute. The join is right to
    // refuse here - the fix belongs at the writer, which now threads the
    // cycle's `tick_timestamp` into every insert.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:59.900Z"),
    )
    .await;
    insert_ambient_row(&pool, "Living Room", 25.0, utc("2026-08-15T12:01:00.100Z")).await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(
      rows[0].ambient_temperature_avg, None,
      "a reading from the next minute must not stand in for this one"
    );
  }

  #[tokio::test]
  async fn an_archive_with_no_ambient_rows_reads_back_exactly_as_before() {
    // The zero-ambient invariant at the query layer: every existing
    // field keeps its value and the new one is simply absent.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:00.000Z"),
    )
    .await;

    let rows = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cpu_usage_avg, Some(5.0));
    assert_eq!(rows[0].cpu_temperature_avg, Some(40.0));
    assert_eq!(rows[0].ambient_temperature_avg, None);
  }

  // ── ambient backfill cursor (#2045) ──

  #[tokio::test]
  async fn max_ambient_summarized_date_ignores_days_that_recorded_no_coverage() {
    // The state migration 16 leaves behind: rows exist, their delta
    // columns are NULL, and `ambient_coverage_minutes` defaulted to 0.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    upsert_from_pool(
      &pool,
      &DailyCoolingSummary {
        date: date(2026, 8, 10),
        coverage_minutes: 1440,
        idle: full_band(30.0, 600),
        low: empty_band(),
        mid: empty_band(),
        high: empty_band(),
        power: PowerSummary::default(),
        ambient: AmbientDeltaSummary {
          coverage_minutes: 900,
          idle: full_band(12.0, 600),
          ..AmbientDeltaSummary::default()
        },
      },
    )
    .await
    .unwrap();
    insert_full_summary_row(
      &pool,
      "2026-08-20",
      full_band(30.0, 600),
      empty_band(),
      empty_band(),
      empty_band(),
      1440,
    )
    .await;

    assert_eq!(
      max_ambient_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 10)),
      "a zero-coverage row must not advance the ambient cursor past the last real one"
    );
  }

  #[tokio::test]
  async fn max_ambient_summarized_date_is_none_when_no_day_recorded_coverage() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;
    insert_full_summary_row(
      &pool,
      "2026-08-20",
      full_band(30.0, 600),
      empty_band(),
      empty_band(),
      empty_band(),
      1440,
    )
    .await;

    assert_eq!(
      max_ambient_summarized_date_from_pool(&pool).await.unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn max_pairable_ambient_archive_timestamp_only_sees_rows_before_the_bound() {
    // The bound is the start of today: today's ambient rows must not
    // count, or a machine collecting ambient right now would look
    // permanently behind and rewind the catch-up on every cycle.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
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
    setup_data_archive(&pool).await;
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
    // `summarize_day` counts coverage per *archived minute*, so an
    // ambient row whose minute has no hardware row can never become
    // coverage however often the day is re-rolled. Counting it would
    // send the catch-up chasing a day it can never fill.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
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

  // ── archive to daily summary, end to end (#2021) ──

  #[tokio::test]
  async fn archived_power_reaches_the_persisted_daily_summary() {
    // The whole path in one test: real archive rows, the real range read,
    // the real fold, the real write, the real read-back. The unit tests
    // above each cover one hop; this is what catches a column name that
    // only two of the four hops agree on.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    setup_cooling_daily_summary(&pool).await;

    for (minute, power) in [(0, 10.0), (1, 20.0), (2, 30.0)] {
      sqlx::query(
        "INSERT INTO DATA_ARCHIVE
           (cpu_avg, cpu_temperature_avg, cpu_temperature_max, cpu_temperature_min,
            cpu_power_avg, cpu_power_max, cpu_power_min, timestamp)
         VALUES (5.0, 40.0, 41.0, 39.0, $1, $2, $3, $4)",
      )
      .bind(power)
      .bind(power + 2.0)
      .bind(power - 2.0)
      .bind(utc(&format!("2026-08-15T12:0{minute}:00.000Z")))
      .execute(&pool)
      .await
      .unwrap();
    }

    let minutes = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();
    let summary =
      crate::persistence::cooling_rollup::summarize_day(date(2026, 8, 15), &minutes)
        .expect("the day was archived, so it must produce a summary");
    upsert_from_pool(&pool, &summary).await.unwrap();

    let days = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(days.len(), 1);
    assert_eq!(
      days[0].power,
      PowerSummary {
        avg: Some(20.0),
        max: Some(32.0),
        min: Some(8.0),
        sample_minutes: 3,
      }
    );
    // And the backfill cursor now agrees the power columns are current.
    assert_eq!(
      max_powered_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 15))
    );
  }

  #[tokio::test]
  async fn an_archive_without_power_persists_a_daily_summary_with_absent_power() {
    // The same path on a machine with no CPU power source: temperature
    // still lands, power stays absent rather than becoming 0 W, and the
    // backfill cursor reports nothing to catch up.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    setup_cooling_daily_summary(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:00.000Z"),
    )
    .await;

    let minutes = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();
    let summary =
      crate::persistence::cooling_rollup::summarize_day(date(2026, 8, 15), &minutes)
        .unwrap();
    upsert_from_pool(&pool, &summary).await.unwrap();

    let days = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(days[0].idle.avg, Some(40.0));
    assert_eq!(days[0].power, PowerSummary::default());
    assert_eq!(
      max_powered_summarized_date_from_pool(&pool).await.unwrap(),
      None
    );
    assert_eq!(
      max_powered_archive_timestamp_before_from_pool(
        &pool,
        &utc("2026-08-16T00:00:00.000Z")
      )
      .await
      .unwrap(),
      None,
      "neither side has power, so the catch-up must never claim to be behind"
    );
  }

  // ── ambient archives to daily summary, end to end (#2045) ──

  #[tokio::test]
  async fn a_paired_ambient_archive_reaches_the_persisted_daily_summary() {
    // The whole path in one test: real archive rows on both sides, the
    // real join, the real fold, the real write, the real read-back. The
    // unit tests above each cover one hop; this is what catches a column
    // name that only some of the hops agree on.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    setup_cooling_daily_summary(&pool).await;

    // Three idle minutes: CPU 40/44/48 against ambient 20/24/28, so
    // every minute's delta is exactly 20 K even though both series are
    // climbing. That is the point of the whole feature - a rise the room
    // explains leaves the delta flat.
    for (minute, cpu, ambient) in [(0, 40.0, 20.0), (1, 44.0, 24.0), (2, 48.0, 28.0)] {
      let tick = utc(&format!("2026-08-15T12:0{minute}:00.000Z"));
      insert_archive_row(&pool, Some(5.0), Some(cpu), tick).await;
      insert_ambient_row(&pool, "Living Room", ambient, tick).await;
    }
    // A fourth minute the ambient sensor missed: it must raise the
    // absolute band average without touching the delta.
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(100.0),
      utc("2026-08-15T12:03:00.000Z"),
    )
    .await;

    let minutes = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();
    let summary =
      crate::persistence::cooling_rollup::summarize_day(date(2026, 8, 15), &minutes)
        .expect("the day was archived, so it must produce a summary");
    upsert_from_pool(&pool, &summary).await.unwrap();

    let days = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(days.len(), 1);
    assert_eq!(days[0].ambient.coverage_minutes, 3);
    assert_eq!(days[0].ambient.idle.avg, Some(20.0));
    assert_eq!(days[0].ambient.idle.sample_minutes, 3);
    // The unpaired hot minute reached the absolute band and no further.
    assert_eq!(days[0].idle.sample_minutes, 4);
    assert_eq!(days[0].idle.avg, Some(58.0));
    // And the backfill cursor now agrees the ambient columns are current.
    assert_eq!(
      max_ambient_summarized_date_from_pool(&pool).await.unwrap(),
      Some(date(2026, 8, 15))
    );
  }

  #[tokio::test]
  async fn an_archive_without_ambient_persists_a_daily_summary_with_absent_deltas() {
    // The same path on a machine with no environmental sensor:
    // temperature still lands, the deltas stay absent rather than
    // becoming 0 K, and the backfill cursor reports nothing to catch up.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_data_archive(&pool).await;
    setup_cooling_daily_summary(&pool).await;
    insert_archive_row(
      &pool,
      Some(5.0),
      Some(40.0),
      utc("2026-08-15T12:00:00.000Z"),
    )
    .await;

    let minutes = select_archive_minutes_for_range_from_pool(
      &pool,
      &utc("2026-08-15T00:00:00.000Z"),
      &utc("2026-08-16T00:00:00.000Z"),
    )
    .await
    .unwrap();
    let summary =
      crate::persistence::cooling_rollup::summarize_day(date(2026, 8, 15), &minutes)
        .unwrap();
    upsert_from_pool(&pool, &summary).await.unwrap();

    let days = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(days[0].idle.avg, Some(40.0));
    assert_eq!(days[0].ambient, AmbientDeltaSummary::default());
    assert_eq!(
      max_ambient_summarized_date_from_pool(&pool).await.unwrap(),
      None
    );
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

  #[tokio::test]
  async fn a_persisted_ambient_delta_summary_round_trips_through_the_daily_read() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_cooling_daily_summary(&pool).await;

    let ambient = AmbientDeltaSummary {
      coverage_minutes: 1200,
      idle: full_band(12.5, 600),
      low: full_band(20.0, 300),
      mid: BandSummary::default(),
      high: full_band(45.25, 100),
    };
    upsert_from_pool(
      &pool,
      &DailyCoolingSummary {
        date: date(2026, 8, 15),
        coverage_minutes: 1440,
        idle: full_band(30.0, 600),
        low: full_band(40.0, 300),
        mid: empty_band(),
        high: full_band(70.0, 100),
        power: PowerSummary::default(),
        ambient,
      },
    )
    .await
    .unwrap();

    let days = select_all_daily_cooling_summaries_from_pool(&pool)
      .await
      .unwrap();

    assert_eq!(days[0].ambient, ambient);
    assert_eq!(
      days[0].ambient.mid,
      BandSummary::default(),
      "a band with no paired minute stays absent, never 0 K"
    );
  }
}
