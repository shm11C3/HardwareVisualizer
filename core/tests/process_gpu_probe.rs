//! Per-process GPU usage cost and coverage measurement (#2720).
//!
//! Ignored by default: it runs for about a minute and exists to produce
//! evidence for deciding whether Process Insight can archive per-process GPU
//! usage, not to gate a build. It measures, on the current OS:
//!
//! - `baseline`: the 1 Hz collector's existing process refresh (CPU + memory),
//!   the cost an archive would add GPU collection on top of;
//! - `live_read`: the production [`LiveProcessGpuCollector::read`] path that
//!   the Live Process Table polls (dedicated `System`, GPU only);
//! - `combined`: CPU + memory + GPU on one `System`, the shape an
//!   always-on archive collection would take;
//! - coverage of the live reads: how many processes have a value, how many
//!   exceed 100%, and how many repeat a nonzero value bit-for-bit from the
//!   previous read (a sign of values the backend retained, see sysinfo#1811).
//!
//! Run it with
//!
//! ```text
//! cargo test -p hardviz-core --release --test process_gpu_probe -- --ignored --nocapture
//! ```
//!
//! `HV_PROBE_GPU_SAMPLES` (default 20) and `HV_PROBE_GPU_INTERVAL_MS` (default
//! 3000, the Live Process Table poll) steer the run;
//! `HV_PROCESS_GPU_PROBE_OUT` names the JSON artifact, otherwise the JSON is
//! printed to stdout. Keep a GPU workload running during the measurement so
//! coverage reflects real GPU clients.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use hardviz_core::collector::LiveProcessGpuCollector;
use serde_json::{Value, json};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

fn env_u64(name: &str, default: u64) -> u64 {
  std::env::var(name)
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(default)
}

/// Median, p90 and max of durations in milliseconds.
fn summarize(mut ms: Vec<f64>) -> Value {
  ms.sort_by(f64::total_cmp);
  let pick = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
  json!({
    "samples": ms.len(),
    "median_ms": pick(0.5),
    "p90_ms": pick(0.9),
    "max_ms": ms[ms.len() - 1],
  })
}

/// Times `samples` refreshes of one `System` with `kind`, after one warm-up
/// refresh that establishes the delta baseline.
fn time_refreshes(kind: ProcessRefreshKind, samples: u64, interval: Duration) -> Value {
  let mut system = System::new().expect("sysinfo System");
  system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
  let mut ms = Vec::new();
  for _ in 0..samples {
    std::thread::sleep(interval);
    let started = Instant::now();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
    ms.push(started.elapsed().as_secs_f64() * 1000.0);
  }
  let mut summary = summarize(ms);
  summary["processes"] = json!(system.processes().len());
  summary
}

#[test]
#[ignore = "evidence probe: run explicitly with --ignored"]
fn process_gpu_probe() {
  let samples = env_u64("HV_PROBE_GPU_SAMPLES", 20).max(2);
  let interval = Duration::from_millis(env_u64("HV_PROBE_GPU_INTERVAL_MS", 3000));
  // Only used to shorten the baseline/combined phases; their per-refresh cost
  // does not depend on the interval as long as it exceeds the refresh time.
  let quick = Duration::from_millis(500);

  let baseline = time_refreshes(
    ProcessRefreshKind::nothing().with_cpu().with_memory(),
    samples,
    quick,
  );
  let combined = time_refreshes(
    ProcessRefreshKind::nothing()
      .with_cpu()
      .with_memory()
      .with_gpu_usage(),
    samples,
    quick,
  );

  // Production path, at the Live Process Table cadence. The first read has
  // no baseline and is excluded from both timing and coverage.
  let collector = LiveProcessGpuCollector::new();
  collector.read();
  let mut names = System::new().expect("sysinfo System");
  let mut read_ms = Vec::new();
  let mut coverage = Vec::new();
  let mut previous: HashMap<(u32, u64), f32> = HashMap::new();
  for _ in 0..samples {
    std::thread::sleep(interval);
    let started = Instant::now();
    let reading = collector.read();
    read_ms.push(started.elapsed().as_secs_f64() * 1000.0);

    let repeated_nonzero = reading
      .iter()
      .filter(|(key, usage)| {
        **usage != 0.0
          && previous
            .get(key)
            .is_some_and(|prev| prev.to_bits() == usage.to_bits())
      })
      .count();
    coverage.push(json!({
      "with_value": reading.len(),
      "nonzero": reading.values().filter(|u| **u > 0.0).count(),
      "over_100": reading.values().filter(|u| **u > 100.0).count(),
      "repeated_nonzero": repeated_nonzero,
    }));
    previous = (*reading).clone();
  }

  names.refresh_processes_specifics(
    ProcessesToUpdate::All,
    true,
    ProcessRefreshKind::nothing(),
  );
  let mut top: Vec<_> = previous
    .iter()
    .filter(|(_, usage)| **usage > 0.0)
    .map(|((pid, _), usage)| {
      let name = names
        .process(sysinfo::Pid::from_u32(*pid))
        .map(|p| p.name().to_string_lossy().into_owned())
        .unwrap_or_default();
      json!({ "pid": pid, "name": name, "gpu_usage": usage })
    })
    .collect();
  top.sort_by(|a, b| {
    b["gpu_usage"]
      .as_f64()
      .unwrap_or(0.0)
      .total_cmp(&a["gpu_usage"].as_f64().unwrap_or(0.0))
  });
  top.truncate(10);

  let mut live_read = summarize(read_ms);
  live_read["interval_ms"] = json!(interval.as_millis() as u64);
  live_read["processes"] = json!(names.processes().len());

  let report = json!({
    "os": std::env::consts::OS,
    "arch": std::env::consts::ARCH,
    "os_version": System::long_os_version().ok(),
    "cpu_count": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
    "baseline_cpu_memory": baseline,
    "combined_cpu_memory_gpu": combined,
    "live_read": live_read,
    "coverage_per_read": coverage,
    "top_last_read": top,
  });
  let text = serde_json::to_string_pretty(&report).expect("serialize report");
  match std::env::var("HV_PROCESS_GPU_PROBE_OUT") {
    Ok(path) => std::fs::write(&path, &text).expect("write probe output"),
    Err(_) => println!("{text}"),
  }
}
