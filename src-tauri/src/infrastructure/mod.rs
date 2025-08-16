#[cfg(target_os = "windows")]
pub mod wmi_provider;

#[cfg(target_os = "windows")]
pub mod nvapi;

#[cfg(target_os = "windows")]
pub mod directx;

#[cfg(target_os = "linux")]
pub mod dmidecode;

#[cfg(target_os = "linux")]
pub mod procfs;

#[cfg(target_os = "linux")]
pub mod drm_sys;

#[cfg(target_os = "linux")]
pub mod lspci;

#[cfg(target_os = "linux")]
pub mod kernel;

#[cfg(target_os = "linux")]
pub mod net_sys;
