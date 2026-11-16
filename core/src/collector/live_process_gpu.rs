//! Live Process GPU usage on-demand collection.
//!
//! Per-process GPU usage for the Live Process Table, collected only while
//! that table polls and never persisted. It is deliberately not part of the
//! 1 Hz collector's process refresh (`sampling::sample_system`): adding it
//! there would make it always-on and feed the archive.
//!
//! - The collector owns its own `sysinfo::System`, separate from the
//!   [`HistoryStore`]'s, so a read never contends with the collector's lock
//!   and never disturbs its CPU deltas. GPU usage is a delta between two
//!   refreshes of the same `System`; the first read has no baseline, so
//!   every process reads as unavailable until the second.
//! - A read refreshes only GPU usage for all processes. This is costly
//!   (about 160 ms for ~1000 processes on macOS), so callers must run it off
//!   the UI/main thread (`spawn_blocking`).
//! - A minimum-interval guard ([`LIVE_READ_MIN_INTERVAL`]) returns the cached
//!   reading for calls inside the window, so near-zero delta windows do not
//!   produce noisy values.
//! - If the previous refresh is older than [`LIVE_READ_MAX_BASELINE_AGE`]
//!   (the table polls every 3 s, so the table was hidden), the new refresh
//!   only re-baselines and returns an empty reading ("measuring"). Otherwise
//!   the first value would average over the hidden period.
//! - Readings are keyed by `(pid, start_time)` so a recycled PID never
//!   inherits another process's value.
//!
//! Meaning of a value: the sum across all GPUs and engines the OS reports,
//! in percent. It is not clamped and can exceed 100; it can differ from Task
//! Manager, which shows the busiest engine. `None` means unavailable or still
//! measuring, never zero:
//! - macOS: only processes registered as GPU clients get a value (18 of 984
//!   in one measurement); all others are `None`.
//! - Linux: a GPU-client process whose engines had zero delta since the
//!   previous refresh reads `None` rather than 0.
//! - Windows: sysinfo adds a process's counter on its first GPU refresh and
//!   reads it from the next one, so a newly seen process is `None` for one
//!   read.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::collector::HistoryStore;
use crate::models::hardware::ProcessInfo;
use crate::utils::rounding;

/// Minimum interval between GPU refreshes. Calls inside the window return
/// the cached reading.
pub const LIVE_READ_MIN_INTERVAL: Duration = Duration::from_secs(1);

/// A refresh older than this is stale: the next one only re-baselines.
pub const LIVE_READ_MAX_BASELINE_AGE: Duration = Duration::from_secs(10);

/// Process identity: `(pid, start_time in seconds since the epoch)`.
pub type ProcessGpuKey = (u32, u64);

/// GPU usage (%) per process. Processes without a value are absent.
pub type ProcessGpuReadings = HashMap<ProcessGpuKey, f32>;

struct CollectorState {
  /// `None` when sysinfo could not initialize; every read is then empty.
  system: Option<System>,
  last_reading: Option<(Instant, Arc<ProcessGpuReadings>)>,
  /// Time of the last actual refresh; cache hits do not move it.
  last_refresh: Option<Instant>,
}

/// On-demand Live Process GPU collector. Never touches the database.
pub struct LiveProcessGpuCollector {
  state: Mutex<CollectorState>,
}

impl Default for LiveProcessGpuCollector {
  fn default() -> Self {
    Self::new()
  }
}

impl LiveProcessGpuCollector {
  pub fn new() -> Self {
    Self {
      state: Mutex::new(CollectorState {
        system: System::new().ok(),
        last_reading: None,
        last_refresh: None,
      }),
    }
  }

  /// Reads per-process GPU usage, returning the last reading when called
  /// again within [`LIVE_READ_MIN_INTERVAL`]. Blocking: callers on an async
  /// runtime must use `spawn_blocking`.
  pub fn read(&self) -> Arc<ProcessGpuReadings> {
    self.read_with(Instant::now(), |system| {
      system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_gpu_usage(),
      );
      system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
          process
            .gpu_usage()
            .map(|usage| ((pid.as_u32(), process.start_time()), usage))
        })
        .collect()
    })
  }

  fn read_with(
    &self,
    now: Instant,
    collect: impl FnOnce(&mut System) -> ProcessGpuReadings,
  ) -> Arc<ProcessGpuReadings> {
    // The lock is held across the refresh on purpose: concurrent callers
    // serialize here, and whoever loses the race finds a fresh reading
    // inside the interval guard instead of refreshing again.
    let mut state = self.state.lock().unwrap();

    if let Some((collected_at, reading)) = &state.last_reading
      && now.duration_since(*collected_at) < LIVE_READ_MIN_INTERVAL
    {
      return Arc::clone(reading);
    }

    // The refresh always runs so the baseline moves forward, but a stale
    // baseline's values span the idle gap and are discarded.
    let stale_baseline = state
      .last_refresh
      .is_some_and(|at| now.duration_since(at) > LIVE_READ_MAX_BASELINE_AGE);
    let collected = match state.system.as_mut() {
      Some(system) => collect(system),
      None => ProcessGpuReadings::new(),
    };
    state.last_refresh = Some(now);
    let reading = Arc::new(if stale_baseline {
      ProcessGpuReadings::new()
    } else {
      collected
    });
    state.last_reading = Some((now, Arc::clone(&reading)));
    reading
  }
}

