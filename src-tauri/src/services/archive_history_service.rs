use chrono::{DateTime, Utc};
use hardviz_core::infrastructure::database::archive_queries::{
  self, ArchiveBucketTimestamp, ArchiveSeriesPoint, DataArchiveColumn, FanArchiveSeries,
  GpuArchiveColumn, ProcessStatRecord,
};

pub async fn fetch_data_archive_series(
  column: DataArchiveColumn,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, String> {
  archive_queries::select_data_archive_series(
    column,
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
  .map_err(|e| format!("Failed to fetch archived hardware series: {e}"))
}

pub async fn fetch_gpu_archive_series(
  column: GpuArchiveColumn,
  gpu_name: &str,
  start: &DateTime<Utc>,
  end: &DateTime<Utc>,
  bucket_width_ms: i64,
  bucket_timestamp: ArchiveBucketTimestamp,
) -> Result<Vec<ArchiveSeriesPoint>, String> {
  archive_queries::select_gpu_archive_series(
    column,
    gpu_name,
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
  .map_err(|e| format!("Failed to fetch archived GPU series: {e}"))
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
  archive_queries::select_fan_archive_series(
    start,
    end,
    bucket_width_ms,
    bucket_timestamp,
  )
  .await
  .map_err(|e| format!("Failed to fetch archived fan series: {e}"))
}

pub async fn fetch_process_stats(
  start: &str,
  end: &str,
) -> Result<Vec<ProcessStatRecord>, String> {
  archive_queries::select_process_stats(start, end, false)
    .await
    .map_err(|e| format!("Failed to fetch process stats: {e}"))
}

pub async fn fetch_process_stats_in_period(
  start: &str,
  end: &str,
) -> Result<Vec<ProcessStatRecord>, String> {
  archive_queries::select_process_stats(start, end, true)
    .await
    .map_err(|e| format!("Failed to fetch process stats in period: {e}"))
}

pub async fn fetch_gpu_archive_names() -> Result<Vec<String>, String> {
  archive_queries::select_gpu_names()
    .await
    .map_err(|e| format!("Failed to fetch archived GPU names: {e}"))
}
