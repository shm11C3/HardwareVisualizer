use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant as ThreadDeadline};

use tokio::sync::{Mutex, oneshot};
use tokio::time::{Instant, timeout_at};

use super::{providers::CPU_INTERVAL_MS, uptime_ms};
use crate::infrastructure::providers::windows::{
  pawn_io::cpu::core_temperature::{
    CoreDtsBoard, CoreDtsProfile, CoreTopology, acquire, read_topology, supported_profile,
  },
  wmi_provider::{Win32BaseBoard, start_baseboard_identity_query},
};
use crate::models::CpuCoreTemperature;
use crate::sensor_service::protocol::Reading;

// Keep the core request within the existing two-second pipe exchange.
const CORE_WAIT_BUDGET: Duration = Duration::from_millis(750);
static INVALIDATION_EPOCH: AtomicU64 = AtomicU64::new(0);

type CoreReading = Reading<Vec<CpuCoreTemperature>>;
type BaseboardQuery = Receiver<Result<Vec<Win32BaseBoard>, String>>;

#[derive(Default)]
enum BoardLookup {
  #[default]
  Unstarted,
  Pending {
    query: BaseboardQuery,
    can_retry: bool,
  },
  Resolved(Option<Win32BaseBoard>),
}

impl BoardLookup {
  fn qualifies(
    &mut self,
    required: CoreDtsBoard,
    start_query: impl FnOnce() -> BaseboardQuery,
  ) -> bool {
    let can_retry = match self {
      Self::Unstarted => {
        *self = Self::Pending {
          query: start_query(),
          can_retry: required == CoreDtsBoard::KabyLakeZ270F,
        };
        return false;
      }
      Self::Pending { query, can_retry } => match query.try_recv() {
        Ok(Ok(mut records)) => {
          let board = if records.len() == 1 {
            records.pop()
          } else {
            None
          };
          let qualified = matches_required_board(board.as_ref(), required);
          *self = Self::Resolved(board);
          return qualified;
        }
        Err(TryRecvError::Empty) => return false,
        Ok(Err(_)) | Err(TryRecvError::Disconnected) => *can_retry,
      },
      Self::Resolved(board) => return matches_required_board(board.as_ref(), required),
    };

    *self = if can_retry {
      Self::Pending {
        query: start_query(),
        can_retry: false,
      }
    } else {
      Self::Resolved(None)
    };
    false
  }
}

fn matches_required_board(
  board: Option<&Win32BaseBoard>,
  required: CoreDtsBoard,
) -> bool {
  board.is_some_and(|board| {
    board.matches_manufacturer_product("ASUSTeK COMPUTER INC.", required.product())
  })
}

pub(super) fn invalidate() {
  INVALIDATION_EPOCH.fetch_add(1, Ordering::AcqRel);
}

#[derive(Default)]
pub(super) struct CoreTemperatures {
  // Separate from the ordinary batch so even a timed-out PawnIO call cannot
  // hold the package, power or motherboard providers' locks.
  state: Mutex<State>,
  // A pending WMI worker outlives the request. This decision belongs to the
  // service and is independent of motherboard sensor preferences or demand.
  board: Mutex<BoardLookup>,
}

#[derive(Default)]
struct State {
  cache: Option<(Context, CoreReading)>,
  worker: Option<JoinHandle<()>>,
  last_attempt_ms: Option<u64>,
}

#[derive(PartialEq, Eq)]
struct Context {
  profile: CoreDtsProfile,
  topology: CoreTopology,
  epoch: u64,
}

impl CoreTemperatures {
  pub(super) async fn request(&self) -> Option<CoreReading> {
    self
      .sample(
        supported_profile()?,
        start_baseboard_identity_query,
        read_topology,
        acquire,
        CORE_WAIT_BUDGET,
      )
      .await
  }

