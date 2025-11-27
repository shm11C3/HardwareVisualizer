use crate::models::hardware::{HardwareMonitorState, MemoryInfo};
use crate::platform::factory::PlatformFactory;

///
/// ## Return memory usage (%). Round used / total * 100
///
/// Returns 0 if total is 0
///
pub fn memory_usage_percent(state: &HardwareMonitorState) -> i32 {
  let system = state.system.lock().unwrap();
  let used = system.used_memory() as f64;
  let total = system.total_memory() as f64;
  if total == 0.0 {
    0
  } else {
    ((used / total) * 100.0).round() as i32
  }
}

///
/// ## Get detailed memory information via Platform
/// Returns `MemoryInfo` on success, error message on failure
///
pub async fn fetch_memory_detail() -> Result<MemoryInfo, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  platform.get_memory_info_detail().await
}
