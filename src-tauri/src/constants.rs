/// Archive interval in seconds for persisting hardware data to database.
///
/// Determines how frequently hardware monitoring data is archived
/// from memory to persistent storage. This interval balances between
/// data granularity and storage efficiency.
///
/// Set to 60 seconds to align with the history buffer size.
pub const HARDWARE_ARCHIVE_INTERVAL_SECONDS: u64 = 60;
