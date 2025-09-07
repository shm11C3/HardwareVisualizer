use crate::models::hardware::{HardwareMonitorState, ProcessInfo};
use sysinfo::{self, ProcessesToUpdate};

/// プロセス平均計算ウィンドウ (秒)
const PROCESS_AVG_WINDOW: usize = 5;

///
/// ## プロセス一覧 (平均 CPU/メモリ使用率含む) を生成する。
///
/// 動作:
/// - sysinfo のプロセス情報を refresh
/// - 各プロセスについて直近 `PROCESS_AVG_WINDOW` サンプルの CPU / メモリ履歴を平均
/// - CPU 使用率はコア数で正規化し小数1桁へ丸め
/// - メモリは KB 履歴を MB へ変換し小数1桁丸め
///
/// 戻り値: `Vec<ProcessInfo>` (履歴がなければ現在値/0 を使用)
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

      // CPU 使用率 (直近 PROCESS_AVG_WINDOW 秒の平均 / コア数 正規化)
      let cpu_usage = process_cpu_histories
        .get(&pid)
        .map(|hist| {
          let len = hist.len().min(PROCESS_AVG_WINDOW);
          if len == 0 {
            return 0.0;
          }
          let sum: f32 = hist.iter().rev().take(len).sum();
          let avg = sum / len as f32;
          // コア数で正規化
          let normalized = avg / num_cores;
          rounding::round1(normalized)
        })
        .unwrap_or(0.0);

      // メモリ使用量 (MB) 直近平均
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