/// Live process list: [`HistoryStore::process_list`] plus per-process GPU
/// usage. Blocking (see [`LiveProcessGpuCollector::read`]).
pub fn live_process_list(
  history: &HistoryStore,
  gpu: &LiveProcessGpuCollector,
) -> Vec<ProcessInfo> {
  let readings = gpu.read();
  attach_gpu_usage(history.process_list_with_start_times(), &readings)
}

/// Joins GPU readings onto processes by `(pid, start_time)`. A missing key
/// stays `None`; it is never converted to 0.
fn attach_gpu_usage(
  processes: Vec<(u64, ProcessInfo)>,
  readings: &ProcessGpuReadings,
) -> Vec<ProcessInfo> {
  processes
    .into_iter()
    .map(|(start_time, mut info)| {
      info.gpu_usage = readings
        .get(&(info.pid as u32, start_time))
        .copied()
        .map(rounding::round1);
      info
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn process(pid: i32) -> ProcessInfo {
    ProcessInfo {
      pid,
      name: format!("p{pid}"),
      cpu_usage: 0.0,
      memory_usage: 0.0,
      gpu_usage: None,
    }
  }

  #[test]
  fn reused_pid_with_different_start_time_gets_no_gpu_usage() {
    let readings: ProcessGpuReadings = HashMap::from([((10, 1000), 42.0)]);

    let joined = attach_gpu_usage(
      vec![
        (1000, process(10)),
        (2000, process(10)),
        (1000, process(11)),
      ],
      &readings,
    );

    assert_eq!(joined[0].gpu_usage, Some(42.0));
    assert_eq!(joined[1].gpu_usage, None, "same PID, newer process");
    assert_eq!(joined[2].gpu_usage, None, "no reading is not zero");
  }

  #[test]
  fn usage_above_one_hundred_is_kept_and_rounded() {
    let readings: ProcessGpuReadings = HashMap::from([((1, 5), 130.26)]);
    let joined = attach_gpu_usage(vec![(5, process(1))], &readings);
    assert_eq!(joined[0].gpu_usage, Some(130.3));
  }

  #[test]
  fn read_inside_min_interval_returns_cached_reading_without_refreshing() {
    let collector = LiveProcessGpuCollector::new();
    let t0 = Instant::now();
    let mut refreshes = 0;

    let first = collector.read_with(t0, |_| {
      refreshes += 1;
      HashMap::from([((1, 1), 7.0)])
    });
    let cached = collector.read_with(t0 + LIVE_READ_MIN_INTERVAL / 2, |_| {
      refreshes += 1;
      HashMap::new()
    });
    let fresh = collector.read_with(t0 + LIVE_READ_MIN_INTERVAL, |_| {
      refreshes += 1;
      HashMap::new()
    });

    assert_eq!(first.get(&(1, 1)), Some(&7.0));
    assert_eq!(cached.get(&(1, 1)), Some(&7.0));
    assert!(fresh.is_empty());
    assert_eq!(refreshes, 2);
  }

  #[test]
  fn refresh_after_a_gap_longer_than_max_baseline_age_only_rebaselines() {
    let collector = LiveProcessGpuCollector::new();
    let t0 = Instant::now();
    let values = || HashMap::from([((1, 1), 7.0)]);

    collector.read_with(t0, |_| values());
    let after_gap =
      collector.read_with(t0 + LIVE_READ_MAX_BASELINE_AGE * 2, |_| values());
    let next = collector.read_with(
      t0 + LIVE_READ_MAX_BASELINE_AGE * 2 + LIVE_READ_MIN_INTERVAL,
      |_| values(),
    );

    assert!(after_gap.is_empty(), "stale baseline reads as measuring");
    assert_eq!(next.get(&(1, 1)), Some(&7.0));
  }
}
