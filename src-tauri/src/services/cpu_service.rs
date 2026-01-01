use crate::models::hardware::HardwareMonitorState;

///
/// ## Return overall CPU usage (%)
///
/// Average the usage of each core from sysinfo's recent sample and round.
/// Returns 0 if CPU count is 0 (error case).
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
/// ## Return vector of CPU usage (%) for each processor
///
/// Return raw f32 values (sysinfo provided values) without rounding.
///
pub fn per_cpu_usage(state: &HardwareMonitorState) -> Vec<f32> {
  let system = state.system.lock().unwrap();
  system.cpus().iter().map(|c| c.cpu_usage()).collect()
}
