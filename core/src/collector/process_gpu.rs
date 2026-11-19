//! Per-process GPU usage collection.
//!
//! Per-process GPU usage for the Live Process Table and, while it demands
//! it, a background consumer such as the Hardware Archive (#2720). It stays
//! out of the 1 Hz collector's process refresh (`sampling::sample_system`)
//! so it costs nothing until a consumer asks: the table polls
//! [`ProcessGpuCollector::read`], and a background consumer turns on
//! [`ProcessGpuCollector::set_background_demand`], after which the collector
//! loop queues a refresh every [`BACKGROUND_REFRESH_INTERVAL`] on a blocking
//! thread without waiting for it, and each snapshot carries the usage
//! accumulated since the previous one. Both consumers share
//! one refresh state, so a refresh made for either serves the other.
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::collector::HistoryStore;
use crate::models::ProcessGpuUsageSpan;
use crate::models::hardware::ProcessInfo;
use crate::utils::rounding;

/// Minimum interval between GPU refreshes. Calls inside the window return
/// the cached reading.
pub const LIVE_READ_MIN_INTERVAL: Duration = Duration::from_secs(1);

/// A refresh older than this is stale: the next one only re-baselines.
pub const MAX_BASELINE_AGE: Duration = Duration::from_secs(15);

/// Interval between refreshes while a background consumer demands readings.
/// Readings are averages over the interval, so a longer interval loses
/// nothing for long-lived processes; it bounds how long a short-lived process
/// can run unseen.
pub const BACKGROUND_REFRESH_INTERVAL: Duration = Duration::from_secs(10);

// A background refresh starts on a 1 s collector tick after the interval
// elapsed and the previous refresh itself took time. Its baseline must still
// count as fresh, or every background reading would be discarded.
const _: () =
  assert!(MAX_BASELINE_AGE.as_secs() > BACKGROUND_REFRESH_INTERVAL.as_secs() + 2);

/// Process identity: `(pid, start_time in seconds since the epoch)`.
pub type ProcessGpuKey = (u32, u64);

/// GPU usage (%) per process. Processes without a value are absent.
pub type ProcessGpuReadings = HashMap<ProcessGpuKey, f32>;

/// GPU usage accumulated per process over refresh intervals.
pub type ProcessGpuUsage = HashMap<ProcessGpuKey, ProcessGpuUsageSpan>;

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

/// Usage accumulated for a background consumer. Every refresh, whoever
/// triggered it, adds its interval here until the consumer takes it, so no
/// interval is counted twice or skipped.
struct BackgroundAccumulator {
  /// When the demand started. An interval that began earlier is not counted:
  /// part of it passed while nobody demanded usage.
  since: Instant,
  usage: ProcessGpuUsage,
}

/// Owner of the per-process GPU usage refresh state. Never touches the
/// database.
pub struct ProcessGpuCollector {
  /// Held across a sysinfo refresh, which can take a large fraction of a
  /// second; only blocking callers take it.
  state: Mutex<CollectorState>,
  /// `Some` while a background consumer demands usage. Held only to update
  /// or swap the map, never across a refresh, so the collector loop can take
  /// it on its async worker without waiting for a refresh in progress.
  background: Mutex<Option<BackgroundAccumulator>>,
  /// Set while a background refresh is queued or running, so the collector
  /// loop does not queue another behind it.
  background_refresh_pending: AtomicBool,
}

/// A queued background refresh. Run it on a blocking thread; dropping it,
/// run or not, lets the next one be queued.
pub struct BackgroundRefresh {
  collector: Arc<ProcessGpuCollector>,
}

impl BackgroundRefresh {
  /// Refreshes when the last refresh is at least
  /// [`BACKGROUND_REFRESH_INTERVAL`] old. Blocking.
  pub fn run(self) {
    self
      .collector
      .refresh_background_with(Instant::now, refresh_gpu_usage);
  }
}

