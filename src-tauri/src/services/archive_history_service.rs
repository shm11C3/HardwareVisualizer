use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::archive_queries::{
  AmbientArchiveSeries, ArchiveBucketTimestamp, ArchiveSeriesPoint, DataArchiveColumn,
  FanArchiveSeries, GpuArchiveColumn, ProcessStatRecord,
};
use hardviz_core::infrastructure::database::dispatch;

use crate::log_warn;

/// Logs one archive read failure with the query arguments, then returns the
/// string the frontend receives. The frontend shows these failures inline
/// without the Rust text, so this line is the diagnostic record in `app_log`
/// (#2311). `query` is the pre-formatted argument list for the failed read.
fn archive_read_failure(
  function: &str,
  query: &str,
  error: impl std::fmt::Display,
  message: &str,
) -> String {
  log_warn!(
    format!("archive read failed: {query}, error={error}"),
    function,
    None::<&str>
  );
  format!("{message}: {error}")
}

pub async fn fetch_data_archive_series(
  column: DataArchiveColumn,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, String> {
  dispatch::data_archive::select_data_archive_series(
    column,
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
  .map_err(|e| {
    archive_read_failure(
      "archive_history_service::fetch_data_archive_series",
      &format!("data_archive_series column={column:?} start={start} end={end} bucket_width_ms={bucket_width_ms} bucket_timestamp={bucket_timestamp:?}"),
      e,
      "Failed to fetch archived hardware series",
    )
  })
}

pub async fn fetch_gpu_archive_series(
  column: GpuArchiveColumn,
  gpu_name: &str,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, String> {
  dispatch::gpu_archive::select_gpu_archive_series(
    column,
    gpu_name,
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
  .map_err(|e| {
    archive_read_failure(
      "archive_history_service::fetch_gpu_archive_series",
      &format!("gpu_archive_series column={column:?} gpu_name={gpu_name:?} start={start} end={end} bucket_width_ms={bucket_width_ms} bucket_timestamp={bucket_timestamp:?}"),
      e,
      "Failed to fetch archived GPU series",
    )
  })
}

/// Every archived fan's bucketed RPM series over one range (#2022).
///
/// One call rather than one per fan: `FAN_ARCHIVE` is row-per-fan, so the
/// caller cannot know how many series exist until the rows come back.
pub async fn fetch_fan_archive_series(
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<FanArchiveSeries>, String> {
  dispatch::fan_archive::select_fan_archive_series(
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
  .map_err(|e| {
    archive_read_failure(
      "archive_history_service::fetch_fan_archive_series",
      &format!("fan_archive_series start={start} end={end} bucket_width_ms={bucket_width_ms} bucket_timestamp={bucket_timestamp:?}"),
      e,
      "Failed to fetch archived fan series",
    )
  })
}

/// The archived ambient temperature and its paired thermal delta over one
/// range (#2046).
///
/// Core pairs the CPU and ambient sides per archived minute before it
/// aggregates, so the ΔT this returns is the mean of real per-minute
/// differences. Nothing downstream may reconstruct it by subtracting the
/// bucket averages.
pub async fn fetch_ambient_archive_series(
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<AmbientArchiveSeries, String> {
  dispatch::ambient_archive::select_ambient_archive_series(
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
  .map_err(|e| {
    archive_read_failure(
      "archive_history_service::fetch_ambient_archive_series",
      &format!("ambient_archive_series start={start} end={end} bucket_width_ms={bucket_width_ms} bucket_timestamp={bucket_timestamp:?}"),
      e,
      "Failed to fetch archived ambient series",
    )
  })
}

pub async fn fetch_process_stats(
  start: &str,
  end: &str,
) -> Result<Vec<ProcessStatRecord>, String> {
  dispatch::process_stats::select_process_stats(start, end, false)
    .await
    .map_err(|e| {
      archive_read_failure(
        "archive_history_service::fetch_process_stats",
        &format!("process_stats start={start:?} end={end:?}"),
        e,
        "Failed to fetch process stats",
      )
    })
}

pub async fn fetch_process_stats_in_period(
  start: &str,
  end: &str,
) -> Result<Vec<ProcessStatRecord>, String> {
  dispatch::process_stats::select_process_stats(start, end, true)
    .await
    .map_err(|e| {
      archive_read_failure(
        "archive_history_service::fetch_process_stats_in_period",
        &format!("process_stats_in_period start={start:?} end={end:?}"),
        e,
        "Failed to fetch process stats in period",
      )
    })
}

pub async fn fetch_gpu_archive_names() -> Result<Vec<String>, String> {
  dispatch::gpu_archive::select_gpu_names()
    .await
    .map_err(|e| {
      archive_read_failure(
        "archive_history_service::fetch_gpu_archive_names",
        "gpu_archive_names",
        e,
        "Failed to fetch archived GPU names",
      )
    })
}
