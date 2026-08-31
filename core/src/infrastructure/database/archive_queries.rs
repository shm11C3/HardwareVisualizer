use super::db;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::SqlitePool;
use std::fmt;

const MAX_ARCHIVE_SERIES_POINTS: i64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveAggregation {
  Avg,
  Max,
  Min,
}

impl ArchiveAggregation {
  fn sql(self) -> &'static str {
    match self {
      Self::Avg => "AVG",
      Self::Max => "MAX",
      Self::Min => "MIN",
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveBucketTimestamp {
  Start,
  End,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ArchiveSeriesPoint {
  pub timestamp: i64,
  pub value: Option<f64>,
}

#[derive(Debug)]
pub enum ArchiveSeriesError {
  Database(sqlx::Error),
  InvalidTimeRange,
  InvalidBucketWidth,
  TooManyPoints { requested: i64, maximum: i64 },
}

impl fmt::Display for ArchiveSeriesError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Database(error) => write!(formatter, "{error}"),
      Self::InvalidTimeRange => {
        write!(formatter, "archive series start must not exceed end")
      }
      Self::InvalidBucketWidth => {
        write!(formatter, "archive series bucket width must be positive")
      }
      Self::TooManyPoints { requested, maximum } => write!(
        formatter,
        "archive series would contain {requested} points; maximum is {maximum}"
      ),
    }
  }
}

impl std::error::Error for ArchiveSeriesError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Database(error) => Some(error),
      _ => None,
    }
  }
}