impl Drop for BackgroundRefresh {
  fn drop(&mut self) {
    self
      .collector
      .background_refresh_pending
      .store(false, Ordering::Release);
  }
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
      background: Mutex::new(None),
      background_refresh_pending: AtomicBool::new(false),
    }
  }

  /// Starts or stops background accumulation. Stopping drops what the
  /// consumer has not taken yet; starting counts only intervals that begin
  /// after it.
  pub fn set_background_demand(&self, demanded: bool) {
    self.set_background_demand_at(demanded, Instant::now());
  }

  fn set_background_demand_at(&self, demanded: bool, now: Instant) {
    let mut background = self.background.lock().unwrap();
    match (demanded, background.is_some()) {
      (true, false) => {
        *background = Some(BackgroundAccumulator {
          since: now,
          usage: ProcessGpuUsage::new(),
        });
      }
      (false, true) => *background = None,
      _ => {}
    }
  }

  /// Queues a background refresh while background demand is on and none is
  /// already queued. Never waits for a refresh in progress, so the collector
  /// loop can call it on its async worker.
  pub fn begin_background_refresh(self: &Arc<Self>) -> Option<BackgroundRefresh> {
    if self.background.lock().unwrap().is_none()
      || self.background_refresh_pending.swap(true, Ordering::AcqRel)
    {
      return None;
    }
    Some(BackgroundRefresh {
      collector: Arc::clone(self),
    })
  }

  fn refresh_background_with(
    &self,
    clock: impl Fn() -> Instant,
    collect: impl FnOnce(&mut System, &mut ProcessGpuReadings) -> ProcessGpuReadings,
  ) {
    let mut state = self.state.lock().unwrap();
    let requested_at = clock();
    // Checked under the lock: a live read may have refreshed meanwhile.
    let due = state
      .last_refresh
      .is_none_or(|at| requested_at.duration_since(at) >= BACKGROUND_REFRESH_INTERVAL);
    if !due || self.background.lock().unwrap().is_none() {
      return;
    }
    let (completed_at, readings) =
      state.refresh(requested_at, clock, collect, &self.background);
    // The reading also serves the next live read inside the guard window.
    state.last_reading = Some((completed_at, Arc::new(readings)));
  }

  /// Takes the usage accumulated since the previous call. Empty while no
  /// background consumer demands it. Never waits for a refresh in progress.
  pub fn take_background_usage(&self) -> ProcessGpuUsage {
    self
      .background
      .lock()
      .unwrap()
      .as_mut()
      .map(|background| std::mem::take(&mut background.usage))
      .unwrap_or_default()
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

    let (completed_at, readings) =
      state.refresh(requested_at, clock, collect, &self.background);
    let reading = Arc::new(readings);
    state.last_reading = Some((completed_at, Arc::clone(&reading)));
    reading
  }
}

