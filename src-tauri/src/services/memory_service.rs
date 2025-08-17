use crate::platform::factory::PlatformFactory;
use crate::structs::hardware::{HardwareMonitorState, MemoryInfo};

///
/// ## メモリ使用率 (%) を返す。used / total * 100 を四捨五入する
///
/// total が 0 の場合は 0
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
/// ## 詳細メモリ情報を Platform 経由で取得する
/// 成功時 `MemoryInfo`、失敗時はエラーメッセージ
///
pub async fn fetch_memory_detail() -> Result<MemoryInfo, String> {
  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  platform.get_memory_info_detail().await
}
