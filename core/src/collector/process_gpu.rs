//! Per-process GPU usage collection.
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
//! - If the previous refresh is older than [`MAX_BASELINE_AGE`]
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
//!   in one measurement); all others are `None`. sysinfo keeps a process's
//!   old value once it leaves the GPU client list, so repeated nonzero
//!   values are dropped as retained (see `drop_retained_readings`).
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
pub const MAX_BASELINE_AGE: Duration = Duration::from_secs(10);

/// Process identity: `(pid, start_time in seconds since the epoch)`.
pub type ProcessGpuKey = (u32, u64);

/// GPU usage (%) per process. Processes without a value are absent.
pub type ProcessGpuReadings = HashMap<ProcessGpuKey, f32>;

struct CollectorState {
  /// `None` when sysinfo could not initialize; every read is then empty.
  system: Option<System>,
  /// Raw sysinfo values from the previous refresh, used to detect values
  /// sysinfo retained instead of recomputing (see [`drop_retained_readings`]).
  previous_raw: ProcessGpuReadings,
  last_reading: Option<(Instant, Arc<ProcessGpuReadings>)>,
  /// Completion time of the last actual refresh; cache hits do not move it.
  last_refresh: Option<Instant>,
}

/// Owner of the per-process GPU usage refresh state. Never touches the
/// database.
pub struct ProcessGpuCollector {
  state: Mutex<CollectorState>,
}

impl Default for ProcessGpuCollector {
  fn default() -> Self {
    Self::new()
  }
}

impl ProcessGpuCollector {
  pub fn new() -> Self {
    Self {
      state: Mutex::new(CollectorState {
        system: System::new().ok(),
        previous_raw: ProcessGpuReadings::new(),
        last_reading: None,
        last_refresh: None,
      }),
    }
  }

  /// Reads per-process GPU usage, returning the last reading when called
  /// again within [`LIVE_READ_MIN_INTERVAL`]. Blocking: callers on an async
  /// runtime must use `spawn_blocking`.
  pub fn read(&self) -> Arc<ProcessGpuReadings> {
    self.read_with(Instant::now, refresh_gpu_usage)
  }

  fn read_with(
    &self,
    clock: impl Fn() -> Instant,
    collect: impl FnOnce(&mut System, &mut ProcessGpuReadings) -> ProcessGpuReadings,
  ) -> Arc<ProcessGpuReadings> {
    // The lock is held across the refresh on purpose: concurrent callers
    // serialize here, and whoever loses the race finds a fresh reading
    // inside the interval guard instead of refreshing again.
    let mut state = self.state.lock().unwrap();
    let requested_at = clock();

    if let Some((collected_at, reading)) = &state.last_reading
      && requested_at.duration_since(*collected_at) < LIVE_READ_MIN_INTERVAL
    {
      return Arc::clone(reading);
    }

    let (completed_at, readings) = state.refresh(requested_at, clock, collect);
    let reading = Arc::new(readings);
    state.last_reading = Some((completed_at, Arc::clone(&reading)));
    reading
  }
}

impl CollectorState {
  /// Refreshes GPU usage and returns the completion time with the readings
  /// for the interval since the previous refresh. The readings are empty when
  /// that refresh is older than [`MAX_BASELINE_AGE`].
  fn refresh(
    &mut self,
    requested_at: Instant,
    clock: impl Fn() -> Instant,
    collect: impl FnOnce(&mut System, &mut ProcessGpuReadings) -> ProcessGpuReadings,
  ) -> (Instant, ProcessGpuReadings) {
    // The refresh always runs so the baseline moves forward, but a stale
    // baseline's values span the idle gap and are discarded.
    let stale_baseline = self
      .last_refresh
      .is_some_and(|at| requested_at.duration_since(at) > MAX_BASELINE_AGE);
    let collected = match self.system.as_mut() {
      Some(system) => collect(system, &mut self.previous_raw),
      None => ProcessGpuReadings::new(),
    };
    // Stamp with the completion time: the refresh itself can take a large
    // part of the guard interval, and the delta window ends here.
    let completed_at = clock();
    self.last_refresh = Some(completed_at);
    let readings = if stale_baseline {
      ProcessGpuReadings::new()
    } else {
      collected
    };
    (completed_at, readings)
  }
}

/// Refreshes only GPU usage for every process and returns the readings
/// sysinfo computed for the interval since the previous refresh.
fn refresh_gpu_usage(
  system: &mut System,
  previous_raw: &mut ProcessGpuReadings,
) -> ProcessGpuReadings {
  system.refresh_processes_specifics(
    ProcessesToUpdate::All,
    true,
    ProcessRefreshKind::nothing().with_gpu_usage(),
  );
  let raw: ProcessGpuReadings = system
    .processes()
    .iter()
    .filter_map(|(pid, process)| {
      process
        .gpu_usage()
        .map(|usage| ((pid.as_u32(), process.start_time()), usage))
    })
    .collect();
  let previous = std::mem::replace(previous_raw, raw.clone());
  if cfg!(target_os = "macos") {
    drop_retained_readings(raw, &previous)
  } else {
    raw
  }
}

