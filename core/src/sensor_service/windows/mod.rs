mod client;
mod host;
mod maintenance;
mod providers;
pub(crate) mod security;
mod server;

pub(crate) use client::collect;
pub use client::{SampleError, diagnostics, motherboard, power, temperature};
pub use host::run;
pub use maintenance::{configure, status};

const MAX_CLIENTS: usize = 16;

pub const SERVICE_NAME: &str = "HardwareVisualizerSensors";
pub const PIPE_NAME: &str = r"\\.\pipe\HardwareVisualizer.Sensors.v1";

fn uptime_ms() -> u64 {
  // A machine-wide clock shared by both processes, including suspend time.
  unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
}

/// Read-only validation invoked from the MSI embedded executable before installing the service.
pub fn validate_install_directory(path: &std::path::Path) -> Result<(), String> {
  security::validate_install_directory(path)
}
