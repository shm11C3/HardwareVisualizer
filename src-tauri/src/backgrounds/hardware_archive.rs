use crate::constants::HARDWARE_ARCHIVE_INTERVAL_SECONDS;
use crate::services::archive_service::ArchiveService;
use crate::structs;
use std::time::Duration;

/// Starts the hardware archive background service.
///
/// This orchestrates the periodic collection and archiving of hardware data
/// by coordinating between data collection (service layer) and persistence (database layer).
pub async fn setup(resources: structs::hardware_archive::MonitorResources) {
  let mut interval =
    tokio::time::interval(Duration::from_secs(HARDWARE_ARCHIVE_INTERVAL_SECONDS));

  loop {
    interval.tick().await;
    ArchiveService::archive_current_snapshot(&resources).await;
  }
}

/// Deletes old archived data beyond the specified retention period.
///
/// This function delegates to the service layer for better separation of concerns.
pub async fn batch_delete_old_data(retention_days: u32) {
  ArchiveService::cleanup_old_data(retention_days).await;
}
