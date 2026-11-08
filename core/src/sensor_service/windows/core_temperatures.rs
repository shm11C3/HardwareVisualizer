use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant as ThreadDeadline};

use tokio::sync::{Mutex, oneshot};
use tokio::time::{Instant, timeout_at};

use super::{providers::CPU_INTERVAL_MS, uptime_ms};
use crate::infrastructure::providers::windows::cpu_core_temperature::{
  CoreTopology, acquire, read_topology, supported_model,
};
use crate::models::CpuCoreTemperature;
use crate::sensor_service::protocol::Reading;

// Keep the core request within the existing two-second pipe exchange.
const CORE_WAIT_BUDGET: Duration = Duration::from_millis(750);
static INVALIDATION_EPOCH: AtomicU64 = AtomicU64::new(0);

type CoreReading = Reading<Vec<CpuCoreTemperature>>;

pub(super) fn invalidate() {
  INVALIDATION_EPOCH.fetch_add(1, Ordering::AcqRel);
}

#[derive(Default)]
pub(super) struct CoreTemperatures {
  // Separate from the ordinary batch so even a timed-out PawnIO call cannot
  // hold the package, power or motherboard providers' locks.
  state: Mutex<State>,
}

#[derive(Default)]
struct State {
  cache: Option<(Context, CoreReading)>,
  worker: Option<JoinHandle<()>>,
  last_attempt_ms: Option<u64>,
}

#[derive(PartialEq, Eq)]
struct Context {
  model: u32,
  topology: CoreTopology,
  epoch: u64,
}

impl CoreTemperatures {
  pub(super) async fn request(&self) -> Option<CoreReading> {
    self
      .sample(supported_model()?, read_topology, acquire, CORE_WAIT_BUDGET)
      .await
  }

  async fn sample(
    &self,
    model: u32,
    read_topology: impl Fn() -> Option<CoreTopology>,
    acquire: impl FnOnce(
      u32,
      &CoreTopology,
      ThreadDeadline,
    ) -> Option<Vec<CpuCoreTemperature>>
    + Send
    + 'static,
    budget: Duration,
  ) -> Option<CoreReading> {
    let deadline = Instant::now() + budget;
    // Observe topology even if concurrent demand consumes this request's
    // wait budget. When queued, observe again before sharing a result.
    let topology = read_topology();
    let mut state = match self.state.try_lock() {
      Ok(state) => state,
      Err(_) => {
        let mut state = timeout_at(deadline, self.state.lock()).await.ok()?;
        if read_topology() != topology {
          state.cache = None;
          return None;
        }
        state
      }
    };
    let epoch = INVALIDATION_EPOCH.load(Ordering::Acquire);
    let Some(topology) = topology else {
      state.cache = None;
      return None;
    };
    let context = Context {
      model,
      topology,
      epoch,
    };
    let now = uptime_ms();
    if let Some((cached_context, reading)) = &state.cache
      && cached_context == &context
      && reading.is_fresh(now)
      && now - reading.acquired_at_ms < CPU_INTERVAL_MS
      && INVALIDATION_EPOCH.load(Ordering::Acquire) == epoch
      && Instant::now() < deadline
    {
      return Some(reading.clone());
    }
    state.cache = None;

    if state
      .worker
      .as_ref()
      .is_some_and(|worker| !worker.is_finished())
      || state
        .last_attempt_ms
        .is_some_and(|last| now.saturating_sub(last) < CPU_INTERVAL_MS)
      || Instant::now() >= deadline
    {
      return None;
    }

    let (sender, receiver) = oneshot::channel();
    let target_topology = context.topology.clone();
    let worker = thread::Builder::new()
      .name("intel-core-dts".into())
      .spawn(move || {
        let result =
          acquire(model, &target_topology, deadline.into_std()).map(|value| Reading {
            acquired_at_ms: uptime_ms(),
            value,
          });
        let _ = sender.send(result);
      })
      .ok()?;
    // Retain the handle on timeout or cancellation. The next request must
    // observe thread exit before scheduling another affinity-changing pass.
    state.worker = Some(worker);
    state.last_attempt_ms = Some(now);

    let reading = timeout_at(deadline, receiver).await.ok()?.ok()??;
    if read_topology().as_ref() != Some(&context.topology)
      || INVALIDATION_EPOCH.load(Ordering::Acquire) != epoch
      || Instant::now() >= deadline
    {
      return None;
    }

    state.cache = Some((context, reading.clone()));
    Some(reading)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::Cell;
  use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
  };

