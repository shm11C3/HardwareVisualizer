use crate::models::hardware::{HardwareMonitorState, ProcessInfo};
use sysinfo::{self, ProcessesToUpdate};

/// Process average calculation window (seconds)
const PROCESS_AVG_WINDOW: usize = 5;

///
/// ## Generate process list (including average CPU/memory usage).
///
/// Behavior:
/// - Refresh sysinfo process information
/// - Average the CPU / memory history of the most recent `PROCESS_AVG_WINDOW` samples for each process
/// - Normalize CPU usage by core count and round to 1 decimal place
/// - Convert memory from KB history to MB and round to 1 decimal place
///
/// Return value: `Vec<ProcessInfo>` (use current value/0 if no history)
///
pub fn collect_process_list(state: &HardwareMonitorState) -> Vec<ProcessInfo> {
  use crate::utils::rounding;

  let mut system = state.system.lock().unwrap();
  let process_cpu_histories = state.process_cpu_histories.lock().unwrap();
  let process_memory_histories = state.process_memory_histories.lock().unwrap();

  system.refresh_processes(ProcessesToUpdate::All, true);
  let num_cores = system.cpus().len() as f32;

  system
    .processes()
    .values()
    .map(|process| {
      let pid = process.pid();

      // CPU usage (average of last PROCESS_AVG_WINDOW seconds / normalized by core count)
      let cpu_usage = process_cpu_histories
        .get(&pid)
        .map(|hist| {
          let len = hist.len().min(PROCESS_AVG_WINDOW);
          if len == 0 {
            return 0.0;
          }
          let sum: f32 = hist.iter().rev().take(len).sum();
          let avg = sum / len as f32;
          // Normalize by core count
          let normalized = avg / num_cores;
          rounding::round1(normalized)
        })
        .unwrap_or(0.0);

      // Memory usage (MB) recent average
      let memory_usage = process_memory_histories
        .get(&pid)
        .map(|hist| {
          let len = hist.len().min(PROCESS_AVG_WINDOW);
          if len == 0 {
            return process.memory() as f32 / 1024.0;
          }
          let sum: f32 = hist.iter().rev().take(len).sum();
          let avg_kb = sum / len as f32; // KB
          let avg_mb = avg_kb / 1024.0;
          rounding::round1(avg_mb)
        })
        .unwrap_or_else(|| process.memory() as f32 / 1024.0);

      ProcessInfo {
        pid: pid.as_u32() as i32,
        name: process.name().to_string_lossy().into_owned(),
        cpu_usage,
        memory_usage,
      }
    })
    .collect()
}