/// Drops values sysinfo 0.40.1 retained on macOS instead of recomputing.
///
/// sysinfo's macOS backend only updates `gpu_usage` for processes found in
/// the current IORegistry walk; a process that stopped being a GPU client,
/// or every process when the walk fails, keeps its previous value forever.
/// The public API cannot tell a retained value from a fresh one, but a
/// recomputed nonzero value is a ratio over a nanosecond-resolution window
/// and does not repeat bit-for-bit, so a nonzero value identical to the
/// previous refresh is treated as retained and dropped (unavailable).
/// A repeated 0 is kept: it is indistinguishable from a real idle reading
/// and claims no GPU load.
///
/// Known gaps, accepted until sysinfo exposes which processes the current
/// walk observed: a retained 0 still reads as 0% (for example when the
/// IORegistry walk fails), and two fresh nonzero readings that happen to
/// round to the same `f32` are dropped for one poll.
///
/// Why not an own IORegistry walk: it would duplicate sysinfo's unsafe
/// macOS GPU code for this one signal. The intended fix is upstream (reset
/// `gpu_usage` for processes the walk did not find); remove this filter
/// once a sysinfo release does that.
fn drop_retained_readings(
  raw: ProcessGpuReadings,
  previous: &ProcessGpuReadings,
) -> ProcessGpuReadings {
  raw
    .into_iter()
    .filter(|(key, usage)| {
      *usage == 0.0
        || previous
          .get(key)
          .is_none_or(|prev| prev.to_bits() != usage.to_bits())
    })
    .collect()
}

/// Live process list: [`HistoryStore::process_list`] plus per-process GPU
/// usage. Blocking (see [`ProcessGpuCollector::read`]).
pub fn live_process_list(
  history: &HistoryStore,
  gpu: &ProcessGpuCollector,
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
    let collector = ProcessGpuCollector::new();
    let t0 = Instant::now();
    let mut refreshes = 0;

    let first = collector.read_with(
      || t0,
      |_, _| {
        refreshes += 1;
        HashMap::from([((1, 1), 7.0)])
      },
    );
    let cached = collector.read_with(
      || t0 + LIVE_READ_MIN_INTERVAL / 2,
      |_, _| {
        refreshes += 1;
        HashMap::new()
      },
    );
    let fresh = collector.read_with(
      || t0 + LIVE_READ_MIN_INTERVAL,
      |_, _| {
        refreshes += 1;
        HashMap::new()
      },
    );

    assert_eq!(first.get(&(1, 1)), Some(&7.0));
    assert_eq!(cached.get(&(1, 1)), Some(&7.0));
    assert!(fresh.is_empty());
    assert_eq!(refreshes, 2);
  }

  #[test]
  fn refresh_after_a_gap_longer_than_max_baseline_age_only_rebaselines() {
    let collector = ProcessGpuCollector::new();
    let t0 = Instant::now();
    let values = || HashMap::from([((1, 1), 7.0)]);

    collector.read_with(|| t0, |_, _| values());
    let after_gap = collector.read_with(|| t0 + MAX_BASELINE_AGE * 2, |_, _| values());
    let next = collector.read_with(
      || t0 + MAX_BASELINE_AGE * 2 + LIVE_READ_MIN_INTERVAL,
      |_, _| values(),
    );

    assert!(after_gap.is_empty(), "stale baseline reads as measuring");
    assert_eq!(next.get(&(1, 1)), Some(&7.0));
  }

  #[test]
  fn guard_window_starts_when_the_refresh_completes() {
    let collector = ProcessGpuCollector::new();
    let t0 = Instant::now();
    let slow_refresh = Duration::from_millis(1500);
    let ticks = std::cell::Cell::new(0);
    // First call: requested at t0, refresh completes at t0 + 1.5 s.
    let clock = || {
      ticks.set(ticks.get() + 1);
      if ticks.get() == 1 {
        t0
      } else {
        t0 + slow_refresh
      }
    };
    collector.read_with(clock, |_, _| HashMap::from([((1, 1), 7.0)]));

    let mut refreshed = false;
    let right_after = collector.read_with(
      || t0 + slow_refresh + LIVE_READ_MIN_INTERVAL / 2,
      |_, _| {
        refreshed = true;
        HashMap::new()
      },
    );

    assert!(
      !refreshed,
      "a call just after a slow refresh must hit the cache"
    );
    assert_eq!(right_after.get(&(1, 1)), Some(&7.0));
  }

  #[test]
  fn nonzero_value_repeated_bit_for_bit_is_treated_as_retained() {
    let previous: ProcessGpuReadings =
      HashMap::from([((1, 1), 12.5), ((2, 2), 0.0), ((3, 3), 4.0)]);
    let raw: ProcessGpuReadings =
      HashMap::from([((1, 1), 12.5), ((2, 2), 0.0), ((3, 3), 4.1), ((4, 4), 9.0)]);

    let kept = drop_retained_readings(raw, &previous);

    assert_eq!(kept.get(&(1, 1)), None, "retained load is not fresh");
    assert_eq!(kept.get(&(2, 2)), Some(&0.0), "repeated idle stays idle");
    assert_eq!(kept.get(&(3, 3)), Some(&4.1));
    assert_eq!(kept.get(&(4, 4)), Some(&9.0), "first value has no previous");
  }
}