  use crate::infrastructure::providers::windows::cpu_core_temperature::synthetic_topology;

  fn readings() -> Vec<CpuCoreTemperature> {
    vec![
      CpuCoreTemperature {
        processor_group: 0,
        logical_processor: 0,
        temperature_celsius: Some(58.0),
      },
      CpuCoreTemperature {
        processor_group: 0,
        logical_processor: 1,
        temperature_celsius: None,
      },
    ]
  }

  #[tokio::test]
  async fn concurrent_demand_shares_one_acquisition_and_preserves_its_time() {
    let sampler = CoreTemperatures::default();
    let topology = || Some(synthetic_topology(&[0b0101, 0b1010]));
    let starts = Arc::new(AtomicUsize::new(0));

    let request = || {
      let starts = starts.clone();
      sampler.sample(
        0x1a,
        topology,
        move |_, _, _| {
          starts.fetch_add(1, Ordering::Relaxed);
          Some(readings())
        },
        CORE_WAIT_BUDGET,
      )
    };

    let (first, second) = tokio::join!(request(), request());
    let first = first.unwrap();
    let second = second.unwrap();
    assert_eq!(starts.load(Ordering::Relaxed), 1);
    assert_eq!(first.acquired_at_ms, second.acquired_at_ms);
    assert_eq!(first.value, second.value);
    assert_eq!(first.value, readings());
  }

  #[tokio::test]
  async fn changed_topology_discards_fresh_results_and_never_reuses_cached_slots() {
    let sampler = CoreTemperatures::default();
    let before = synthetic_topology(&[0b0101, 0b1010]);
    // Both snapshots have representative processors 0 and 1.
    let after = synthetic_topology(&[0b1001, 0b0110]);
    let observations = Cell::new(0);
    let outcome = sampler
      .sample(
        0x1a,
        || {
          let previous = observations.replace(observations.get() + 1);
          Some(if previous == 0 {
            before.clone()
          } else {
            after.clone()
          })
        },
        |_, _, _| Some(readings()),
        CORE_WAIT_BUDGET,
      )
      .await;
    assert!(outcome.is_none());
    assert!(sampler.state.lock().await.cache.is_none());

    let sampler = CoreTemperatures::default();
    assert!(
      sampler
        .sample(
          0x1a,
          || Some(before.clone()),
          |_, _, _| Some(readings()),
          CORE_WAIT_BUDGET
        )
        .await
        .is_some()
    );
    assert!(
      sampler
        .sample(
          0x1a,
          || Some(after.clone()),
          |_, _, _| {
            panic!("a changed topology does not bypass the CPU acquisition rate")
          },
          CORE_WAIT_BUDGET
        )
        .await
        .is_none()
    );
    assert!(sampler.state.lock().await.cache.is_none());
  }

  #[tokio::test]
  async fn expired_worker_is_discarded_and_blocks_new_passes_until_exit() {
    let sampler = CoreTemperatures::default();
    let topology = || Some(synthetic_topology(&[0b0101, 0b1010]));
    let (release, blocked) = mpsc::channel();
    let first_worker = Arc::new(std::sync::Mutex::new(None));
    let worker_id = first_worker.clone();
    let expired = sampler
      .sample(
        0x1a,
        topology,
        move |_, _, _| {
          *worker_id.lock().unwrap() = Some(thread::current().id());
          blocked.recv_timeout(Duration::from_secs(5)).unwrap();
          Some(readings())
        },
        Duration::from_millis(100),
      )
      .await;
    assert!(expired.is_none());

    // Remove the rate gate to exercise the in-flight gate independently.
    sampler.state.lock().await.last_attempt_ms = None;
    assert!(
      sampler
        .sample(
          0x1a,
          topology,
          |_, _, _| {
            panic!("a second pass cannot start before the expired thread exits")
          },
          CORE_WAIT_BUDGET
        )
        .await
        .is_none()
    );

    release.send(()).unwrap();
    let worker = sampler.state.lock().await.worker.take().unwrap();
    worker.join().unwrap();
    assert!(sampler.state.lock().await.cache.is_none());

    let fresh = sampler
      .sample(
        0x1a,
        topology,
        move |_, _, _| {
          assert_ne!(Some(thread::current().id()), *first_worker.lock().unwrap());
          Some(readings())
        },
        CORE_WAIT_BUDGET,
      )
      .await;
    assert_eq!(fresh.unwrap().value, readings());
  }
}