impl CollectorState {
  /// Refreshes GPU usage and returns the completion time with the readings
  /// for the interval since the previous refresh. The readings are empty when
  /// that refresh is older than [`MAX_BASELINE_AGE`]. While background demand
  /// is on, the readings are also accumulated, weighted by the interval.
  fn refresh(
    &mut self,
    requested_at: Instant,
    clock: impl Fn() -> Instant,
    collect: impl FnOnce(&mut System, &mut ProcessGpuReadings) -> ProcessGpuReadings,
    background: &Mutex<Option<BackgroundAccumulator>>,
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
    let interval_start = self.last_refresh.replace(completed_at);
    let readings = if stale_baseline {
      ProcessGpuReadings::new()
    } else {
      collected
    };
    if let Some(start) = interval_start
      && let Some(background) = background.lock().unwrap().as_mut()
      && start >= background.since
    {
      let seconds = completed_at.duration_since(start).as_secs_f64();
      for (key, usage) in &readings {
        background
          .usage
          .entry(*key)
          .or_default()
          .add(*usage, seconds);
      }
    }
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

  fn demanded_collector() -> ProcessGpuCollector {
    let collector = ProcessGpuCollector::new();
    collector.set_background_demand(true);
    collector
  }

  #[test]
  fn background_usage_is_weighted_by_each_refresh_interval() {
    let collector = demanded_collector();
    let t0 = Instant::now();
    let t1 = t0 + BACKGROUND_REFRESH_INTERVAL;
    let t2 = t1 + Duration::from_secs(5);

    // The first refresh only sets the baseline; it covers no interval.
    collector.refresh_background_with(|| t0, |_, _| HashMap::from([((1, 1), 99.0)]));
    collector.refresh_background_with(|| t1, |_, _| HashMap::from([((1, 1), 30.0)]));
    // A live read is another refresh of the same state and is counted too.
    collector.read_with(|| t2, |_, _| HashMap::from([((1, 1), 60.0), ((2, 2), 0.0)]));

    let usage = collector.take_background_usage();
    let first = usage[&(1, 1)];
    assert_eq!(first.covered_seconds, 15.0);
    assert_eq!(
      first.average_percent(),
      Some((30.0 * 10.0 + 60.0 * 5.0) / 15.0)
    );
    assert_eq!(
      usage[&(2, 2)].average_percent(),
      Some(0.0),
      "a reading of 0 is kept as 0"
    );
    assert!(
      collector.take_background_usage().is_empty(),
      "taking drains what was accumulated"
    );
  }

  #[test]
  fn nothing_accumulates_without_background_demand() {
    let collector = Arc::new(ProcessGpuCollector::new());
    let t0 = Instant::now();

    collector.read_with(|| t0, |_, _| HashMap::from([((1, 1), 7.0)]));
    collector.read_with(
      || t0 + LIVE_READ_MIN_INTERVAL,
      |_, _| HashMap::from([((1, 1), 7.0)]),
    );

    assert!(collector.begin_background_refresh().is_none());
    assert!(collector.take_background_usage().is_empty());
  }

  #[test]
  fn a_stale_baseline_adds_no_background_interval() {
    let collector = demanded_collector();
    let t0 = Instant::now();

    collector.refresh_background_with(|| t0, |_, _| HashMap::from([((1, 1), 7.0)]));
    collector.refresh_background_with(
      || t0 + MAX_BASELINE_AGE * 2,
      |_, _| HashMap::from([((1, 1), 7.0)]),
    );

    assert!(collector.take_background_usage().is_empty());
  }

  #[test]
  fn background_refresh_waits_for_its_interval_after_any_refresh() {
    let collector = demanded_collector();
    let t0 = Instant::now();
    let mut refreshes = 0;

    collector.read_with(|| t0, |_, _| HashMap::new());
    collector.refresh_background_with(
      || t0 + BACKGROUND_REFRESH_INTERVAL / 2,
      |_, _| {
        refreshes += 1;
        HashMap::new()
      },
    );
    collector.refresh_background_with(
      || t0 + BACKGROUND_REFRESH_INTERVAL,
      |_, _| {
        refreshes += 1;
        HashMap::new()
      },
    );

    assert_eq!(
      refreshes, 1,
      "a live read moves the background schedule too"
    );
  }

  #[test]
  fn a_background_refresh_serves_the_next_live_read() {
    let collector = demanded_collector();
    let t0 = Instant::now();
    collector.refresh_background_with(|| t0, |_, _| HashMap::new());
    collector.refresh_background_with(
      || t0 + BACKGROUND_REFRESH_INTERVAL,
      |_, _| HashMap::from([((1, 1), 7.0)]),
    );

    let mut refreshed = false;
    let live = collector.read_with(
      || t0 + BACKGROUND_REFRESH_INTERVAL + LIVE_READ_MIN_INTERVAL / 2,
      |_, _| {
        refreshed = true;
        HashMap::new()
      },
    );

    assert!(!refreshed);
    assert_eq!(live.get(&(1, 1)), Some(&7.0));
  }

  #[test]
  fn withdrawing_demand_drops_untaken_usage() {
    let collector = demanded_collector();
    let t0 = Instant::now();
    collector.refresh_background_with(|| t0, |_, _| HashMap::new());
    collector.refresh_background_with(
      || t0 + BACKGROUND_REFRESH_INTERVAL,
      |_, _| HashMap::from([((1, 1), 7.0)]),
    );

    collector.set_background_demand(false);
    collector.set_background_demand(true);

    assert!(collector.take_background_usage().is_empty());
  }

  #[test]
  fn an_interval_that_began_before_the_demand_is_not_counted() {
    let collector = ProcessGpuCollector::new();
    let t0 = Instant::now();
    let reading = || HashMap::from([((1, 1), 50.0)]);
    let t1 = t0 + BACKGROUND_REFRESH_INTERVAL;
    let t2 = t1 + BACKGROUND_REFRESH_INTERVAL;
    let t3 = t2 + BACKGROUND_REFRESH_INTERVAL;

    collector.read_with(|| t0, |_, _| reading());
    collector.set_background_demand_at(true, t0 + Duration::from_secs(9));
    collector.refresh_background_with(|| t1, |_, _| reading());
    assert!(
      collector.take_background_usage().is_empty(),
      "nine of these ten seconds passed without demand"
    );

    collector.refresh_background_with(|| t2, |_, _| reading());
    assert_eq!(
      collector.take_background_usage()[&(1, 1)].covered_seconds,
      BACKGROUND_REFRESH_INTERVAL.as_secs_f64()
    );

    collector.set_background_demand_at(false, t2);
    collector.set_background_demand_at(true, t2 + LIVE_READ_MIN_INTERVAL);
    collector.refresh_background_with(|| t3, |_, _| reading());
    assert!(
      collector.take_background_usage().is_empty(),
      "re-enabling demand starts a new baseline too"
    );
  }

  #[test]
  fn only_one_background_refresh_is_queued_at_a_time() {
    let collector = Arc::new(ProcessGpuCollector::new());
    assert!(collector.begin_background_refresh().is_none(), "no demand");

    collector.set_background_demand(true);
    let queued = collector.begin_background_refresh();
    assert!(queued.is_some());
    assert!(
      collector.begin_background_refresh().is_none(),
      "one is already pending"
    );

    drop(queued);
    assert!(collector.begin_background_refresh().is_some());
  }

  #[test]
  fn the_collector_loop_never_waits_for_a_refresh_in_progress() {
    let collector = Arc::new(demanded_collector());
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let reader = {
      let collector = Arc::clone(&collector);
      std::thread::spawn(move || {
        collector.read_with(Instant::now, |_, _| {
          started_tx.send(()).unwrap();
          release_rx.recv().unwrap();
          HashMap::new()
        });
      })
    };
    started_rx.recv().unwrap();

    // Both calls the loop makes on its async worker return while the live
    // read still holds the refresh state.
    assert!(collector.take_background_usage().is_empty());
    assert!(collector.begin_background_refresh().is_some());

    release_tx.send(()).unwrap();
    reader.join().unwrap();
  }
}
