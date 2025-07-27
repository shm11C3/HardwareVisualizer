use crate::platform::factory::PlatformFactory;
use crate::structs::hardware::{GraphicInfo, MemoryInfo, NetworkInfo};
use specta::Type;

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NameValue {
    pub name: String,
    pub value: i32,
}

pub struct HardwareService;

impl HardwareService {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
        let platform = PlatformFactory::create();
        let memory_service = platform.memory_service();
        memory_service.get_memory_info().await
    }

    pub async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
        let platform = PlatformFactory::create();
        let memory_service = platform.memory_service();
        memory_service.get_detailed_memory_info().await
    }

    pub async fn get_gpu_usage(&self) -> Result<f32, String> {
        let platform = PlatformFactory::create();
        let gpu_service = platform.gpu_service();
        gpu_service.get_gpu_usage().await
    }

    pub async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        let platform = PlatformFactory::create();
        let gpu_service = platform.gpu_service();
        gpu_service.get_all_gpus().await
    }

    pub async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        let platform = PlatformFactory::create();
        let gpu_service = platform.gpu_service();
        gpu_service.get_nvidia_gpus().await
    }

    pub async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        let platform = PlatformFactory::create();
        let gpu_service = platform.gpu_service();
        gpu_service.get_amd_gpus().await
    }

    pub async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        let platform = PlatformFactory::create();
        let gpu_service = platform.gpu_service();
        gpu_service.get_intel_gpus().await
    }

    pub async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
        let platform = PlatformFactory::create();
        let network_service = platform.network_service();
        network_service.get_network_info().await
    }

    pub async fn get_nvidia_gpu_temperature(&self) -> Result<Vec<NameValue>, String> {
        use crate::platform::common::nvidia_gpu_service;
        match nvidia_gpu_service::get_nvidia_gpu_temperature().await {
            Ok(temps) => Ok(temps.into_iter().map(|temp| NameValue {
                name: temp.name,
                value: temp.value,
            }).collect()),
            Err(e) => Err(format!("Failed to get NVIDIA GPU temperature: {:?}", e)),
        }
    }

    pub async fn get_nvidia_gpu_cooler_stat(&self) -> Result<Vec<NameValue>, String> {
        use crate::platform::common::nvidia_gpu_service;
        match nvidia_gpu_service::get_nvidia_gpu_cooler_stat().await {
            Ok(stats) => Ok(stats.into_iter().map(|stat| NameValue {
                name: stat.name,
                value: stat.value,
            }).collect()),
            Err(e) => Err(format!("Failed to get NVIDIA GPU cooler status: {:?}", e)),
        }
    }
}