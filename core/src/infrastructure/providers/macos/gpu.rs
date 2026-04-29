use std::{
  sync::OnceLock,
  sync::atomic::{AtomicU32, Ordering},
  time::Duration,
};

use crate::infrastructure::providers::io_kit::io_report::GpuUsageIOReport;

/// Store the `f32` as raw bits so it can be kept inside an `AtomicU32`.
static GPU_USAGE_BITS: OnceLock<AtomicU32> = OnceLock::new();

/// Indicates whether the sampler thread has been started.
static GPU_SAMPLER_STARTED: OnceLock<()> = OnceLock::new();

pub fn init_gpu_usage_sampler_thread() -> Result<(), String> {
  // No-op if already started.
  if GPU_SAMPLER_STARTED.get().is_some() {
    return Ok(());
  }

  // Initialize the atomic value (`NaN` means “not sampled yet”).
  let _ = GPU_USAGE_BITS.get_or_init(|| AtomicU32::new(f32::NAN.to_bits()));

  // Even under races, `OnceLock` ensures this is set only once.
  GPU_SAMPLER_STARTED
    .set(())
    .map_err(|_| "GPU sampler already started".to_string())?;

  std::thread::Builder::new()
    .name("gpu-usage-sampler".to_string())
    .spawn(|| {
      // Dedicated thread: keep `GpuUsageIOReport` confined here.
      let mut sampler = match GpuUsageIOReport::new() {
        Ok(x) => x,
        Err(_) => return, // Failed to start; add logging if needed.
      };

      loop {
        std::thread::sleep(Duration::from_millis(1000));

        if let Ok(usage) = sampler.sample_usage()
          && let Some(a) = GPU_USAGE_BITS.get()
        {
          a.store(usage.to_bits(), Ordering::Relaxed);
        }
      }
    })
    .expect("failed to spawn gpu-usage-sampler thread");

  Ok(())
}

pub fn read_gpu_usage_cached() -> Option<f32> {
  let bits = GPU_USAGE_BITS.get()?.load(Ordering::Relaxed);
  let v = f32::from_bits(bits);
  if v.is_nan() { None } else { Some(v) }
}
