use crate::infrastructure;
use crate::models::hardware::{HardwareMonitorState, SysInfo};
use crate::platform::factory::PlatformFactory;

///
/// Collect hardware information in aggregate
///
/// - Get CPU / GPU / Memory / Storage respectively
/// - Continue with None (or empty) for individual failures
/// - Return Err if all of CPU / GPU / Memory cannot be obtained
///
pub async fn collect_hardware_info(
  state: &HardwareMonitorState,
) -> Result<SysInfo, String> {
  let cpu = infrastructure::providers::sysinfo_provider::get_cpu_info(
    state.system.lock().unwrap(),
  )
  .ok();

  let platform =
    PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;

  // Execute GPU / Memory in parallel
  let (gpus_res, memory_res, storage_res) =
    tokio::join!(platform.get_gpu_info(), platform.get_memory_info(), async {
      infrastructure::providers::sysinfo_provider::get_storage_info()
    });

  let gpus = gpus_res.ok();
  let memory = memory_res.ok();
  let storage = storage_res.map_err(|e| format!("Failed to get storage info: {e}"))?;

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
