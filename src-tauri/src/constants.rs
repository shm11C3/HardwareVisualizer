/// Maximum number of data points to retain in hardware monitoring history.
/// 
/// This controls the circular buffer size for real-time hardware metrics
/// (CPU usage, memory usage, GPU usage). When the buffer reaches this capacity,
/// the oldest data points are removed to make room for new ones.
/// 
/// With 1-second sampling intervals, this provides 60 seconds (1 minute) 
/// of historical data for chart visualization.
pub const HARDWARE_HISTORY_BUFFER_SIZE: usize = 60;

/// Maximum time range in seconds for hardware history queries.
/// 
/// This limits how far back in time users can request historical data
/// from the monitoring service. Prevents excessive memory usage and
/// ensures reasonable response times for history requests.
/// 
/// Set to 3600 seconds (1 hour) as a reasonable upper bound for
/// real-time monitoring use cases.
pub const MAX_HISTORY_QUERY_DURATION_SECONDS: u32 = 3600;

/// Archive interval in seconds for persisting hardware data to database.
/// 
/// Determines how frequently hardware monitoring data is archived
/// from memory to persistent storage. This interval balances between
/// data granularity and storage efficiency.
/// 
/// Set to 60 seconds to align with the history buffer size.
pub const HARDWARE_ARCHIVE_INTERVAL_SECONDS: u64 = 60;