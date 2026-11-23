use crate::{
  enums::error::PlatformError,
  infrastructure::providers::macos::{gpu, gpu_info, io_kit::iokit_info},
  models,
};

pub async fn get_gpu_usage() -> Result<(f32, String), PlatformError> {
  gpu::init_gpu_usage_sampler_thread().map_err(PlatformError::fault)?;

  match gpu::read_gpu_usage_cached() {
    Some(v) => Ok((v * 100.0, "IOKit".to_string())),
    None => Err(PlatformError::unavailable("GPU usage is not ready yet")),
  }
}

pub async fn get_gpu_info() -> Result<Vec<models::hardware::GraphicInfo>, PlatformError> {
  gpu_info::get_gpu_info().await.map_err(PlatformError::fault)
}

pub async fn get_gpu_memory_usage()
-> Result<Option<models::hardware::GpuMemoryUsage>, PlatformError> {
  gpu_info::get_gpu_memory_usage()
    .await
    .map_err(PlatformError::fault)
}

pub async fn sample_gpus() -> Vec<models::GpuSample> {
  static CACHED_GPU_NAME: tokio::sync::OnceCell<String> =
    tokio::sync::OnceCell::const_new();

  let _ = gpu::init_gpu_usage_sampler_thread();

  let gpu_name = CACHED_GPU_NAME
    .get_or_init(|| async {
      gpu_info::get_gpu_info()
        .await
        .ok()
        .and_then(|list| list.into_iter().next())
        .map(|info| info.name)
        .unwrap_or_else(|| "Apple GPU".to_string())
    })
    .await;

  let usage = gpu::read_gpu_usage_cached().map(|v| v * 100.0);
  let memory_kb: Option<f32> =
    tokio::task::spawn_blocking(iokit_info::get_gpu_memory_usage_from_iokit)
      .await
      .ok()
      .flatten()
      .and_then(|mem| mem.in_use_bytes)
      .map(|bytes| (bytes / 1024) as f32);

  vec![models::GpuSample {
    gpu_id: format!("iokit:{}", gpu_name),
    name: gpu_name.clone(),
    usage,
    temperature: None,
    dedicated_memory_kb: memory_kb,
    cooler_level: None,
    source: "IOKit".to_string(),
  }]
}

pub fn sample_power_draw() -> models::PowerDraw {
  #[cfg(target_arch = "aarch64")]
  {
    gpu::read_power_draw_cached()
  }
  #[cfg(not(target_arch = "aarch64"))]
  {
    models::PowerDraw::default()
  }
}
