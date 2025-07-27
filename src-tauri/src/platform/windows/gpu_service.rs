use crate::platform::windows::directx_gpu_service;
use crate::platform::common::nvidia_gpu_service;
use crate::platform::traits::GpuService;
use crate::platform::windows::wmi_service;
use crate::structs::hardware::GraphicInfo;
use crate::{log_error, log_internal};
use async_trait::async_trait;

pub struct WindowsGpuService;

#[async_trait]
impl GpuService for WindowsGpuService {
  async fn get_gpu_usage(&self) -> Result<f32, String> {
    match nvidia_gpu_service::get_nvidia_gpu_usage().await {
      Ok(usage) => Ok(usage * 100.0),
      Err(_) => match wmi_service::get_gpu_usage_by_device_and_engine("3D").await {
        Ok(usage) => Ok(usage * 100.0),
        Err(e) => Err(format!(
          "Failed to get GPU usage from both NVIDIA API and WMI: {e:?}"
        )),
      },
    }
  }

  async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    nvidia_gpu_service::get_nvidia_gpu_info().await
  }

  async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    directx_gpu_service::get_amd_gpu_info().await
  }

  async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    directx_gpu_service::get_intel_gpu_info().await
  }

  async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    let mut gpus_result = Vec::new();

    let (nvidia_gpus_result, amd_gpus_result, intel_gpus_result) = tokio::join!(
      self.get_nvidia_gpus(),
      self.get_amd_gpus(),
      self.get_intel_gpus(),
    );

    match nvidia_gpus_result {
      Ok(nvidia_gpus) => gpus_result.extend(nvidia_gpus),
      Err(e) => log_error!("nvidia_error", "get_all_gpus", Some(e)),
    }

    match amd_gpus_result {
      Ok(amd_gpus) => gpus_result.extend(amd_gpus),
      Err(e) => log_error!("amd_error", "get_all_gpus", Some(e)),
    }

    match intel_gpus_result {
      Ok(intel_gpus) => gpus_result.extend(intel_gpus),
      Err(e) => log_error!("intel_error", "get_all_gpus", Some(e)),
    }

    Ok(gpus_result)
  }
}