  async fn sample(
    &self,
    profile: CoreDtsProfile,
    start_board_query: impl FnOnce() -> BaseboardQuery,
    read_topology: impl Fn() -> Option<CoreTopology>,
    acquire: impl FnOnce(
      CoreDtsProfile,
      &CoreTopology,
      ThreadDeadline,
    ) -> Option<Vec<CpuCoreTemperature>>
    + Send
    + 'static,
    budget: Duration,
  ) -> Option<CoreReading> {
    let deadline = Instant::now() + budget;
    if let Some(board) = profile.required_board()
      && !self.board.lock().await.qualifies(board, start_board_query)
    {
      return None;
    }

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
      profile,
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
          acquire(profile, &target_topology, deadline.into_std()).map(|value| Reading {
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

  use crate::infrastructure::providers::windows::pawn_io::cpu::core_temperature::synthetic_topology;

  const FIXTURE_PROFILE: CoreDtsProfile = CoreDtsProfile::Legacy {
    signature: 0x0001_06a4,
    zero_offset_mask: 0,
  };

  const SKYLAKE_Z170_P_D3: CoreDtsProfile = CoreDtsProfile::Legacy {
    signature: 0x0005_06e3,
    zero_offset_mask: 0,
  };
  const KABY_LAKE_Z270F: CoreDtsProfile = CoreDtsProfile::Legacy {
    signature: 0x0009_06e9,
    zero_offset_mask: 0,
  };

  fn unexpected_board_query() -> BaseboardQuery {
    panic!("unexpected baseboard query")
  }

  fn board(manufacturer: Option<&str>, product: Option<&str>) -> Win32BaseBoard {
    serde_json::from_value(serde_json::json!({
      "Manufacturer": manufacturer,
      "Product": product,
    }))
    .unwrap()
  }

  fn matching_board(product: &str) -> Win32BaseBoard {
    board(Some(" ASUSTeK COMPUTER INC. "), Some(product))
  }

  fn fixture_topology() -> Option<CoreTopology> {
    Some(synthetic_topology(&[0b0101, 0b1010]))
  }

  async fn assert_board_unqualified(
    sampler: &CoreTemperatures,
    profile: CoreDtsProfile,
    start_query: impl FnOnce() -> BaseboardQuery,
  ) {
    assert!(
      sampler
        .sample(
          profile,
          start_query,
          || panic!("topology discovery before board admission"),
          |_, _, _| panic!("MSR acquisition before board admission"),
          CORE_WAIT_BUDGET,
        )
        .await
        .is_none()
    );
  }

  #[tokio::test]
  async fn pending_exact_board_admits_only_after_local_query_and_caches_the_result() {
    for (profile, product) in [
      (SKYLAKE_Z170_P_D3, " Z170-P D3 "),
      (KABY_LAKE_Z270F, " STRIX Z270F GAMING "),
    ] {
      let sampler = CoreTemperatures::default();
      let (sender, receiver) = mpsc::channel();
      assert_board_unqualified(&sampler, profile, || receiver).await;
      assert_board_unqualified(&sampler, profile, unexpected_board_query).await;

      // Other supported processors do not wait for the optional board query.
      assert_eq!(
        sampler
          .sample(
            FIXTURE_PROFILE,
            unexpected_board_query,
            fixture_topology,
            |_, _, _| Some(readings()),
            CORE_WAIT_BUDGET,
          )
          .await
          .unwrap()
          .value,
        readings()
      );
      // The worker sends the sample just before it exits. Finish that
      // synthetic acquisition before switching to the board-gated profile.
      let worker = {
        let mut state = sampler.state.lock().await;
        state.last_attempt_ms = None;
        state.worker.take().unwrap()
      };
      worker.join().unwrap();

      sender.send(Ok(vec![matching_board(product)])).unwrap();
      let sampled = sampler
        .sample(
          profile,
          unexpected_board_query,
          fixture_topology,
          move |admitted, _, _| {
            assert_eq!(admitted, profile);
            Some(readings())
          },
          CORE_WAIT_BUDGET,
        )
        .await
        .unwrap();
      let cached = sampler
        .sample(
          profile,
          unexpected_board_query,
          fixture_topology,
          |_, _, _| panic!("fresh board-qualified sample should be reused"),
          CORE_WAIT_BUDGET,
        )
        .await
        .unwrap();
      assert_eq!(sampled.acquired_at_ms, cached.acquired_at_ms);
      assert_eq!(sampled.value, cached.value);
    }
  }

  #[tokio::test]
  async fn completed_unqualified_board_inventory_is_cached_without_retry() {
    for (profile, correct_product, other_product) in [
      (SKYLAKE_Z170_P_D3, "Z170-P D3", "Z170-P"),
      (KABY_LAKE_Z270F, "STRIX Z270F GAMING", "STRIX Z270E GAMING"),
    ] {
      for records in [
        vec![],
        vec![matching_board(other_product)],
        vec![matching_board(&correct_product.to_ascii_lowercase())],
        vec![board(Some("ASUS"), Some(correct_product))],
        vec![board(None, Some(correct_product))],
        vec![board(Some("ASUSTeK COMPUTER INC."), None)],
        vec![
          matching_board(correct_product),
          matching_board(correct_product),
        ],
        vec![
          matching_board(correct_product),
          matching_board(other_product),
        ],
      ] {
        let sampler = CoreTemperatures::default();
        let (sender, receiver) = mpsc::channel();
        sender.send(Ok(records)).unwrap();
        assert_board_unqualified(&sampler, profile, || receiver).await;
        assert_board_unqualified(&sampler, profile, unexpected_board_query).await;
        assert_board_unqualified(&sampler, profile, unexpected_board_query).await;
      }
    }
  }

  #[tokio::test]
  async fn kaby_lake_retries_one_failed_or_disconnected_board_query() {
    for disconnect in [false, true] {
      let sampler = CoreTemperatures::default();
      let (sender, receiver) = mpsc::channel();
      assert_board_unqualified(&sampler, KABY_LAKE_Z270F, || receiver).await;
      if !disconnect {
        sender.send(Err("WMI unavailable".into())).unwrap();
      }
      drop(sender);
      let (retry_sender, retry_receiver) = mpsc::channel();
      assert_board_unqualified(&sampler, KABY_LAKE_Z270F, || retry_receiver).await;
      assert_board_unqualified(&sampler, KABY_LAKE_Z270F, unexpected_board_query).await;
      retry_sender
        .send(Ok(vec![matching_board("STRIX Z270F GAMING")]))
        .unwrap();
      assert_eq!(
        sampler
          .sample(
            KABY_LAKE_Z270F,
            unexpected_board_query,
            fixture_topology,
            |_, _, _| Some(readings()),
            CORE_WAIT_BUDGET,
          )
          .await
          .unwrap()
          .value,
        readings()
      );
    }
  }

  #[tokio::test]
  async fn exhausted_board_query_failures_stay_unavailable() {
    for profile in [SKYLAKE_Z170_P_D3, KABY_LAKE_Z270F] {
      let sampler = CoreTemperatures::default();
      let (sender, receiver) = mpsc::channel();
      assert_board_unqualified(&sampler, profile, || receiver).await;
      drop(sender);
      if profile == KABY_LAKE_Z270F {
        let (retry_sender, retry_receiver) = mpsc::channel();
        retry_sender.send(Err("WMI unavailable".into())).unwrap();
        assert_board_unqualified(&sampler, profile, || retry_receiver).await;
      }
      assert_board_unqualified(&sampler, profile, unexpected_board_query).await;
      assert_board_unqualified(&sampler, profile, unexpected_board_query).await;
    }
  }

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
        FIXTURE_PROFILE,
        unexpected_board_query,
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
        FIXTURE_PROFILE,
        unexpected_board_query,
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
          FIXTURE_PROFILE,
          unexpected_board_query,
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
          FIXTURE_PROFILE,
          unexpected_board_query,
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
        FIXTURE_PROFILE,
        unexpected_board_query,
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
          FIXTURE_PROFILE,
          unexpected_board_query,
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
        FIXTURE_PROFILE,
        unexpected_board_query,
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
