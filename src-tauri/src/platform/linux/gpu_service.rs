use crate::platform::common::nvidia_gpu_service;
use crate::platform::traits::GpuService;
#[cfg(target_os = "linux")]
use crate::platform::linux::{amd_gpu_linux, gpu_linux, intel_gpu_linux};
use crate::structs::hardware::GraphicInfo;
use crate::{log_error, log_internal};
use async_trait::async_trait;

pub struct LinuxGpuService;

#[async_trait]
impl GpuService for LinuxGpuService {
  async fn get_gpu_usage(&self) -> Result<f32, String> {
    if let Ok(usage) = nvidia_gpu_service::get_nvidia_gpu_usage().await {
      return Ok(usage * 100.0);
    }

    #[cfg(target_os = "linux")]
    {
      for card_id in 0..=9 {
        let vendor_path = format!("/sys/class/drm/card{card_id}/device/vendor");
        if !std::path::Path::new(&vendor_path).exists() {
          continue;
        }

        match gpu_linux::detect_gpu_vendor(card_id) {
          gpu_linux::GpuVendor::Nvidia => {
            if let Ok(usage) = nvidia_gpu_service::get_nvidia_gpu_usage().await {
              return Ok(usage * 100.0);
            }
          }
          gpu_linux::GpuVendor::Amd => {
            if let Ok(usage) = amd_gpu_linux::get_amd_gpu_usage(card_id).await {
              return Ok(usage);
            }
          }
          gpu_linux::GpuVendor::Intel => {
            if let Ok(usage) = intel_gpu_linux::get_intel_gpu_usage(card_id).await {
              return Ok(usage);
            }
          }
        }
      }
    }

    Err("No GPU usage information available".to_string())
  }

  async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    nvidia_gpu_service::get_nvidia_gpu_info().await
  }

  async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    #[cfg(target_os = "linux")]
    {
      let mut gpus = Vec::new();
      for card_id in gpu_linux::get_all_card_ids() {
        if let gpu_linux::GpuVendor::Amd = gpu_linux::detect_gpu_vendor(card_id) {
          if let Ok(info) = amd_gpu_linux::get_amd_graphic_info(card_id).await {
            gpus.push(info);
          }
        }
      }
      Ok(gpus)
    }
    #[cfg(not(target_os = "linux"))]
    {
      Ok(vec![])
    }
  }

  async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    #[cfg(target_os = "linux")]
    {
      let mut gpus = Vec::new();
      for card_id in gpu_linux::get_all_card_ids() {
        if let gpu_linux::GpuVendor::Intel = gpu_linux::detect_gpu_vendor(card_id) {
          if let Ok(info) = intel_gpu_linux::get_intel_graphic_info(card_id).await {
            gpus.push(info);
          }
        }
      }
      Ok(gpus)
    }
    #[cfg(not(target_os = "linux"))]
    {
      Ok(vec![])
    }
  }

  async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
    let mut gpus_result = Vec::new();

    let nvidia_gpus_result = self.get_nvidia_gpus().await;
    match nvidia_gpus_result {
      Ok(nvidia_gpus) => gpus_result.extend(nvidia_gpus),
      Err(e) => log_error!("nvidia_error", "get_all_gpus", Some(e)),
    }

    #[cfg(target_os = "linux")]
    {
      for card_id in gpu_linux::get_all_card_ids() {
        match gpu_linux::detect_gpu_vendor(card_id) {
          gpu_linux::GpuVendor::Amd => {
            if let Ok(info) = amd_gpu_linux::get_amd_graphic_info(card_id).await {
              gpus_result.push(info);
            }
          }
          gpu_linux::GpuVendor::Intel => {
            if let Ok(info) = intel_gpu_linux::get_intel_graphic_info(card_id).await {
              gpus_result.push(info);
            }
          }
          _ => {}
        }
      }
    }

    Ok(gpus_result)
  }
}
