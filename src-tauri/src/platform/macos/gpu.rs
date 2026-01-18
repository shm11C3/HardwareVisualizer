use crate::infrastructure::providers::macos::gpu;

pub async fn get_gpu_usage() -> Result<f32, String> {
  gpu::init_gpu_usage_sampler_thread()?;

  match gpu::read_gpu_usage_cached() {
    Some(v) => Ok(v * 100.0),
    None => Err("GPU usage is not ready yet".into()),
  }
}
