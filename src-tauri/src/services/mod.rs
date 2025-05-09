pub mod directx_gpu_service;
pub mod language;
pub mod nvidia_gpu_service;
pub mod setting_service;
pub mod system_info_service;

#[cfg(target_os = "windows")]
pub mod wmi_service;

#[cfg(target_os = "linux")]
pub mod dmidecode;

#[cfg(target_os = "linux")]
pub mod ip_linux;

#[cfg(target_os = "linux")]
pub mod amd_gpu_linux;

#[cfg(target_os = "linux")]
pub mod intel_gpu_linux;
