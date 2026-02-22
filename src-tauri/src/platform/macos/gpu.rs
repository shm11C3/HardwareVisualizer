use crate::{
  infrastructure::providers::macos::{gpu, gpu_info},
  models,
};

pub async fn get_gpu_usage() -> Result<(f32, String), String> {
  gpu::init_gpu_usage_sampler_thread()?;

  match gpu::read_gpu_usage_cached() {
    Some(v) => Ok((v * 100.0, "IOKit".to_string())),
    None => Err("GPU usage is not ready yet".into()),
  }
}

pub async fn get_gpu_info() -> Result<Vec<models::hardware::GraphicInfo>, String> {
  gpu_info::get_gpu_info().await
}

pub async fn get_gpu_memory_usage()
-> Result<Option<models::hardware::GpuMemoryUsage>, String> {
  gpu_info::get_gpu_memory_usage().await
}
