use hardviz_core::infrastructure::database::archive_queries::{
  self, ArchiveRecord, DataArchiveColumn, GpuArchiveColumn, ProcessStatRecord,
};

pub async fn fetch_data_archive_records(
  column: DataArchiveColumn,
  start: &str,
  end: &str,
) -> Result<Vec<ArchiveRecord>, String> {
  archive_queries::select_data_archive_records(column, start, end)
    .await
    .map_err(|e| format!("Failed to fetch archived hardware records: {e}"))
}

pub async fn fetch_gpu_archive_records(
  column: GpuArchiveColumn,
  gpu_name: &str,
  start: &str,
  end: &str,
) -> Result<Vec<ArchiveRecord>, String> {
  archive_queries::select_gpu_archive_records(column, gpu_name, start, end)
    .await
    .map_err(|e| format!("Failed to fetch archived GPU records: {e}"))
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