impl From<sqlx::Error> for ArchiveSeriesError {
  fn from(error: sqlx::Error) -> Self {
    Self::Database(error)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataArchiveColumn {
  CpuAvg,
  CpuMax,
  CpuMin,
  CpuTemperatureAvg,
  CpuTemperatureMax,
  CpuTemperatureMin,
  CpuPowerAvg,
  CpuPowerMax,
  CpuPowerMin,
  GpuPowerAvg,
  GpuPowerMax,
  GpuPowerMin,
  AnePowerAvg,
  AnePowerMax,
  AnePowerMin,
  PackagePowerAvg,
  PackagePowerMax,
  PackagePowerMin,
  RamAvg,
  RamMax,
  RamMin,
}

impl DataArchiveColumn {
  fn sql(self) -> &'static str {
    match self {
      Self::CpuAvg => "cpu_avg",
      Self::CpuMax => "cpu_max",
      Self::CpuMin => "cpu_min",
      Self::CpuTemperatureAvg => "cpu_temperature_avg",
      Self::CpuTemperatureMax => "cpu_temperature_max",
      Self::CpuTemperatureMin => "cpu_temperature_min",
      Self::CpuPowerAvg => "cpu_power_avg",
      Self::CpuPowerMax => "cpu_power_max",
      Self::CpuPowerMin => "cpu_power_min",
      Self::GpuPowerAvg => "gpu_power_avg",
      Self::GpuPowerMax => "gpu_power_max",
      Self::GpuPowerMin => "gpu_power_min",
      Self::AnePowerAvg => "ane_power_avg",
      Self::AnePowerMax => "ane_power_max",
      Self::AnePowerMin => "ane_power_min",
      Self::PackagePowerAvg => "package_power_avg",
      Self::PackagePowerMax => "package_power_max",
      Self::PackagePowerMin => "package_power_min",
      Self::RamAvg => "ram_avg",
      Self::RamMax => "ram_max",
      Self::RamMin => "ram_min",
    }
  }

  fn aggregation(self) -> ArchiveAggregation {
    match self {
      Self::CpuAvg
      | Self::CpuTemperatureAvg
      | Self::CpuPowerAvg
      | Self::GpuPowerAvg
      | Self::AnePowerAvg
      | Self::PackagePowerAvg
      | Self::RamAvg => ArchiveAggregation::Avg,
      Self::CpuMax
      | Self::CpuTemperatureMax
      | Self::CpuPowerMax
      | Self::GpuPowerMax
      | Self::AnePowerMax
      | Self::PackagePowerMax
      | Self::RamMax => ArchiveAggregation::Max,
      Self::CpuMin
      | Self::CpuTemperatureMin
      | Self::CpuPowerMin
      | Self::GpuPowerMin
      | Self::AnePowerMin
      | Self::PackagePowerMin
      | Self::RamMin => ArchiveAggregation::Min,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuArchiveColumn {
  UsageAvg,
  UsageMax,
  UsageMin,
  TemperatureAvg,
  TemperatureMax,
  TemperatureMin,
  DedicatedMemoryAvg,
  DedicatedMemoryMax,
  DedicatedMemoryMin,
}

impl GpuArchiveColumn {
  fn sql(self) -> &'static str {
    match self {
      Self::UsageAvg => "usage_avg",
      Self::UsageMax => "usage_max",
      Self::UsageMin => "usage_min",
      Self::TemperatureAvg => "temperature_avg",
      Self::TemperatureMax => "temperature_max",
      Self::TemperatureMin => "temperature_min",
      Self::DedicatedMemoryAvg => "dedicated_memory_avg",
      Self::DedicatedMemoryMax => "dedicated_memory_max",
      Self::DedicatedMemoryMin => "dedicated_memory_min",
    }
  }

  fn aggregation(self) -> ArchiveAggregation {
    match self {
      Self::UsageAvg | Self::TemperatureAvg | Self::DedicatedMemoryAvg => {
        ArchiveAggregation::Avg
      }
      Self::UsageMax | Self::TemperatureMax | Self::DedicatedMemoryMax => {
        ArchiveAggregation::Max
      }
      Self::UsageMin | Self::TemperatureMin | Self::DedicatedMemoryMin => {
        ArchiveAggregation::Min
      }
    }
  }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ArchiveRecord {
  pub id: i64,
  pub value: Option<f64>,
  pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct AggregatedArchiveBucket {
  timestamp: i64,
  value: Option<f64>,
  value_count: i64,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ProcessStatRecord {
  pub pid: i64,
  pub process_name: String,
  pub avg_cpu_usage: f64,
  pub avg_memory_usage: f64,
  pub total_execution_sec: i64,
  pub latest_timestamp: String,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct GpuNameRow {
  gpu_name: String,
}

pub async fn select_data_archive_series(
  column: DataArchiveColumn,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, ArchiveSeriesError> {
  let pool = db::get_pool().await?;
  select_data_archive_series_from_pool(
    &pool,
    column,
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
}

pub async fn select_gpu_archive_series(
  column: GpuArchiveColumn,
  gpu_name: &str,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, ArchiveSeriesError> {
  let pool = db::get_pool().await?;
  select_gpu_archive_series_from_pool(
    &pool,
    column,
    gpu_name,
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
}

async fn select_data_archive_series_from_pool(
  pool: &SqlitePool,
  column: DataArchiveColumn,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, ArchiveSeriesError> {
  let bounds = ArchiveSeriesBounds::new(start, end, bucket_width_ms, bucket_timestamp)?;
  let sql = data_archive_series_sql(column, bucket_timestamp);
  let rows = sqlx::query_as::<_, AggregatedArchiveBucket>(&sql)
    .bind(format_datetime(start))
    .bind(format_datetime(end))
    .bind(bucket_width_ms)
    .fetch_all(pool)
    .await?;

  Ok(fill_archive_series(rows, bounds))
}

/// One fan's bucketed RPM series over the requested range (#2022).
///
/// A separate result type from a bare `Vec<ArchiveSeriesPoint>` because
/// `FAN_ARCHIVE` is row-per-fan: how many series a machine has is
/// configuration-dependent, so the query answers with the sources it
/// actually found rather than with a fixed set the caller names.
#[derive(Debug, Clone, PartialEq)]
pub struct FanArchiveSeries {
  pub source: String,
  pub points: Vec<ArchiveSeriesPoint>,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct AggregatedFanBucket {
  source: String,
  timestamp: i64,
  value: Option<f64>,
  value_count: i64,
}

/// Every archived fan's bucket-average RPM over `[start, end]`, on the
/// same bucket grid the CPU series use so the fan lane lines up with the
/// lanes above it.
///
/// One round trip for every fan rather than one per source: the timeline
/// mounts all of them together, and the number of fans is not known until
/// the rows come back.
pub async fn select_fan_archive_series(
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<FanArchiveSeries>, ArchiveSeriesError> {
  let pool = db::get_pool().await?;
  select_fan_archive_series_from_pool(
    &pool,
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
}

async fn select_fan_archive_series_from_pool(
  pool: &SqlitePool,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<FanArchiveSeries>, ArchiveSeriesError> {
  let bounds = ArchiveSeriesBounds::new(start, end, bucket_width_ms, bucket_timestamp)?;
  let bucket = bucket_timestamp_sql(bucket_timestamp, "$3");
  // Compare via epoch milliseconds rather than a raw TEXT comparison, the
  // same way `cooling_daily_summary`'s range read does. `FAN_ARCHIVE.timestamp`
  // is written through sqlx's native `DateTime<Utc>` encoding (a `+00:00`
  // offset suffix), which does not sort correctly against a differently
  // shaped bind string - a `Z`-suffixed bound would drop the row sitting
  // exactly on it.
  let epoch_ms = sqlite_epoch_milliseconds();
  let sql = format!(
    "SELECT source,
            {bucket} AS timestamp,
            AVG(CAST(rpm AS REAL)) AS value,
            COUNT(rpm) AS value_count
     FROM FAN_ARCHIVE
     WHERE {epoch_ms} BETWEEN $1 AND $2
     GROUP BY source, 2
     ORDER BY source ASC, 2 ASC"
  );
  let rows = sqlx::query_as::<_, AggregatedFanBucket>(&sql)
    .bind(start.timestamp_millis())
    .bind(end.timestamp_millis())
    .bind(bucket_width_ms)
    .fetch_all(pool)
    .await?;

  // Grouped in Rust rather than by a query per source: the rows already
  // arrive ordered by source, and each source's buckets then go through
  // the same gap-filling every other archive series uses.
  let mut series: Vec<FanArchiveSeries> = Vec::new();
  let mut current: Vec<AggregatedArchiveBucket> = Vec::new();
  let mut current_source: Option<String> = None;

  for row in rows {
    if current_source.as_deref() != Some(row.source.as_str()) {
      if let Some(source) = current_source.take() {
        series.push(FanArchiveSeries {
          source,
          points: fill_archive_series(std::mem::take(&mut current), bounds),
        });
      }
      current_source = Some(row.source.clone());
    }
    current.push(AggregatedArchiveBucket {
      timestamp: row.timestamp,
      value: row.value,
      value_count: row.value_count,
    });
  }
  if let Some(source) = current_source {
    series.push(FanArchiveSeries {
      source,
      points: fill_archive_series(current, bounds),
    });
  }

  Ok(series)
}

async fn select_gpu_archive_series_from_pool(
  pool: &SqlitePool,
  column: GpuArchiveColumn,
  gpu_name: &str,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, ArchiveSeriesError> {
  let bounds = ArchiveSeriesBounds::new(start, end, bucket_width_ms, bucket_timestamp)?;
  let sql = gpu_archive_series_sql(column, bucket_timestamp);
  let rows = sqlx::query_as::<_, AggregatedArchiveBucket>(&sql)
    .bind(gpu_name)
    .bind(format_datetime(start))
    .bind(format_datetime(end))
    .bind(bucket_width_ms)
    .fetch_all(pool)
    .await?;

  Ok(fill_archive_series(rows, bounds))
}

#[derive(Debug, Clone, Copy)]
struct ArchiveSeriesBounds {
  first: i64,
  last: i64,
  end: i64,
  width: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
  point_count: i64,
}

impl ArchiveSeriesBounds {
  fn new(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    width: i64,
    bucket_timestamp: ArchiveBucketTimestamp,
  ) -> Result<Self, ArchiveSeriesError> {
    if width <= 0 {
      return Err(ArchiveSeriesError::InvalidBucketWidth);
    }

    let start = start.timestamp_millis();
    let end = end.timestamp_millis();
    if start > end {
      return Err(ArchiveSeriesError::InvalidTimeRange);
    }

    let overflow_error = || ArchiveSeriesError::TooManyPoints {
      requested: MAX_ARCHIVE_SERIES_POINTS + 1,
      maximum: MAX_ARCHIVE_SERIES_POINTS,
    };
    let (first, last) = match bucket_timestamp {
      ArchiveBucketTimestamp::Start => (
        floor_to_bucket(start, width).ok_or_else(overflow_error)?,
        floor_to_bucket(end, width).ok_or_else(overflow_error)?,
      ),
      ArchiveBucketTimestamp::End => (
        ceil_to_bucket(start, width).ok_or_else(overflow_error)?,
        ceil_to_bucket(end, width).ok_or_else(overflow_error)?,
      ),
    };
    let point_count = last
      .checked_sub(first)
      .and_then(|span| span.checked_div(width))
      .and_then(|count| count.checked_add(1))
      .ok_or_else(overflow_error)?;
    if point_count > MAX_ARCHIVE_SERIES_POINTS {
      return Err(ArchiveSeriesError::TooManyPoints {
        requested: point_count,
        maximum: MAX_ARCHIVE_SERIES_POINTS,
      });
    }

    Ok(Self {
      first,
      last,
      end,
      width,
      bucket_timestamp,
      point_count,
    })
  }
}

fn floor_to_bucket(timestamp: i64, width: i64) -> Option<i64> {
  timestamp.div_euclid(width).checked_mul(width)
}

fn ceil_to_bucket(timestamp: i64, width: i64) -> Option<i64> {
  let floor = floor_to_bucket(timestamp, width)?;
  if floor == timestamp {
    Some(floor)
  } else {
    floor.checked_add(width)
  }
}

fn format_datetime(datetime: &DateTime<Utc>) -> String {
  datetime.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// SQL fragment converting the `timestamp` TEXT column to epoch
/// milliseconds. `pub(crate)` so other Core query modules (e.g.
/// `cooling_daily_summary`) can filter/compare against `DATA_ARCHIVE`
/// timestamps without duplicating this conversion or relying on raw TEXT
/// comparison, which only sorts correctly when every writer produces the
/// exact same string shape.
pub(crate) fn sqlite_epoch_milliseconds() -> &'static str {
  "(CAST(strftime('%s', timestamp) AS INTEGER) * 1000 + \
   CAST(substr(strftime('%f', timestamp), 4, 3) AS INTEGER))"
}

fn bucket_timestamp_sql(
  bucket_timestamp: ArchiveBucketTimestamp,
  width_parameter: &str,
) -> String {
  let timestamp = sqlite_epoch_milliseconds();
  match bucket_timestamp {
    ArchiveBucketTimestamp::Start => {
      format!("(({timestamp}) / {width_parameter}) * {width_parameter}")
    }
    ArchiveBucketTimestamp::End => format!(
      "((({timestamp}) + {width_parameter} - 1) / {width_parameter}) * {width_parameter}"
    ),
  }
}

fn data_archive_series_sql(
  column: DataArchiveColumn,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> String {
  let bucket = bucket_timestamp_sql(bucket_timestamp, "$3");
  format!(
    "SELECT {bucket} AS timestamp,
            {}(CAST({} AS REAL)) AS value,
            COUNT({}) AS value_count
     FROM DATA_ARCHIVE
     WHERE timestamp BETWEEN $1 AND $2
     GROUP BY 1
     ORDER BY 1 ASC",
    column.aggregation().sql(),
    column.sql(),
    column.sql()
  )
}

fn gpu_archive_series_sql(
  column: GpuArchiveColumn,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> String {
  let bucket = bucket_timestamp_sql(bucket_timestamp, "$4");
  format!(
    "SELECT {bucket} AS timestamp,
            {}(CAST({} AS REAL)) AS value,
            COUNT({}) AS value_count
     FROM GPU_DATA_ARCHIVE
     WHERE gpu_name = $1
       AND timestamp BETWEEN $2 AND $3
     GROUP BY 1
     ORDER BY 1 ASC",
    column.aggregation().sql(),
    column.sql(),
    column.sql()
  )
}

fn fill_archive_series(
  rows: Vec<AggregatedArchiveBucket>,
  bounds: ArchiveSeriesBounds,
) -> Vec<ArchiveSeriesPoint> {
  let mut rows = rows.into_iter().peekable();
  let mut series = Vec::with_capacity(bounds.point_count as usize);
  let mut timestamp = bounds.first;

  while timestamp <= bounds.last {
    while rows.peek().is_some_and(|row| row.timestamp < timestamp) {
      rows.next();
    }

    let row = if rows.peek().is_some_and(|row| row.timestamp == timestamp) {
      rows.next()
    } else {
      None
    };
    let should_include = match bounds.bucket_timestamp {
      ArchiveBucketTimestamp::Start => true,
      ArchiveBucketTimestamp::End => {
        timestamp <= bounds.end || row.as_ref().is_some_and(|row| row.value_count > 0)
      }
    };

    if should_include {
      series.push(ArchiveSeriesPoint {
        timestamp,
        value: row.and_then(|row| row.value),
      });
    }

    if timestamp == bounds.last {
      break;
    }
    timestamp = timestamp.saturating_add(bounds.width);
  }

  series
}

pub async fn select_process_stats(
  start: &str,
  end: &str,
  order_by_cpu_desc: bool,
) -> Result<Vec<ProcessStatRecord>, sqlx::Error> {
  let pool = db::get_pool().await?;
  let order_by = if order_by_cpu_desc {
    " ORDER BY avg_cpu_usage DESC"
  } else {
    ""
  };
  let sql = format!(
    "SELECT
       pid,
       process_name,
       AVG(cpu_usage) AS avg_cpu_usage,
       AVG(memory_usage) AS avg_memory_usage,
       MAX(execution_sec) AS total_execution_sec,
       MAX(timestamp) AS latest_timestamp
     FROM PROCESS_STATS
     WHERE timestamp BETWEEN $1 AND $2
     GROUP BY pid, process_name{order_by}"
  );

  sqlx::query_as::<_, ProcessStatRecord>(&sql)
    .bind(start)
    .bind(end)
    .fetch_all(&pool)
    .await
}

pub async fn select_gpu_names() -> Result<Vec<String>, sqlx::Error> {
  let pool = db::get_pool().await?;
  let rows = sqlx::query_as::<_, GpuNameRow>(
    "SELECT DISTINCT gpu_name
     FROM GPU_DATA_ARCHIVE
     WHERE gpu_name IS NOT NULL
       AND gpu_name != 'Unknown'
     ORDER BY gpu_name ASC",
  )
  .fetch_all(&pool)
  .await?;

  Ok(rows.into_iter().map(|row| row.gpu_name).collect())
}

#[cfg(test)]
fn data_archive_select_sql(column: DataArchiveColumn) -> String {
  format!(
    "SELECT id, CAST({} AS REAL) AS value, timestamp
     FROM DATA_ARCHIVE
     WHERE timestamp BETWEEN $1 AND $2
     ORDER BY timestamp ASC, id ASC",
    column.sql()
  )
}

#[cfg(test)]
fn gpu_archive_select_sql(column: GpuArchiveColumn) -> String {
  format!(
    "SELECT id, CAST({} AS REAL) AS value, timestamp
     FROM GPU_DATA_ARCHIVE
     WHERE gpu_name = $1
       AND timestamp BETWEEN $2 AND $3
     ORDER BY timestamp ASC, id ASC",
    column.sql()
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::sqlite::SqlitePool;

  #[test]
  fn data_archive_columns_are_whitelisted() {
    assert_eq!(DataArchiveColumn::CpuAvg.sql(), "cpu_avg");
    assert_eq!(DataArchiveColumn::CpuMax.sql(), "cpu_max");
    assert_eq!(DataArchiveColumn::CpuMin.sql(), "cpu_min");
    assert_eq!(
      DataArchiveColumn::CpuTemperatureAvg.sql(),
      "cpu_temperature_avg"
    );
    assert_eq!(
      DataArchiveColumn::CpuTemperatureMax.sql(),
      "cpu_temperature_max"
    );
    assert_eq!(
      DataArchiveColumn::CpuTemperatureMin.sql(),
      "cpu_temperature_min"
    );
    assert_eq!(DataArchiveColumn::RamAvg.sql(), "ram_avg");
    assert_eq!(DataArchiveColumn::RamMax.sql(), "ram_max");
    assert_eq!(DataArchiveColumn::RamMin.sql(), "ram_min");
    assert_eq!(DataArchiveColumn::CpuPowerAvg.sql(), "cpu_power_avg");
    assert_eq!(DataArchiveColumn::CpuPowerMax.sql(), "cpu_power_max");
    assert_eq!(DataArchiveColumn::CpuPowerMin.sql(), "cpu_power_min");
    assert_eq!(DataArchiveColumn::GpuPowerAvg.sql(), "gpu_power_avg");
    assert_eq!(DataArchiveColumn::GpuPowerMax.sql(), "gpu_power_max");
    assert_eq!(DataArchiveColumn::GpuPowerMin.sql(), "gpu_power_min");
    assert_eq!(DataArchiveColumn::AnePowerAvg.sql(), "ane_power_avg");
    assert_eq!(DataArchiveColumn::AnePowerMax.sql(), "ane_power_max");
    assert_eq!(DataArchiveColumn::AnePowerMin.sql(), "ane_power_min");
    assert_eq!(
      DataArchiveColumn::PackagePowerAvg.sql(),
      "package_power_avg"
    );
    assert_eq!(
      DataArchiveColumn::PackagePowerMax.sql(),
      "package_power_max"
    );
    assert_eq!(
      DataArchiveColumn::PackagePowerMin.sql(),
      "package_power_min"
    );
  }

  #[test]
  fn gpu_archive_columns_are_whitelisted() {
    assert_eq!(GpuArchiveColumn::UsageAvg.sql(), "usage_avg");
    assert_eq!(GpuArchiveColumn::UsageMax.sql(), "usage_max");
    assert_eq!(GpuArchiveColumn::UsageMin.sql(), "usage_min");
    assert_eq!(GpuArchiveColumn::TemperatureAvg.sql(), "temperature_avg");
    assert_eq!(GpuArchiveColumn::TemperatureMax.sql(), "temperature_max");
    assert_eq!(GpuArchiveColumn::TemperatureMin.sql(), "temperature_min");
    assert_eq!(
      GpuArchiveColumn::DedicatedMemoryAvg.sql(),
      "dedicated_memory_avg"
    );
    assert_eq!(
      GpuArchiveColumn::DedicatedMemoryMax.sql(),
      "dedicated_memory_max"
    );
    assert_eq!(
      GpuArchiveColumn::DedicatedMemoryMin.sql(),
      "dedicated_memory_min"
    );
  }

  #[tokio::test]
  async fn data_archive_integer_values_decode_as_f64() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_avg INTEGER,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (id, cpu_avg, timestamp)
       VALUES (1, 42, '2026-06-08T00:00:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, ArchiveRecord>(&data_archive_select_sql(
      DataArchiveColumn::CpuAvg,
    ))
    .bind("2026-06-08T00:00:00.000Z")
    .bind("2026-06-08T00:01:00.000Z")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows[0].value, Some(42.0));
  }

  #[tokio::test]
  async fn data_archive_cpu_temperature_preserves_values_and_missing_readings() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_temperature_avg REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (id, cpu_temperature_avg, timestamp)
       VALUES
         (1, 52.5, '2026-06-08T00:00:00.000Z'),
         (2, NULL, '2026-06-08T00:01:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, ArchiveRecord>(&data_archive_select_sql(
      DataArchiveColumn::CpuTemperatureAvg,
    ))
    .bind("2026-06-08T00:00:00.000Z")
    .bind("2026-06-08T00:02:00.000Z")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows[0].value, Some(52.5));
    assert_eq!(rows[1].value, None);
  }

  #[tokio::test]
  async fn data_archive_returns_each_power_series_and_preserves_null() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_power_avg REAL,
        gpu_power_avg REAL,
        ane_power_avg REAL,
        package_power_avg REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (
        id, cpu_power_avg, gpu_power_avg, ane_power_avg, package_power_avg, timestamp
      ) VALUES (1, 8.5, 4.25, NULL, 13.75, '2026-06-08T00:00:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cases = [
      (DataArchiveColumn::CpuPowerAvg, Some(8.5)),
      (DataArchiveColumn::GpuPowerAvg, Some(4.25)),
      (DataArchiveColumn::AnePowerAvg, None),
      (DataArchiveColumn::PackagePowerAvg, Some(13.75)),
    ];
    for (column, expected) in cases {
      let rows = sqlx::query_as::<_, ArchiveRecord>(&data_archive_select_sql(column))
        .bind("2026-06-08T00:00:00.000Z")
        .bind("2026-06-08T00:01:00.000Z")
        .fetch_all(&pool)
        .await
        .unwrap();
      assert_eq!(rows[0].value, expected);
    }
  }

  #[tokio::test]
  async fn data_archive_records_are_ordered_by_timestamp_and_id() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_avg REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (id, cpu_avg, timestamp)
       VALUES
         (3, 30.0, '2026-06-08T00:02:00.000Z'),
         (1, 10.0, '2026-06-08T00:01:00.000Z'),
         (2, 20.0, '2026-06-08T00:01:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, ArchiveRecord>(&data_archive_select_sql(
      DataArchiveColumn::CpuAvg,
    ))
    .bind("2026-06-08T00:00:00.000Z")
    .bind("2026-06-08T00:03:00.000Z")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
      rows.into_iter().map(|row| row.id).collect::<Vec<_>>(),
      vec![1, 2, 3]
    );
  }

  #[tokio::test]
  async fn gpu_archive_integer_values_decode_as_f64() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE GPU_DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        gpu_name TEXT,
        usage_avg INTEGER,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO GPU_DATA_ARCHIVE (id, gpu_name, usage_avg, timestamp)
       VALUES (1, 'GPU', 65, '2026-06-08T00:00:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, ArchiveRecord>(&gpu_archive_select_sql(
      GpuArchiveColumn::UsageAvg,
    ))
    .bind("GPU")
    .bind("2026-06-08T00:00:00.000Z")
    .bind("2026-06-08T00:01:00.000Z")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows[0].value, Some(65.0));
  }

  #[tokio::test]
  async fn gpu_archive_records_are_ordered_by_timestamp_and_id() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE GPU_DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        gpu_name TEXT,
        usage_avg REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO GPU_DATA_ARCHIVE (id, gpu_name, usage_avg, timestamp)
       VALUES
         (3, 'GPU', 30.0, '2026-06-08T00:02:00.000Z'),
         (1, 'GPU', 10.0, '2026-06-08T00:01:00.000Z'),
         (2, 'GPU', 20.0, '2026-06-08T00:01:00.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, ArchiveRecord>(&gpu_archive_select_sql(
      GpuArchiveColumn::UsageAvg,
    ))
    .bind("GPU")
    .bind("2026-06-08T00:00:00.000Z")
    .bind("2026-06-08T00:03:00.000Z")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
      rows.into_iter().map(|row| row.id).collect::<Vec<_>>(),
      vec![1, 2, 3]
    );
  }

  fn utc(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
      .unwrap()
      .with_timezone(&Utc)
  }

  #[tokio::test]
  async fn data_archive_series_applies_avg_max_and_min_per_bucket() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_avg REAL,
        cpu_max REAL,
        cpu_min REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (id, cpu_avg, cpu_max, cpu_min, timestamp)
       VALUES
         (1, 10.0, 70.0, 8.0, '2026-06-08T00:00:10.000Z'),
         (2, 30.0, 90.0, 5.0, '2026-06-08T00:01:10.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let start = utc("2026-06-08T00:00:00.000Z");
    let end = utc("2026-06-08T00:04:59.999Z");

    let cases = [
      (DataArchiveColumn::CpuAvg, 20.0),
      (DataArchiveColumn::CpuMax, 90.0),
      (DataArchiveColumn::CpuMin, 5.0),
    ];
    for (column, expected) in cases {
      let series = select_data_archive_series_from_pool(
        &pool,
        column,
        &start,
        &end,
        300_000,
        ArchiveBucketTimestamp::Start,
      )
      .await
      .unwrap();

      assert_eq!(series.len(), 1);
      assert_eq!(series[0].value, Some(expected));
    }
  }

  #[tokio::test]
  async fn data_archive_series_preserves_null_values_and_fills_missing_buckets() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_temperature_avg REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (id, cpu_temperature_avg, timestamp)
       VALUES
         (1, 52.5, '2026-06-08T00:00:10.000Z'),
         (2, NULL, '2026-06-08T00:01:10.000Z'),
         (3, NULL, '2026-06-08T00:03:10.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let series = select_data_archive_series_from_pool(
      &pool,
      DataArchiveColumn::CpuTemperatureAvg,
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:03:59.999Z"),
      60_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(
      series.iter().map(|point| point.value).collect::<Vec<_>>(),
      vec![Some(52.5), None, None, None]
    );
  }

  #[tokio::test]
  async fn end_timestamp_buckets_use_query_start_and_omit_empty_trailing_bucket() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        cpu_avg REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (id, cpu_avg, timestamp)
       VALUES
         (1, 10.0, '2026-06-08T00:00:30.000Z'),
         (2, 20.0, '2026-06-08T00:01:00.000Z'),
         (3, 30.0, '2026-06-08T00:01:00.001Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let series = select_data_archive_series_from_pool(
      &pool,
      DataArchiveColumn::CpuAvg,
      &utc("2026-06-08T00:00:00.001Z"),
      &utc("2026-06-08T00:02:00.001Z"),
      60_000,
      ArchiveBucketTimestamp::End,
    )
    .await
    .unwrap();

    assert_eq!(
      series,
      vec![
        ArchiveSeriesPoint {
          timestamp: utc("2026-06-08T00:01:00.000Z").timestamp_millis(),
          value: Some(15.0),
        },
        ArchiveSeriesPoint {
          timestamp: utc("2026-06-08T00:02:00.000Z").timestamp_millis(),
          value: Some(30.0),
        },
      ]
    );
  }

  #[tokio::test]
  async fn gpu_archive_series_filters_by_name_before_aggregating() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
      "CREATE TABLE GPU_DATA_ARCHIVE (
        id INTEGER PRIMARY KEY,
        gpu_name TEXT,
        usage_max REAL,
        timestamp DATETIME
      )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
      "INSERT INTO GPU_DATA_ARCHIVE (id, gpu_name, usage_max, timestamp)
       VALUES
         (1, 'GPU A', 40.0, '2026-06-08T00:00:10.000Z'),
         (2, 'GPU A', 70.0, '2026-06-08T00:01:10.000Z'),
         (3, 'GPU B', 99.0, '2026-06-08T00:01:10.000Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let series = select_gpu_archive_series_from_pool(
      &pool,
      GpuArchiveColumn::UsageMax,
      "GPU A",
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:04:59.999Z"),
      300_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(series[0].value, Some(70.0));
  }

  // ── fan archive series (#2022) ──

  async fn setup_fan_archive(pool: &SqlitePool) {
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

  /// Binds the native `DateTime<Utc>` the real writer
  /// (`fan_archive::insert`) uses, so the range query under test has to
  /// compare against sqlx's own encoding rather than a hand-formatted
  /// literal that happens to match the bound's shape.
  async fn insert_fan_row(pool: &SqlitePool, source: &str, rpm: i64, timestamp: &str) {
    sqlx::query("INSERT INTO FAN_ARCHIVE (source, rpm, timestamp) VALUES ($1, $2, $3)")
      .bind(source)
      .bind(rpm)
      .bind(utc(timestamp))
      .execute(pool)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn fan_archive_series_returns_one_series_per_fan() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    insert_fan_row(&pool, "Fan 1", 900, "2026-06-08T00:00:10.000Z").await;
    insert_fan_row(&pool, "Fan 1", 1100, "2026-06-08T00:01:10.000Z").await;
    insert_fan_row(&pool, "Fan 2", 1500, "2026-06-08T00:00:10.000Z").await;

    let series = select_fan_archive_series_from_pool(
      &pool,
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:04:59.999Z"),
      300_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(
      series
        .iter()
        .map(|entry| entry.source.as_str())
        .collect::<Vec<_>>(),
      vec!["Fan 1", "Fan 2"]
    );
    assert_eq!(series[0].points[0].value, Some(1000.0));
    assert_eq!(series[1].points[0].value, Some(1500.0));
  }

  #[tokio::test]
  async fn fan_archive_series_leaves_unrecorded_buckets_empty() {
    // The lane must break where nothing was recorded rather than drawing
    // a line straight through the gap.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    insert_fan_row(&pool, "Fan 1", 900, "2026-06-08T00:00:10.000Z").await;
    insert_fan_row(&pool, "Fan 1", 1100, "2026-06-08T00:03:10.000Z").await;

    let series = select_fan_archive_series_from_pool(
      &pool,
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:03:59.999Z"),
      60_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(
      series[0]
        .points
        .iter()
        .map(|point| point.value)
        .collect::<Vec<_>>(),
      vec![Some(900.0), None, None, Some(1100.0)]
    );
  }

  #[tokio::test]
  async fn fan_archive_series_keeps_an_inactive_reading_as_a_real_zero() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    insert_fan_row(&pool, "Fan 3", 0, "2026-06-08T00:00:10.000Z").await;

    let series = select_fan_archive_series_from_pool(
      &pool,
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:00:59.999Z"),
      60_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(series[0].points[0].value, Some(0.0));
  }

  #[tokio::test]
  async fn fan_archive_series_includes_the_row_sitting_exactly_on_each_bound() {
    // The regression: the writer stores `+00:00`-suffixed text while a
    // `Z`-suffixed bound compares differently under SQLite's lexicographic
    // ordering, so the rows exactly on the range edges silently dropped
    // out and the lane lost its first and last bucket.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    insert_fan_row(&pool, "Fan 1", 800, "2026-06-08T00:00:00.000Z").await;
    insert_fan_row(&pool, "Fan 1", 1200, "2026-06-08T00:02:00.000Z").await;

    let series = select_fan_archive_series_from_pool(
      &pool,
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:02:00.000Z"),
      60_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(
      series[0]
        .points
        .iter()
        .map(|point| point.value)
        .collect::<Vec<_>>(),
      vec![Some(800.0), None, Some(1200.0)],
      "both boundary rows must survive the range filter"
    );
  }

  #[tokio::test]
  async fn fan_archive_series_excludes_the_row_just_outside_each_bound() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;
    insert_fan_row(&pool, "Fan 1", 100, "2026-06-07T23:59:59.999Z").await;
    insert_fan_row(&pool, "Fan 1", 800, "2026-06-08T00:00:00.000Z").await;
    insert_fan_row(&pool, "Fan 1", 900, "2026-06-08T00:01:00.001Z").await;

    let series = select_fan_archive_series_from_pool(
      &pool,
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:01:00.000Z"),
      60_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(
      series[0]
        .points
        .iter()
        .map(|point| point.value)
        .collect::<Vec<_>>(),
      vec![Some(800.0), None]
    );
  }

  #[tokio::test]
  async fn fan_archive_series_is_empty_without_a_fan_source() {
    // Exactly the signal the lane's capability gate reads: no series at
    // all, rather than one series pinned at zero.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup_fan_archive(&pool).await;

    let series = select_fan_archive_series_from_pool(
      &pool,
      &utc("2026-06-08T00:00:00.000Z"),
      &utc("2026-06-08T00:04:59.999Z"),
      60_000,
      ArchiveBucketTimestamp::Start,
    )
    .await
    .unwrap();

    assert_eq!(series, Vec::new());
  }

  #[test]
  fn archive_series_bounds_reject_unbounded_point_counts() {
    let error = ArchiveSeriesBounds::new(
      &utc("2026-06-01T00:00:00.000Z"),
      &utc("2026-07-01T00:00:00.000Z"),
      1,
      ArchiveBucketTimestamp::Start,
    )
    .unwrap_err();

    assert!(matches!(error, ArchiveSeriesError::TooManyPoints { .. }));
  }

  #[test]
  fn archive_series_bounds_reject_extreme_point_counts() {
    let error = ArchiveSeriesBounds::new(
      &DateTime::<Utc>::MIN_UTC,
      &DateTime::<Utc>::MAX_UTC,
      1,
      ArchiveBucketTimestamp::End,
    )
    .unwrap_err();

    assert!(matches!(error, ArchiveSeriesError::TooManyPoints { .. }));
  }
}
