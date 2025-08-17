use crate::structs::hardware::HardwareMonitorState;

///
/// ## 全体 CPU 使用率 (%) を返す
///
/// sysinfo が保持する直近サンプルの各コア usage を平均し四捨五入。
/// CPU が 0 個 (異常系) の場合は 0 を返す。
///
pub fn overall_cpu_usage(state: &HardwareMonitorState) -> i32 {
  let system = state.system.lock().unwrap();
  let cpus = system.cpus();
  if cpus.is_empty() {
    return 0;
  }
  let total: f32 = cpus.iter().map(|c| c.cpu_usage()).sum();
  (total / cpus.len() as f32).round() as i32
}

///
/// ## 各プロセッサの CPU 使用率 (%) のベクタを返す
///
/// 取得した値は丸めず生の f32 (sysinfo 提供値) を返す。
///
pub fn per_cpu_usage(state: &HardwareMonitorState) -> Vec<f32> {
  let system = state.system.lock().unwrap();
  system.cpus().iter().map(|c| c.cpu_usage()).collect()
}
