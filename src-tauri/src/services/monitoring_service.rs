use crate::structs::hardware::HardwareMonitorState;

/// 履歴抽出の最大秒数
const HISTORY_CAP: usize = 3600;

///
/// ## CPU 使用率履歴
///
/// (最新から `seconds` 秒, 上限 HISTORY_CAP) を逆順スライス収集
///
pub fn cpu_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.cpu_history.lock().unwrap();
  let take_n = seconds.min(HISTORY_CAP as u32) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}

///
/// ## メモリ使用率履歴
///
/// (最新から `seconds` 秒, 上限 HISTORY_CAP) を逆順スライス収集
///
pub fn memory_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.memory_history.lock().unwrap();
  let take_n = seconds.min(HISTORY_CAP as u32) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}

///
/// ## GPU 使用率履歴
///
/// (最新から `seconds` 秒, 上限 HISTORY_CAP) を逆順スライス収集
///
pub fn gpu_usage_history(state: &HardwareMonitorState, seconds: u32) -> Vec<f32> {
  let history = state.gpu_history.lock().unwrap();
  let take_n = seconds.min(HISTORY_CAP as u32) as usize;

  history.iter().rev().take(take_n).cloned().collect()
}
