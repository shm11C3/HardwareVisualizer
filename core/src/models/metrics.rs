/// One sample of per-GPU metrics, normalized across vendors and platforms.
///
/// `None` indicates the metric is unavailable for this vendor/platform.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuMetric {
  pub gpu_id: String,
  pub gpu_name: String,
  pub gpu_usage: Option<f32>,
  pub gpu_temperature: Option<f32>,
  pub gpu_source: String,
  pub gpu_dedicated_memory_usage_kb: Option<f32>,
  pub gpu_cooler_level: Option<u32>,
}

/// One sample of per-process metrics carried inside [`MetricsSnapshot`].
///
/// Persistence subscribers consume these to build per-process rolling
/// stats without sharing the collector's `Arc<Mutex<...>>` history maps.
/// The window adapter ignores them.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessSample {
  pub pid: u32,
  pub name: String,
  /// Raw per-tick CPU usage as reported by sysinfo (sum across all cores).
  pub cpu_usage: f32,
  /// Memory usage in KB, matching the existing on-disk archive format.
  pub memory_kb: f32,
  /// Process run time in seconds.
  pub run_time_secs: u64,
}

/// One real-time snapshot of system + GPU metrics, fanned out via
/// [`crate::event_bus::EventBus`] to all in-process subscribers
/// (window adapter, persistence, future tray/overlay/alert consumers).
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsSnapshot {
  pub cpu_usage: f32,
  pub memory_usage: f32,
  pub processors_usage: Vec<f32>,
  pub gpus: Vec<GpuMetric>,
  pub processes: Vec<ProcessSample>,
}
