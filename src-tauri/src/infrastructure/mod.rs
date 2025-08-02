#[cfg(target_os = "windows")]
pub mod wmi_provider;

#[cfg(target_os = "linux")]
pub mod dmidecode;

#[cfg(target_os = "linux")]
pub mod procfs;
