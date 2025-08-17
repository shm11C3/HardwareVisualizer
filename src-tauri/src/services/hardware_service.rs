use crate::infrastructure;
use crate::platform::factory::PlatformFactory;
use crate::structs::hardware::{HardwareMonitorState, SysInfo};

///
/// ハードウェア情報を統合収集する
///
/// - CPU / GPU / Memory / Storage をそれぞれ取得
/// - 個別失敗は None (または空) として続行
/// - CPU / GPU / Memory が全て取得不能なら Err
///
pub async fn collect_hardware_info(
  state: &HardwareMonitorState,
) -> Result<SysInfo, String> {
  let cpu =
    infrastructure::sysinfo_provider::get_cpu_info(state.system.lock().unwrap()).ok();

  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;
  let gpus = platform.get_gpu_info().await.ok();
  let memory = platform.get_memory_info().await.ok();
  let storage = infrastructure::sysinfo_provider::get_storage_info().unwrap_or_default();

  if cpu.is_none() && gpus.is_none() && memory.is_none() {
    return Err("Failed to get any hardware info".to_string());
  }

  Ok(SysInfo {
    cpu,
    memory,
    gpus,
    storage,
  })
}
