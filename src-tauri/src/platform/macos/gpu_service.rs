use crate::platform::traits::GpuService;
use crate::structs::hardware::GraphicInfo;
use async_trait::async_trait;

pub struct MacOSGpuService;

#[async_trait]
impl GpuService for MacOSGpuService {
  async fn get_gpu_usage(&self) -> Result<f32, String> {
    Err("macOS GPU usage not implemented yet".to_string())
  }

  async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    Ok(vec![])
  }

  async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    Err("macOS AMD GPU info not implemented yet".to_string())
  }

  async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    Err("macOS Intel GPU info not implemented yet".to_string())
  }

  async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    Err("macOS GPU info not implemented yet".to_string())
  }
}
