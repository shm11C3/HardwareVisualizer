use crate::structs::hardware::{GraphicInfo, MemoryInfo, NetworkInfo};
use async_trait::async_trait;

#[async_trait]
pub trait MemoryService: Send + Sync {
  async fn get_memory_info(&self) -> Result<MemoryInfo, String>;
  async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String>;
}

#[async_trait]
pub trait GpuService: Send + Sync {
  async fn get_gpu_usage(&self) -> Result<f32, String>;
  async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String>;
  async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String>;
  async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String>;
  async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String>;
}

#[async_trait]
pub trait NetworkService: Send + Sync {
  async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String>;
}

pub trait PlatformServices: Send + Sync {
  fn memory_service(&self) -> Box<dyn MemoryService>;
  fn gpu_service(&self) -> Box<dyn GpuService>;
  fn network_service(&self) -> Box<dyn NetworkService>;
}
